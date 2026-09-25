//! Slint-side application controller.
//!
//! The Tauri shell drives `app_ops` from `#[tauri::command]` functions. Slint
//! has no command layer, so this controller calls the same `app_ops` functions
//! from short-lived worker threads and reports results through a channel that
//! the UI event loop drains in `mod.rs`.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{mpsc, Arc, Mutex};

use crate::app_ops::autodetect;
use crate::app_ops::core::{self, AppState};
use crate::app_ops::operations;
use crate::l0_core::runtime::log_info;
use crate::ui_dialogs;
use upt_chipdb as chiplib;
use upt_devices::ch34x::{Ch34xDevice, Ch34xSettings, ChipKind};
use upt_devices::serprog;
use upt_hal::hal_router::HalRouter;
use upt_proto::firmware;

const HEX_PREVIEW_BYTES: usize = 4096;
const FALLBACK_READ_SIZE: u64 = 256 * 1024;

/// One row in the Slint device list.
#[derive(Clone)]
pub struct DeviceRow {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub detail: String,
}

/// Result events produced by controller worker threads.
pub enum SlintAppEvent {
    Status(String),
    Devices(Vec<DeviceRow>),
    Connected(String),
    Chip(String),
    Firmware {
        path: String,
        length: usize,
        format: String,
    },
    Progress {
        task: String,
        done: u64,
        total: u64,
        phase: Option<String>,
        message: Option<String>,
        elapsed_ms: Option<u64>,
    },
    Busy(bool),
    ReadData {
        length: usize,
        preview: String,
    },
    Error(String),
}

/// Shared controller used by Slint callbacks.
pub struct SlintController {
    state: Arc<Mutex<AppState>>,
    router: Arc<Mutex<HalRouter>>,
    root_dir: PathBuf,
    sender: mpsc::Sender<SlintAppEvent>,
    candidates: Mutex<HashMap<String, autodetect::ProgrammerCandidate>>,
    firmware: Mutex<Option<Vec<u8>>>,
    read_data: Mutex<Option<Vec<u8>>>,
}

impl SlintController {
    pub fn new(
        state: Arc<Mutex<AppState>>,
        router: Arc<Mutex<HalRouter>>,
        root_dir: PathBuf,
        sender: mpsc::Sender<SlintAppEvent>,
    ) -> Self {
        Self {
            state,
            router,
            root_dir,
            sender,
            candidates: Mutex::new(HashMap::new()),
            firmware: Mutex::new(None),
            read_data: Mutex::new(None),
        }
    }

    fn send(&self, event: SlintAppEvent) {
        let _ = self.sender.send(event);
    }

    fn send_progress(&self, task: &str, done: u64, total: u64) {
        self.send(SlintAppEvent::Progress {
            task: task.to_string(),
            done,
            total,
            phase: None,
            message: None,
            elapsed_ms: None,
        });
    }

    fn send_bad_block_progress(&self, done: u32, total: u32) {
        self.send_progress("bad_block", done as u64, total as u64);
    }

    fn send_erase_progress(&self, progress: &operations::EraseProgress) {
        self.send(SlintAppEvent::Progress {
            task: "erase".to_string(),
            done: progress.done,
            total: progress.total,
            phase: Some(progress.phase.clone()),
            message: Some(progress.message.clone()),
            elapsed_ms: progress.elapsed_ms,
        });
    }

    /// Scan USB programmers and serial ports on a worker thread.
    pub fn scan_programmers(self: &Arc<Self>) {
        let this = Arc::clone(self);
        std::thread::spawn(move || {
            this.send(SlintAppEvent::Status("正在扫描编程器…".to_string()));

            let mut candidates = autodetect::scan_ch34x();
            let ports: Vec<String> = serialport::available_ports()
                .map(|list| list.into_iter().map(|port| port.port_name).collect())
                .unwrap_or_default();
            candidates.extend(autodetect::scan_serprog(&ports, true));

            if let Ok(mut cache) = this.candidates.lock() {
                cache.clear();
                for candidate in &candidates {
                    cache.insert(candidate.id.clone(), candidate.clone());
                }
            }

            let rows: Vec<DeviceRow> = candidates
                .iter()
                .map(|candidate| DeviceRow {
                    id: candidate.id.clone(),
                    kind: candidate.kind.clone(),
                    name: candidate.name.clone(),
                    detail: candidate.detail.clone(),
                })
                .collect();
            this.send(SlintAppEvent::Status(format!(
                "扫描完成：{} 个设备",
                rows.len()
            )));
            this.send(SlintAppEvent::Devices(rows));
        });
    }

