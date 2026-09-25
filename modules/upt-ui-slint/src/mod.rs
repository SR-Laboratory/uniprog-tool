//! Minimal Slint shell for the compile-time `ui = "slint"` profile.
//!
//! The window shows the existing device/chip state and exposes the first real
//! workflows (scan, connect, detect, load firmware) through the shared
//! `app_ops` layer. Blocking work runs in [`controller`] worker threads and
//! reports back through a channel drained by a Slint timer.

#![cfg(feature = "ui-slint")]

pub mod controller;
pub mod ui_host;

use std::sync::{mpsc, Arc};
use std::time::Duration;

use slint::{ComponentHandle, ModelRc, VecModel};

use crate::boot::AppRuntime;
use crate::l0_core::ui::UiEvent;
pub use controller::{SlintAppEvent, SlintController};
pub use ui_host::SlintUiHost;

slint::slint! {
    import { ListView } from "std-widgets.slint";

    export struct DeviceItem {
        id: string,
        kind: string,
        name: string,
        detail: string,
    }

    export component MainWindow inherits Window {
        title: "UniProgrammer";
        width: 1080px;
        height: 680px;
        background: #0f131a;

        callback scan-devices();
        callback connect-device(string);
        callback detect-chip();
        callback load-firmware();

        in-out property <string> app_title: "UniProgrammer";
        in-out property <string> device_status: "设备：未扫描";
        in-out property <string> chip_status: "芯片：未检测";
        in-out property <string> firmware_status: "固件：未加载";
        in-out property <string> status_text: "就绪";
        in-out property <string> event_text: "尚未收到事件";
        in-out property <string> progress_text: "";
        in-out property <float> progress_value: 0.0;
        in-out property <bool> busy: false;
        in-out property <[DeviceItem]> devices: [];

        VerticalLayout {
            HorizontalLayout {
                vertical-stretch: 1;
                spacing: 1px;

                Rectangle {
                    background: #141a23;
                    min-width: 280px;
                    max-width: 280px;

                    VerticalLayout {
                        padding: 14px;
                        spacing: 8px;

                        Text {
                            text: "设备";
                            color: #e8edf5;
                            font-size: 16px;
                            font-weight: 600;
                        }
                        Rectangle {
                            background: #1b2330;
                            border-radius: 6px;
                            min-height: 52px;
                            VerticalLayout {
                                padding: 8px;
                                Text {
                                    text: root.device_status;
                                    color: #a9b7c8;
                                    font-size: 12px;
                                    wrap: word-wrap;
                                }
                            }
                        }
                        Rectangle {
                            height: 30px;
                            background: #2d6cdf;
                            border-radius: 4px;
                            Text {
                                text: "扫描设备";
                                color: #ffffff;
                                font-size: 13px;
                                horizontal-alignment: center;
                                vertical-alignment: center;
                            }
                            TouchArea {
                                width: parent.width;
                                height: parent.height;
                                clicked => { root.scan-devices(); }
                            }
                        }
                        ListView {
                            height: 170px;
                            for device in root.devices : Rectangle {
                                height: 48px;
                                background: #1b2330;
                                border-radius: 4px;
                                Text {
                                    x: 8px;
                                    y: 5px;
                                    width: parent.width - 16px;
                                    text: device.name;
                                    color: #e8edf5;
                                    font-size: 13px;
                                    wrap: word-wrap;
                                }
                                Text {
                                    x: 8px;
                                    y: 24px;
                                    width: parent.width - 16px;
                                    text: device.detail;
                                    color: #8b98ab;
                                    font-size: 11px;
                                    wrap: word-wrap;
                                }
                                TouchArea {
                                    width: parent.width;
                                    height: parent.height;
                                    clicked => { root.connect-device(device.id); }
                                }
                            }
                        }

                        Text {
                            text: "芯片";
                            color: #e8edf5;
                            font-size: 16px;
                            font-weight: 600;
                        }
                        Rectangle {
                            background: #1b2330;
                            border-radius: 6px;
                            min-height: 110px;
                            VerticalLayout {
                                padding: 8px;
                                Text {
                                    text: root.chip_status;
                                    color: #a9b7c8;
                                    font-size: 12px;
                                    wrap: word-wrap;
                                }
                            }
                        }
                        Rectangle {
                            height: 30px;
                            background: #2d6cdf;
                            border-radius: 4px;
                            Text {
                                text: "检测芯片";
                                color: #ffffff;
                                font-size: 13px;
                                horizontal-alignment: center;
                                vertical-alignment: center;
                            }
                            TouchArea {
                                width: parent.width;
                                height: parent.height;
                                clicked => { root.detect-chip(); }
                            }
                        }
                        Rectangle { vertical-stretch: 1; }
                    }
                }

                Rectangle {
                    horizontal-stretch: 1;
                    background: #0f131a;

                    VerticalLayout {
                        padding: 14px;
                        spacing: 10px;

                        Text {
                            text: root.app_title;
                            color: #e8edf5;
                            font-size: 22px;
                            font-weight: 700;
                        }
                        Text {
                            text: "Slint 工作区";
                            color: #8b98ab;
                            font-size: 14px;
                        }

                        Rectangle {
                            height: 30px;
                            background: #2d6cdf;
                            border-radius: 4px;
                            min-width: 160px;
                            max-width: 200px;
                            Text {
                                text: "加载固件…";
                                color: #ffffff;
                                font-size: 13px;
                                horizontal-alignment: center;
                                vertical-alignment: center;
                            }
                            TouchArea {
                                width: parent.width;
                                height: parent.height;
                                clicked => { root.load-firmware(); }
                            }
                        }
                        Text {
                            text: root.firmware_status;
                            color: #c8d3e2;
                            font-size: 13px;
                            wrap: word-wrap;
                        }

                        Rectangle {
                            background: #151b24;
                            border-radius: 8px;
                            border-width: 1px;
                            border-color: #232c3a;
                            vertical-stretch: 1;

                            Text {
                                x: 20px;
                                y: 0px;
                                width: parent.width - 40px;
                                height: parent.height;
                                text: "后续接入：读写、擦除、校验、Hex 查看与坏块管理";
                                color: #8b98ab;
                                horizontal-alignment: center;
                                vertical-alignment: center;
                                wrap: word-wrap;
                            }
                        }

                        Text {
                            text: root.progress_text;
                            color: #c8d3e2;
                            font-size: 13px;
                            visible: root.progress_text != "";
                        }
                        Rectangle {
                            height: 6px;
                            background: #232c3a;
                            border-radius: 3px;
                            visible: root.progress_text != "";
                            Rectangle {
                                width: parent.width * root.progress_value;
                                background: #4c8dff;
                                border-radius: 3px;
                            }
                        }

                        Text {
                            text: root.event_text;
                            color: #8b98ab;
                            font-size: 12px;
                            wrap: word-wrap;
                        }
                    }
                }
            }

            Rectangle {
                height: 28px;
                background: #141a23;

                HorizontalLayout {
                    padding-left: 12px;
                    padding-right: 12px;
                    Text {
                        text: root.status_text;
                        color: #c8d3e2;
                        font-size: 12px;
                        vertical-alignment: center;
                    }
                }
            }
        }
    }
}

