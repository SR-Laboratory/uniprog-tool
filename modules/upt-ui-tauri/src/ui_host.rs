//! Tauri implementation of the L0 [`UiHost`] service.
//!
//! The host is created before `boot_with_ui` runs but only receives its
//! [`AppHandle`] from the Tauri `setup` hook. Events emitted during boot (when
//! no handle exists yet) are dropped, matching the headless `NullUiHost`
//! behaviour; business commands run after `setup`, so their progress reaches
//! the WebView normally.

use std::path::PathBuf;
use std::sync::OnceLock;

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use super::dialogs;
use crate::l0_core::ui::{ConfirmRequest, FileFilter, UiEvent, UiHost, UiLogLevel, UiTheme};

#[derive(Clone, Serialize)]
struct StatusEvent {
    message: String,
}

#[derive(Clone, Serialize)]
struct LogEvent {
    level: UiLogLevel,
    message: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProgressEvent {
    done: u64,
    total: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    phase: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
    #[serde(rename = "elapsedMs", skip_serializing_if = "Option::is_none")]
    elapsed_ms: Option<u64>,
}

#[derive(Clone, Serialize)]
struct BusyEvent {
    running: bool,
}

#[derive(Clone, Serialize)]
struct ThemeEvent {
    theme: UiTheme,
}

#[derive(Clone, Serialize)]
struct LanguageEvent {
    language: String,
}

/// Tauri-backed UI host. The handle is installed by the Tauri setup hook.
#[derive(Default)]
pub struct TauriUiHost {
    app: OnceLock<AppHandle>,
}

impl TauriUiHost {
    pub fn new() -> Self {
        Self::default()
    }

    /// Bind the host to the running Tauri application.
    pub fn attach(&self, app: AppHandle) {
        if self.app.set(app).is_err() {
            eprintln!("TauriUiHost: AppHandle 已经挂载，忽略重复调用");
        }
    }

    fn app(&self) -> Option<&AppHandle> {
        self.app.get()
    }
}

impl UiHost for TauriUiHost {
    fn emit(&self, event: UiEvent) {
        let Some(app) = self.app() else {
            return;
        };

        match event {
            UiEvent::Status { message } => {
                let _ = app.emit("ui_status", StatusEvent { message });
            }
            UiEvent::Log { level, message } => {
                let _ = app.emit("ui_log", LogEvent { level, message });
            }
            UiEvent::Progress {
                task,
                done,
                total,
                phase,
                message,
                elapsed_ms,
            } => {
                let _ = app.emit(
                    &format!("{task}_progress"),
                    ProgressEvent {
                        done,
                        total,
                        phase,
                        message,
                        elapsed_ms,
                    },
                );
            }
            UiEvent::Busy { running } => {
                let _ = app.emit("ui_busy", BusyEvent { running });
            }
            UiEvent::ThemeChanged { theme } => {
                let _ = app.emit("ui_theme_changed", ThemeEvent { theme });
            }
            UiEvent::LanguageChanged { language } => {
                let _ = app.emit("ui_language_changed", LanguageEvent { language });
            }
        }
    }

    fn confirm(&self, _request: ConfirmRequest) -> bool {
        // Native Tauri confirmation dialogs arrive with the dialog layer port.
        false
    }

    fn open_file(&self, _filters: &[FileFilter]) -> Result<Option<PathBuf>, String> {
        dialogs::open_file().map(|path| path.map(PathBuf::from))
    }

    fn save_file(
        &self,
        _filters: &[FileFilter],
        default_name: &str,
    ) -> Result<Option<PathBuf>, String> {
        dialogs::save_file(default_name, "bin").map(|path| path.map(PathBuf::from))
    }

    fn open_external(&self, url: &str) -> Result<(), String> {
        open::that(url).map_err(|error| format!("打开链接失败 {url}: {error}"))
    }

    fn theme(&self) -> UiTheme {
        UiTheme::System
    }
}