    /// Connect the programmer selected in the Slint list.
    pub fn connect_device(self: &Arc<Self>, id: &str) {
        let candidate = self
            .candidates
            .lock()
            .ok()
            .and_then(|cache| cache.get(id).cloned());
        let Some(candidate) = candidate else {
            self.send(SlintAppEvent::Error(format!("找不到设备: {id}")));
            return;
        };

        let this = Arc::clone(self);
        std::thread::spawn(move || {
            this.send(SlintAppEvent::Status(format!(
                "正在连接 {}…",
                candidate.name
            )));
            let result = this.connect_candidate(&candidate);
            match result {
                Ok(name) => {
                    this.send(SlintAppEvent::Status(format!("已连接 {name}")));
                    this.send(SlintAppEvent::Connected(name));
                }
                Err(error) => this.send(SlintAppEvent::Error(error)),
            }
        });
    }

    fn connect_candidate(
        &self,
        candidate: &autodetect::ProgrammerCandidate,
    ) -> Result<String, String> {
        let mut state = self.state.lock().map_err(|error| error.to_string())?;

        match candidate.kind.as_str() {
            "serprog" => {
                let port = candidate
                    .port
                    .clone()
                    .ok_or_else(|| "serprog 缺少串口信息".to_string())?;
                let device = serprog::Serprog::open(&port)?;
                let name = format!("serprog ({port})");
                state.ch34x = None;
                state.serprog = Some(device);
                state.detected = None;
                state.connected_device = Some(name.clone());
                log_info(&format!("slint: 已连接 {name}"));
                Ok(name)
            }
            kind @ ("ch341" | "ch347" | "ch347f") => {
                let chip_kind = match kind {
                    "ch341" => ChipKind::Ch341A,
                    "ch347" => ChipKind::Ch347T,
                    _ => ChipKind::Ch347F,
                };
                let settings = Ch34xSettings {
                    kind: chip_kind,
                    spi_mode: 0,
                    freq_khz: 1000,
                    // VCC 供电与 SPI/IO 信号电平绑定到同一目标轨。
                    io_level_mv: 3300,
                    device_index: candidate.device_index.unwrap_or(0),
                    usb_bus: candidate.usb_bus,
                    usb_address: candidate.usb_address,
                };

                // Per-operation lifecycle: open once and run a JEDEC probe so a
                // connected-but-dead programmer is reported immediately.
                {
                    let device = Ch34xDevice::open(&settings)?;
                    let _ = core::spi_read_jedec(&device)?;
                }

                let name = candidate.name.clone();
                state.ch34x = Some(settings);
                state.serprog = None;
                state.detected = None;
                state.connected_device = Some(name.clone());
                log_info(&format!("slint: 已连接 {name}"));
                Ok(name)
            }
            other => Err(format!("未知编程器类型: {other}")),
        }
    }

    /// Detect the chip connected to the selected programmer.
    pub fn detect_chip(self: &Arc<Self>) {
        let this = Arc::clone(self);
        std::thread::spawn(move || {
            this.send(SlintAppEvent::Status("正在识别芯片…".to_string()));
            let result = (|| -> Result<String, String> {
                this.ensure_chip_lib()?;
                let mut state = this.state.lock().map_err(|error| error.to_string())?;
                let result = core::detect_chip(&mut state)?;
                Ok(result.text)
            })();

            match result {
                Ok(text) => {
                    log_info("slint: 芯片识别完成");
                    this.send(SlintAppEvent::Status("芯片识别完成".to_string()));
                    this.send(SlintAppEvent::Chip(text));
                }
                Err(error) => this.send(SlintAppEvent::Error(error)),
            }
        });
    }

