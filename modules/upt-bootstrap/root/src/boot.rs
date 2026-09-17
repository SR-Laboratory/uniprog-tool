//! UI-neutral application boot.
//!
//! This module owns everything that must happen before a UI shell starts:
//! settings/log initialisation, L0 plugin loading and boot check, HAL router
//! startup, and construction of the shared application state. Tauri, Slint or
//! any future shell receives the already-booted runtime and only drives the UI.

use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::app_ops::core::AppState;
use crate::l0_core::host::{HostApi, HostContext};
use crate::l0_core::ui::{NullUiHost, UiHost};
use crate::l0_core::{console, runtime, settings, unipkg_protocol, upt_log};
use upt_hal::hal_router::HalRouter;
use upt_plugin::{BootCheck, PluginManager};

/// Fully booted application state shared by the selected UI shell.
pub struct AppRuntime {
    pub root_dir: std::path::PathBuf,
    pub state: Mutex<AppState>,
    pub plugin_manager: Mutex<PluginManager>,
    pub hal_router: Mutex<HalRouter>,
    /// UI service injected by the selected shell. Core code only talks to the
    /// [`UiHost`] trait; headless boots use [`NullUiHost`].
    pub ui: Arc<dyn UiHost>,
}

impl AppRuntime {
    /// Build the unipkg asset snapshot consumed by the WebView shells.
    pub fn unipkg_assets(&self) -> unipkg_protocol::UnipkgProtocol {
        let manager = self
            .plugin_manager
            .lock()
            .expect("plugin manager mutex poisoned");
        unipkg_protocol::UnipkgProtocol::from_manager(&manager)
    }
}

/// Write a readable Chinese boot-error report next to the executable.
fn write_boot_error(base: &Path, boot: &BootCheck) {
    let mut message = String::from("UniProgrammer 启动失败：必需插件缺失或无效\n\n");

    if !boot.missing.is_empty() {
        message.push_str("缺少的必需插件:\n");
        for name in &boot.missing {
            message.push_str(&format!("  - {name}\n"));
        }
        message.push('\n');
    }

    if !boot.invalid.is_empty() {
        message.push_str("无效的必需插件:\n");
        for name in &boot.invalid {
            message.push_str(&format!("  - {name}\n"));
        }
        message.push('\n');
    }

    message.push_str("请恢复 plugins/builtin 目录下的内置插件清单后重试。\n");

    let error_path = base.join("uniprog-boot-error.txt");
    if let Err(e) = std::fs::write(&error_path, message.as_bytes()) {
        eprintln!("写入启动错误文件失败 {}: {e}", error_path.display());
    }
}

fn default_state() -> AppState {
    AppState {
        ch34x: None,
        serprog: None,
        lib: None,
        connected_device: None,
        detected: None,
        sidecar_adapter: None,
        sidecar_device: None,
        last_serial_ports: Vec::new(),
        cached_serprog: Vec::new(),
        operation_running: false,
    }
}

/// Boot the L0/HAL layers without creating any UI.
///
/// Returns an error string after writing `uniprog-boot-error.txt` when the
/// required L1 plugin set is missing or invalid.
pub fn boot() -> Result<AppRuntime, String> {
    boot_with_ui(Arc::new(NullUiHost))
}

/// Boot with a shell-provided UI service.
///
/// The shell constructs its [`UiHost`] first, then hands it to this function so
/// core code can report progress before the window is created.
pub fn boot_with_ui(ui: Arc<dyn UiHost>) -> Result<AppRuntime, String> {
    let debug_console = settings::startup_debug_console();
    let log_level = if debug_console {
        upt_log::Level::Debug
    } else {
        upt_log::Level::Info
    };

    // Text logging first, then the optional console, so sidecar stderr and
    // backend logs all end up in the same place.
    upt_log::init(
        log_level,
        debug_console || cfg!(debug_assertions),
        Some(settings::log_file()),
    )?;

    if debug_console {
        console::attach();
        std::env::set_var("UNIPROG_DEBUG_CONSOLE", "1");
        runtime::log_info("调试控制台已启用");
    }

    let host = HostContext::new(runtime::exe_dir());
    let root_dir = host.root_dir();
    host.log(
        upt_log::Level::Info,
        &format!("UniProgrammer 启动，根目录: {}", root_dir.display()),
    );

    let mut plugin_manager = PluginManager::load(&root_dir);
    let boot_check = plugin_manager.boot_check();
    if !boot_check.missing.is_empty() || !boot_check.invalid.is_empty() {
        host.log(upt_log::Level::Info, "启动失败：L1 必需插件缺失或无效");
        write_boot_error(&root_dir, &boot_check);
        return Err("L1 required plugin set is missing or invalid".to_string());
    }
    host.log(
        upt_log::Level::Info,
        &format!(
            "插件扫描完成：{} 个插件，{} 个错误",
            plugin_manager.plugins.len(),
            plugin_manager.errors.len()
        ),
    );

    let hal_router = HalRouter::start(&mut plugin_manager, &root_dir);

    Ok(AppRuntime {
        root_dir,
        state: Mutex::new(default_state()),
        plugin_manager: Mutex::new(plugin_manager),
        hal_router: Mutex::new(hal_router),
        ui,
    })
}
