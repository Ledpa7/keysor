pub use crate::ui::ClickType;

/// Shared atomic flag to force target refresh upon clicks
pub static FORCE_UIA_REFRESH: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn start_indicator() {
    if let Err(e) = crate::ui::get_ui().start() {
        eprintln!("[Error] Failed to start UI: {}", e);
    }
}

pub fn show_indicator() {
    crate::ui::get_ui().show(true);
}

pub fn hide_indicator() {
    crate::ui::get_ui().show(false);
}

pub fn update_indicator_position() {
    crate::ui::get_ui().update_position();
}

pub fn trigger_click_motion(click_type: ClickType) {
    crate::ui::get_ui().trigger_click_motion(click_type);
}

pub fn check_magnetic_snapping() {
    crate::ui::get_ui().check_magnetic_snapping();
}

pub fn check_global_magnetic_snapping() {
    crate::ui::get_ui().check_global_magnetic_snapping();
}

pub fn is_currently_snapped() -> bool {
    crate::ui::get_ui().is_currently_snapped()
}
