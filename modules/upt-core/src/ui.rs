//! UI service interface.
//!
//! L0 business logic never talks to Tauri, Slint or any other toolkit
//! directly. A shell injects a [`UiHost`] implementation at startup and core
//! code reports progress, status and log events through it. The same interface
//! also covers blocking interactions such as file dialogs and confirmations,
//! so a new shell only has to implement one trait.
//!
//! The Tauri shell currently emits its own event payloads. It will be ported
//! to this interface incrementally; the Slint shell is written against it from
//! the start.

use std::path::PathBuf;

use serde::Serialize;

/// Severity for user-visible log/status messages.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum UiLogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

/// Visual theme requested by the user.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum UiTheme {
    #[default]
    System,
    Light,
    Dark,
}

/// Transport-neutral event pushed from core to the UI shell.
///
/// `task` is a stable ASCII identifier (`read`, `write`, `verify`, `erase`,
/// `bad_block`, ...) so each shell can map it to its own widget or event name.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum UiEvent {
    /// Plain user-visible message (status bar, toast, log panel).
    Status { message: String },
    /// Log line routed through the same channel as business events.
    Log { level: UiLogLevel, message: String },
    /// Long-running operation progress. `total == 0` means "unknown".
    ///
    /// `phase`, `message` and `elapsed_ms` are optional extras used by erase
    /// and other multi-stage operations; simple read/write/verify callers
    /// leave them as `None`.
    Progress {
        task: String,
        done: u64,
        total: u64,
        phase: Option<String>,
        message: Option<String>,
        elapsed_ms: Option<u64>,
    },
    /// Whether a blocking operation is currently running.
    Busy { running: bool },
    /// The effective theme changed.
    ThemeChanged { theme: UiTheme },
    /// The UI language changed (`zh-CN`, `en-US`, ...).
    LanguageChanged { language: String },
}

impl UiEvent {
    /// Simple progress event without phase/message/elapsed extras.
    pub fn progress(task: impl Into<String>, done: u64, total: u64) -> Self {
        Self::Progress {
            task: task.into(),
            done,
            total,
            phase: None,
            message: None,
            elapsed_ms: None,
        }
    }
}

/// File type filter used by open/save dialogs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileFilter {
    /// Human-readable filter name, e.g. `Firmware`.
    pub name: String,
    /// Extensions without the leading dot, e.g. `["bin", "hex"]`.
    pub extensions: Vec<String>,
}

impl FileFilter {
    pub fn new(
        name: impl Into<String>,
        extensions: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            name: name.into(),
            extensions: extensions.into_iter().map(Into::into).collect(),
        }
    }
}

/// Style hint for a modal confirmation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ConfirmKind {
    #[default]
    Info,
    Warning,
    Danger,
}

/// A blocking yes/no question shown by the shell.
#[derive(Clone, Debug)]
pub struct ConfirmRequest {
    pub title: String,
    pub message: String,
    pub confirm_label: String,
    pub cancel_label: String,
    pub kind: ConfirmKind,
}

impl ConfirmRequest {
    pub fn new(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            message: message.into(),
            confirm_label: "确认".to_string(),
            cancel_label: "取消".to_string(),
            kind: ConfirmKind::Info,
        }
    }

    pub fn danger(mut self) -> Self {
        self.kind = ConfirmKind::Danger;
        self
    }
}

/// The single service a UI shell implements for core code.
///
/// All methods must be callable from any thread. Implementations that own an
/// event loop (Tauri, Slint) should translate calls into a thread-safe message
/// and return immediately; the blocking dialog methods are only called from
/// the shell's own UI thread.
pub trait UiHost: Send + Sync {
    /// Push an event to the shell. Must not block.
    fn emit(&self, event: UiEvent);

    /// Ask the user a yes/no question.
    fn confirm(&self, request: ConfirmRequest) -> bool;

    /// Show an open-file dialog; `Ok(None)` means the user cancelled.
    fn open_file(&self, filters: &[FileFilter]) -> Result<Option<PathBuf>, String>;

    /// Show a save-file dialog; `Ok(None)` means the user cancelled.
    fn save_file(
        &self,
        filters: &[FileFilter],
        default_name: &str,
    ) -> Result<Option<PathBuf>, String>;

    /// Open a URL in the system browser.
    fn open_external(&self, url: &str) -> Result<(), String>;

    /// Current theme preference. Defaults to [`UiTheme::System`].
    fn theme(&self) -> UiTheme {
        UiTheme::System
    }

    /// Current UI language tag. Defaults to Simplified Chinese.
    fn language(&self) -> String {
        "zh-CN".to_string()
    }
}

/// Default host used by headless tests and by boots that do not have a shell
/// yet. Events are dropped and dialogs are cancelled.
#[derive(Debug, Default, Clone, Copy)]
pub struct NullUiHost;

impl UiHost for NullUiHost {
    fn emit(&self, _event: UiEvent) {}

    fn confirm(&self, _request: ConfirmRequest) -> bool {
        false
    }

    fn open_file(&self, _filters: &[FileFilter]) -> Result<Option<PathBuf>, String> {
        Ok(None)
    }

    fn save_file(
        &self,
        _filters: &[FileFilter],
        _default_name: &str,
    ) -> Result<Option<PathBuf>, String> {
        Ok(None)
    }

    fn open_external(&self, _url: &str) -> Result<(), String> {
        Ok(())
    }
}