    /// Pick and decode a firmware file.
    pub fn load_firmware(self: &Arc<Self>) {
        let path = match ui_dialogs::open_file() {
            Ok(Some(path)) => path,
            Ok(None) => return,
            Err(error) => {
                self.send(SlintAppEvent::Error(error));
                return;
            }
        };

        let this = Arc::clone(self);
        std::thread::spawn(move || {
            this.send(SlintAppEvent::Status(format!("正在加载固件 {path}…")));
            match firmware::load_firmware_file(&path) {
                Ok((bytes, format)) => {
                    let length = bytes.len();
                    if let Ok(mut slot) = this.firmware.lock() {
                        *slot = Some(bytes);
                    }
                    this.send(SlintAppEvent::Firmware {
                        path,
                        length,
                        format: format.to_string(),
                    });
                }
                Err(error) => this.send(SlintAppEvent::Error(error)),
            }
        });
    }

    /// Read the connected chip into the controller buffer and show a hex preview.
    pub fn read_chip(self: &Arc<Self>) {
        let this = Arc::clone(self);
        std::thread::spawn(move || {
            this.send(SlintAppEvent::Busy(true));
            this.send(SlintAppEvent::Status("正在读取芯片…".to_string()));

            let result = (|| -> Result<(usize, String), String> {
                this.ensure_chip_lib()?;
                let mut state = this.state.lock().map_err(|error| error.to_string())?;
                let mut router = this.router.lock().map_err(|error| error.to_string())?;
                let (size, start_addr) = match state.detected.as_ref() {
                    Some(info) => (info.size, 0),
                    None => (FALLBACK_READ_SIZE, 0),
                };

                let mut progress = |done: u64, total: u64| {
                    this.send_progress("read", done, total);
                };
                let mut bad_block = |done: u32, total: u32| {
                    this.send_bad_block_progress(done, total);
                };
                let data = operations::read_chip(
                    &mut state,
                    size,
                    start_addr,
                    None,
                    &mut progress,
                    &mut bad_block,
                    Some(&mut router),
                )?;

                let preview = hex_preview(&data, start_addr, HEX_PREVIEW_BYTES);
                let length = data.len();
                if let Ok(mut slot) = this.read_data.lock() {
                    *slot = Some(data);
                }
                Ok((length, preview))
            })();

            this.send(SlintAppEvent::Busy(false));
            match result {
                Ok((length, preview)) => {
                    log_info(&format!("slint: 读取完成 {length} 字节"));
                    this.send(SlintAppEvent::Status(format!("读取完成：{length} 字节")));
                    this.send(SlintAppEvent::ReadData { length, preview });
                }
                Err(error) => this.send(SlintAppEvent::Error(error)),
            }
        });
    }

    /// Write the loaded firmware to the connected chip.
    pub fn write_chip(self: &Arc<Self>) {
        let this = Arc::clone(self);
        std::thread::spawn(move || {
            this.send(SlintAppEvent::Busy(true));
            let result = (|| -> Result<String, String> {
                let data = this.take_firmware()?;
                this.ensure_connected()?;
                let mut state = this.state.lock().map_err(|error| error.to_string())?;
                let mut router = this.router.lock().map_err(|error| error.to_string())?;
                let mut progress = |done: u64, total: u64| {
                    this.send_progress("write", done, total);
                };
                let mut bad_block = |done: u32, total: u32| {
                    this.send_bad_block_progress(done, total);
                };
                operations::write_chip(
                    &mut state,
                    &data,
                    0,
                    None,
                    None,
                    &mut progress,
                    &mut bad_block,
                    Some(&mut router),
                )
            })();

            this.finish_operation("写入", result);
        });
    }

    /// Verify the connected chip against the loaded firmware.
    pub fn verify_chip(self: &Arc<Self>) {
        let this = Arc::clone(self);
        std::thread::spawn(move || {
            this.send(SlintAppEvent::Busy(true));
            let result = (|| -> Result<String, String> {
                let data = this.take_firmware()?;
                this.ensure_connected()?;
                let mut state = this.state.lock().map_err(|error| error.to_string())?;
                let mut router = this.router.lock().map_err(|error| error.to_string())?;
                let mut progress = |done: u64, total: u64| {
                    this.send_progress("verify", done, total);
                };
                let mut bad_block = |done: u32, total: u32| {
                    this.send_bad_block_progress(done, total);
                };
                operations::verify_chip(
                    &mut state,
                    &data,
                    0,
                    None,
                    &mut progress,
                    &mut bad_block,
                    Some(&mut router),
                )
            })();

            this.finish_operation("校验", result);
        });
    }

