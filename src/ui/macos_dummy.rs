use crate::ui::{KeysorUi, ClickType};

pub struct MacosDummyUi;

impl MacosDummyUi {
    pub fn new() -> Self {
        MacosDummyUi
    }
}

impl KeysorUi for MacosDummyUi {
    fn start(&self) -> Result<(), String> {
        println!("[UI] macOS Dummy UI started.");
        Ok(())
    }

    fn show(&self, _visible: bool) {
        // No-op for dummy UI
    }

    fn update_position(&self) {
        // No-op for dummy UI
    }

    fn trigger_click_motion(&self, _click_type: ClickType) {
        // No-op for dummy UI
    }

    fn check_magnetic_snapping(&self) {
        // No-op for dummy UI
    }



    fn check_global_magnetic_snapping(&self) {
        // No-op for dummy UI
    }

    fn is_currently_snapped(&self) -> bool {
        false
    }
}
