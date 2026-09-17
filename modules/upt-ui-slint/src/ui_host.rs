//! Channel-based [`UiHost`] implementation for the Slint shell.
//!
//! Core code can run on worker threads while Slint properties may only be
//! changed from the event loop. `SlintUiHost` therefore only pushes events into
//! a standard channel; [`crate::ui_slint::run`] drains it from a Slint timer on
//! the UI thread and applies the resulting property updates there.

use std::path::PathBuf;
use std::sync::{mpsc, Arc, Mutex};

use crate::l0_core::ui::{ConfirmRequest, FileFilter, UiEvent, UiHost, UiTheme};

/// UI service passed to `boot_with_ui` by the Slint shell.
pub struct SlintUiHost {
    sender: mpsc::Sender<UiEvent>,
    receiver: Mutex<Option<mpsc::Receiver<UiEvent>>>,
}

impl SlintUiHost {
    /// Create a host plus its pending-event channel.
    pub fn new() -> Arc<Self> {
        let (sender, receiver) = mpsc::channel();
        Arc::new(Self {
            sender,
            receiver: Mutex::new(Some(receiver)),
        })
    }

    /// Take the receiving end once, when the Slint event loop starts.
    pub fn take_receiver(&self) -> Option<mpsc::Receiver<UiEvent>> {
        self.receiver.lock().ok().and_then(|mut slot| slot.take())
    }
}

impl UiHost for SlintUiHost {
    fn emit(&self, event: UiEvent) {
        // Core keeps running even if the UI has already closed.
        let _ = self.sender.send(event);
    }

    fn confirm(&self, _request: ConfirmRequest) -> bool {
        // Slint confirmation dialogs are added together with the first
        // destructive operations.
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

    fn open_external(&self, url: &str) -> Result<(), String> {
        open::that(url).map_err(|error| format!("打开链接失败 {url}: {error}"))
    }

    fn theme(&self) -> UiTheme {
        UiTheme::System
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forwards_events_to_receiver() {
        let host = SlintUiHost::new();
        host.emit(UiEvent::Status {
            message: "hello".to_string(),
        });

        let receiver = host.take_receiver().expect("receiver should be available");
        match receiver.try_recv() {
            Ok(UiEvent::Status { message }) => assert_eq!(message, "hello"),
            other => panic!("unexpected event: {other:?}"),
        }
        assert!(host.take_receiver().is_none());
    }
}
