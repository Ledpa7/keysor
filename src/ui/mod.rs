use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClickType {
    None,
    Left,
    Right,
    Scroll,
}

pub trait KeysorUi: Send + Sync + 'static {
    /// Start the UI system (window loops, threads, etc.)
    fn start(&self) -> Result<(), String>;

    /// Set UI visibility
    fn show(&self, visible: bool);

    /// Notify UI that position needs update
    fn update_position(&self);

    /// Trigger visual click animations
    fn trigger_click_motion(&self, click_type: ClickType);

    /// Perform magnetic snapping calculation for HUD buttons
    fn check_magnetic_snapping(&self);



    /// Perform global magnetic snapping check
    fn check_global_magnetic_snapping(&self);

    /// Check if the cursor is currently snapped
    fn is_currently_snapped(&self) -> bool;
}

#[cfg(windows)]
pub mod win_gdi;

#[cfg(windows)]
pub mod win_uia;

#[cfg(target_os = "macos")]
pub mod macos_dummy;

static ACTIVE_UI: OnceLock<Box<dyn KeysorUi>> = OnceLock::new();

/// Get reference to the active platform UI controller
pub fn get_ui() -> &'static dyn KeysorUi {
    ACTIVE_UI.get_or_init(|| {
        #[cfg(windows)]
        {
            Box::new(win_gdi::WindowsGdiUi::new())
        }
        #[cfg(target_os = "macos")]
        {
            Box::new(macos_dummy::MacosDummyUi::new())
        }
        #[cfg(not(any(windows, target_os = "macos")))]
        {
            panic!("Unsupported platform");
        }
    }).as_ref()
}
