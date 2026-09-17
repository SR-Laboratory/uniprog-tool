#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod app_ops;
pub mod boot;
pub mod l0_core;
pub mod ui_tauri;

use app_ops::core;
use boot::AppRuntime;
use l0_core::unipkg_protocol;
use std::sync::Mutex;
use tauri::{Emitter, Manager, WindowEvent};

fn main() {
    let runtime = match boot::boot() {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("启动失败: {error}");
            std::process::exit(1);
        }
    };

    let plugin_assets = runtime.unipkg_assets();
    let AppRuntime {
        state,
        plugin_manager,
        hal_router,
        ..
    } = runtime;

    let builder = tauri::Builder::default()
        .manage(state)
        .manage(plugin_manager)
        .manage(hal_router)
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let busy = window
                    .state::<Mutex<core::AppState>>()
                    .lock()
                    .map(|s| s.operation_running)
                    .unwrap_or(false);
                if busy {
                    // Rust 侧同步拦截，保证任务栏右键“关闭窗口”也走确认流程。
                    api.prevent_close();
                    let _ = window.emit("close_requested_while_busy", ());
                }
            }
        });
    let builder = ui_tauri::commands::attach_handler(builder);

    unipkg_protocol::UnipkgProtocol::register(builder, plugin_assets)
        .run(tauri::generate_context!())
        .expect("启动失败");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nor_4byte_boundary() {
        assert!(!core::nor_requires_4byte(0x0100_0000)); // exactly 16 MiB: 3-byte mode
        assert!(core::nor_requires_4byte(0x0100_0001)); // above 16 MiB: 4-byte mode
        assert!(core::nor_requires_4byte(0x0200_0000));
    }

    #[test]
    fn jedec_candidates_cover_shifted_nand_id() {
        let raw = [0xFF, 0x01, 0x25, 0xFF, 0xFF];
        let ids = core::jedec_id_candidates(&raw);
        assert!(ids.contains(&"0125".to_string()));
        assert!(ids.contains(&"FF0125".to_string()));
        assert!(ids.contains(&"0125FF".to_string()));
    }
}
