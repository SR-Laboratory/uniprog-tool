//! Minimal Slint shell for the compile-time `ui = "slint"` profile.
//!
//! The window is intentionally shallow: left column shows device/chip state,
//! the center is the future workspace, and the bottom bar shows status. The
//! real work this round is the [`ui_host`] bridge that turns the L0 `UiHost`
//! event stream into thread-safe Slint property updates.

#![cfg(feature = "ui-slint")]

pub mod ui_host;

use std::sync::Arc;
use std::time::Duration;

use crate::boot::AppRuntime;
use crate::l0_core::ui::UiEvent;

pub use ui_host::SlintUiHost;

slint::slint! {
    export component MainWindow inherits Window {
        title: "UniProgrammer";
        width: 1080px;
        height: 680px;
        background: #0f131a;

        in-out property <string> app_title: "UniProgrammer";
        in-out property <string> device_status: "设备：等待连接";
        in-out property <string> chip_status: "芯片：未检测";
        in-out property <string> status_text: "就绪";
        in-out property <string> event_text: "尚未收到事件";
        in-out property <string> progress_text: "";
        in-out property <float> progress_value: 0.0;
        in-out property <bool> busy: false;

        VerticalLayout {
            HorizontalLayout {
                vertical-stretch: 1;
                spacing: 1px;

                Rectangle {
                    background: #141a23;
                    min-width: 260px;
                    max-width: 260px;

                    VerticalLayout {
                        padding: 14px;
                        spacing: 10px;

                        Text {
                            text: "设备";
                            color: #e8edf5;
                            font-size: 16px;
                            font-weight: 600;
                        }
                        Rectangle {
                            background: #1b2330;
                            border-radius: 6px;
                            min-height: 120px;
                            VerticalLayout {
                                padding: 10px;
                                Text {
                                    text: root.device_status;
                                    color: #a9b7c8;
                                    wrap: word-wrap;
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
                            min-height: 120px;
                            VerticalLayout {
                                padding: 10px;
                                Text {
                                    text: root.chip_status;
                                    color: #a9b7c8;
                                    wrap: word-wrap;
                                }
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
                            text: "Slint 工作区（骨架）";
                            color: #8b98ab;
                            font-size: 14px;
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
                                text: "后续接入：设备连接、芯片检测、读写/擦除/校验、固件加载与 Hex 查看";
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

    window.set_status_text(format!("就绪 · 根目录: {}", runtime.root_dir.display()).into());
    {
        let manager = runtime
            .plugin_manager
            .lock()
            .map_err(|error| format!("读取插件状态失败: {error}"))?;
        window.set_event_text(format!("已加载 {} 个内置插件", manager.plugins.len()).into());
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
                apply_event(&window, event);
            }
        },
    );

    window
        .run()
        .map_err(|error| format!("Slint 事件循环异常: {error}"))
}

fn apply_event(window: &MainWindow, event: UiEvent) {
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
