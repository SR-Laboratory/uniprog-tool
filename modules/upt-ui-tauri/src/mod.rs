//! UI shell (Tauri) modules. Everything that depends on the Tauri crate
//! should live under this module so the L0 core stays transport-agnostic.

pub mod commands;
pub mod dialogs;
pub mod run;
pub mod ui_host;

pub use run::run;