/// Run the Slint event loop until the window closes.
pub fn run(runtime: AppRuntime, ui_host: Arc<SlintUiHost>) -> Result<(), String> {
    let window = MainWindow::new().map_err(|error| format!("创建 Slint 窗口失败: {error}"))?;
    let AppRuntime {
        root_dir,
        state,
        plugin_manager,
        hal_router,
        ..
    } = runtime;

    window.set_status_text(format!("就绪 · 根目录: {}", root_dir.display()).into());
    if let Ok(manager) = plugin_manager.lock() {
        window.set_event_text(format!("已加载 {} 个内置插件", manager.plugins.len()).into());
    }

    let (app_sender, app_receiver) = mpsc::channel();
    let controller = Arc::new(SlintController::new(
        Arc::new(state),
        Arc::new(hal_router),
        root_dir,
        app_sender,
    ));

    {
        let controller = Arc::clone(&controller);
        window.on_scan_devices(move || controller.scan_programmers());
    }
    {
        let controller = Arc::clone(&controller);
        window.on_connect_device(move |id| controller.connect_device(&id));
    }
    {
        let controller = Arc::clone(&controller);
        window.on_detect_chip(move || controller.detect_chip());
    }
    {
        let controller = Arc::clone(&controller);
        window.on_load_firmware(move || controller.load_firmware());
    }

    let receiver = ui_host
        .take_receiver()
        .ok_or_else(|| "SlintUiHost 事件接收端已被取走".to_string())?;
    let weak = window.as_weak();
    let timer = slint::Timer::default();
    timer.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(80),
        move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            while let Ok(event) = receiver.try_recv() {
                apply_ui_event(&window, event);
            }
            while let Ok(event) = app_receiver.try_recv() {
                apply_app_event(&window, event);
            }
        },
    );

    window
        .run()
        .map_err(|error| format!("Slint 事件循环异常: {error}"))
}

