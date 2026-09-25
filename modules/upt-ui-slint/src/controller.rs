//! Slint-side application controller.
//!
//! The Tauri shell drives `app_ops` from `#[tauri::command]` functions. Slint
//! has no command layer, so this controller calls the same `app_ops` functions
//! from short-lived worker threads and reports results through a channel that
//! the UI event loop drains in `mod.rs`.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{mpsc, Arc, Mutex};

use crate::app_ops::autodetect;
use crate::app_ops::core::{self, AppState};
use crate::l0_core::runtime::log_info;
use crate::ui_dialogs;
use upt_chipdb as chiplib;
use upt_devices::ch34x::{Ch34xDevice, Ch34xSettings, ChipKind};
use upt_devices::serprog;
use upt_hal::hal_router::HalRouter;
use upt_proto::firmware;

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
        }
    }

    fn send(&self, event: SlintAppEvent) {
        let _ = self.sender.send(event);
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
                let _router = this.router.lock().map_err(|error| error.to_string())?;
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