    /// Erase the connected chip.
    pub fn erase_chip(self: &Arc<Self>) {
        let this = Arc::clone(self);
        std::thread::spawn(move || {
            this.send(SlintAppEvent::Busy(true));
            let result = (|| -> Result<String, String> {
                this.ensure_connected()?;
                let mut state = this.state.lock().map_err(|error| error.to_string())?;
                let mut router = this.router.lock().map_err(|error| error.to_string())?;
                let mut progress = |progress: operations::EraseProgress| {
                    this.send_erase_progress(&progress);
                };
                let mut bad_block = |done: u32, total: u32| {
                    this.send_bad_block_progress(done, total);
                };
                operations::chip_erase(
                    &mut state,
                    None,
                    &mut progress,
                    &mut bad_block,
                    Some(&mut router),
                )
            })();

            this.finish_operation("擦除", result);
        });
    }

    /// Save the last read buffer to a user-selected file.
    pub fn save_read_data(self: &Arc<Self>) {
        let data = self.read_data.lock().ok().and_then(|slot| slot.clone());
        let Some(data) = data else {
            self.send(SlintAppEvent::Error("还没有读取数据可保存".to_string()));
            return;
        };

        let path = match ui_dialogs::save_file("uniprog_read.bin", "bin") {
            Ok(Some(path)) => path,
            Ok(None) => return,
            Err(error) => {
                self.send(SlintAppEvent::Error(error));
                return;
            }
        };

        let this = Arc::clone(self);
        std::thread::spawn(move || match fs::write(&path, &data) {
            Ok(()) => {
                this.send(SlintAppEvent::Status(format!(
                    "已保存 {} 字节到 {path}",
                    data.len()
                )));
            }
            Err(error) => this.send(SlintAppEvent::Error(format!("保存失败 {path}: {error}"))),
        });
    }

    fn take_firmware(&self) -> Result<Vec<u8>, String> {
        self.firmware
            .lock()
            .map_err(|error| error.to_string())?
            .clone()
            .ok_or_else(|| "请先加载固件".to_string())
    }

    fn ensure_connected(&self) -> Result<(), String> {
        let state = self.state.lock().map_err(|error| error.to_string())?;
        if state.ch34x.is_none() && state.serprog.is_none() && state.sidecar_adapter.is_none() {
            return Err("请先连接编程器".to_string());
        }
        Ok(())
    }

    fn finish_operation(self: &Arc<Self>, label: &str, result: Result<String, String>) {
        self.send(SlintAppEvent::Busy(false));
        match result {
            Ok(message) => {
                log_info(&format!("slint: {label}完成: {message}"));
                self.send(SlintAppEvent::Status(format!("{label}完成：{message}")));
            }
            Err(error) => self.send(SlintAppEvent::Error(format!("{label}失败：{error}"))),
        }
    }

    fn ensure_chip_lib(&self) -> Result<(), String> {
        let mut state = self.state.lock().map_err(|error| error.to_string())?;
        if state.lib.is_some() {
            return Ok(());
        }

        let xml = self.root_dir.join("chiplib.xml");
        let bin = self.root_dir.join("chiplib.bin");
        let lib = chiplib::Chiplib::load_auto(&xml.to_string_lossy(), &bin.to_string_lossy())?;
        state.lib = Some(lib);
        Ok(())
    }
}

/// Format the first `max_bytes` of `data` as a classic address/hex/ascii dump.
fn hex_preview(data: &[u8], base_addr: u64, max_bytes: usize) -> String {
    let count = data.len().min(max_bytes);
    let mut output = String::new();

    for (row, chunk) in data[..count].chunks(16).enumerate() {
        output.push_str(&format!("{:08X}  ", base_addr + (row * 16) as u64));
        for index in 0..16 {
            match chunk.get(index) {
                Some(byte) => output.push_str(&format!("{byte:02X} ")),
                None => output.push_str("   "),
            }
        }
        output.push_str(" |");
        for byte in chunk {
            output.push(if byte.is_ascii_graphic() || *byte == b' ' {
                *byte as char
            } else {
                '.'
            });
        }
        output.push_str("|\n");
    }

    if data.len() > count {
        output.push_str(&format!("... 仅显示前 {count} 字节\n"));
    }
    output
}