fn apply_ui_event(window: &MainWindow, event: UiEvent) {
    match event {
        UiEvent::Status { message } => {
            window.set_event_text(message.into());
        }
        UiEvent::Log { level, message } => {
            window.set_event_text(format!("[{level:?}] {message}").into());
        }
        UiEvent::Progress {
            task,
            done,
            total,
            phase,
            message,
            elapsed_ms,
        } => {
            let mut text = match (total, &message) {
                (0, Some(message)) => format!("{task}: {message}"),
                (0, None) => format!("{task}: {done}"),
                (_, Some(message)) => format!("{task}: {message} ({done}/{total})"),
                (_, None) => format!("{task}: {done}/{total}"),
            };
            if let Some(phase) = phase {
                text.push_str(&format!(" · {phase}"));
            }
            if let Some(elapsed_ms) = elapsed_ms {
                text.push_str(&format!(" · {elapsed_ms} ms"));
            }

            window.set_progress_text(text.into());
            window.set_progress_value(if total > 0 {
                done as f32 / total as f32
            } else {
                0.0
            });
            window.set_busy(true);
        }
        UiEvent::Busy { running } => {
            window.set_busy(running);
        }
        UiEvent::ThemeChanged { .. } | UiEvent::LanguageChanged { .. } => {}
    }
}

fn apply_app_event(window: &MainWindow, event: SlintAppEvent) {
    match event {
        SlintAppEvent::Status(message) => {
            window.set_status_text(message.into());
        }
        SlintAppEvent::Devices(rows) => {
            let count = rows.len();
            let items: Vec<DeviceItem> = rows
                .into_iter()
                .map(|row| DeviceItem {
                    id: row.id.into(),
                    kind: row.kind.into(),
                    name: row.name.into(),
                    detail: row.detail.into(),
                })
                .collect();
            window.set_devices(ModelRc::new(VecModel::from(items)));
            if count == 0 {
                window.set_device_status("设备：未发现支持的编程器".into());
                window.set_event_text("扫描完成，未发现设备".into());
            } else {
                window.set_device_status(format!("设备：发现 {count} 个").into());
                window.set_event_text(format!("扫描完成：{count} 个设备").into());
            }
        }
        SlintAppEvent::Connected(name) => {
            window.set_device_status(format!("设备：{name}").into());
            window.set_event_text(format!("已连接 {name}").into());
        }
        SlintAppEvent::Chip(text) => {
            window.set_chip_status(text.clone().into());
            window.set_event_text(text.into());
        }
        SlintAppEvent::Firmware {
            path,
            length,
            format,
        } => {
            window.set_firmware_status(format!("固件：{path}\n{length} 字节 · {format}").into());
            window.set_event_text(format!("已加载固件 {path}（{length} 字节，{format}）").into());
        }
        SlintAppEvent::Error(message) => {
            window.set_status_text(format!("错误：{message}").into());
            window.set_event_text(message.into());
        }
    }
}
