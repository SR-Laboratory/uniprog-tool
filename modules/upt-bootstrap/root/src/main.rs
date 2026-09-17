#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod app_ops;
pub mod boot;
pub mod l0_core;

#[cfg(feature = "ui-slint")]
pub mod ui_slint;
#[cfg(not(feature = "ui-slint"))]
pub mod ui_tauri;

/// Tauri shell (default).
#[cfg(not(feature = "ui-slint"))]
fn main() {
    use std::sync::Arc;
    use ui_tauri::ui_host::TauriUiHost;

    let ui_host = Arc::new(TauriUiHost::new());
    let runtime = match boot::boot_with_ui(ui_host.clone()) {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("启动失败: {error}");
            std::process::exit(1);
        }
    };

    ui_tauri::run(runtime, ui_host);
}

/// Slint shell skeleton (compile-time switch).
#[cfg(feature = "ui-slint")]
fn main() {
    use l0_core::ui::NullUiHost;
    use std::sync::Arc;

    let runtime = match boot::boot_with_ui(Arc::new(NullUiHost)) {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("启动失败: {error}");
            std::process::exit(1);
        }
    };

    if let Err(error) = ui_slint::run(runtime) {
        eprintln!("Slint 启动失败: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use crate::app_ops::core;

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
