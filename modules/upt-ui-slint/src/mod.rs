//! Minimal Slint shell skeleton.
//!
//! This module proves the compile-time shell switch before feature parity
//! work starts: it receives the same [`crate::boot::AppRuntime`] as the Tauri
//! shell and renders an empty window with a status line. Business panels,
//! dialogs and the hex viewer are added incrementally on top of the L0
//! `UiHost` service.

#![cfg(feature = "ui-slint")]

use crate::boot::AppRuntime;

slint::slint! {
    export component MainWindow inherits Window {
        title: "UniProgrammer";
        width: 960px;
        height: 640px;
        background: #10141b;

        in-out property <string> status: "就绪";

        VerticalLayout {
            padding: 16px;
            spacing: 10px;

            Text {
                text: "UniProgrammer";
                color: #e8edf5;
                font-size: 24px;
                horizontal-alignment: center;
            }

            Text {
                text: "Slint shell skeleton";
                color: #8b98ab;
                horizontal-alignment: center;
            }

            Rectangle {
                background: #161c26;
                border-radius: 6px;
                min-height: 220px;

                Text {
                    text: "工作区占位：后续接入设备、芯片与固件面板";
                    color: #8b98ab;
                    horizontal-alignment: center;
                    vertical-alignment: center;
                    wrap: word-wrap;
                }
            }

            Text {
                text: root.status;
                color: #c8d3e2;
                font-size: 13px;
                wrap: word-wrap;
            }
        }
    }
}

/// Run the Slint event loop until the window closes.
pub fn run(runtime: AppRuntime) -> Result<(), String> {
    let window = MainWindow::new().map_err(|error| format!("创建 Slint 窗口失败: {error}"))?;
    window.set_status(format!("根目录: {}", runtime.root_dir.display()).into());
    window
        .run()
        .map_err(|error| format!("Slint 事件循环异常: {error}"))
}
