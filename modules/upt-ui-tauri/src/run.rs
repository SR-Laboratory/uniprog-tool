//! Tauri shell entry point.
//!
//! The boot layer already created all shared state; this module only starts the
//! Tauri application, attaches the WebView handler and wires the close guard.

use std::sync::{Arc, Mutex};

use tauri::{Emitter, Manager, WindowEvent};

use super::commands;
use super::ui_host::TauriUiHost;
use crate::app_ops::core;
use crate::boot::AppRuntime;
use crate::l0_core::ui::UiHost;
use crate::l0_core::unipkg_protocol;

/// Start the Tauri shell and block until the application exits.
pub fn run(runtime: AppRuntime, ui_host: Arc<TauriUiHost>) {
    let plugin_assets = runtime.unipkg_assets();
    let managed_ui: Arc<dyn UiHost> = runtime.ui.clone();
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
        .manage(managed_ui)
        .setup(move |app| {
            ui_host.attach(app.handle().clone());
            Ok(())
        })
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
    let builder = commands::attach_handler(builder);

    unipkg_protocol::UnipkgProtocol::register(builder, plugin_assets)
        .run(tauri::generate_context!())
        .expect("启动失败");
}
