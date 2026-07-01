use std::sync::OnceLock;

#[allow(dead_code)]
pub enum MouseButton {
    Left,
    Right,
}

#[derive(Debug, Clone)]
pub struct KeyEvent {
    pub vk_code: u32,
    pub is_keydown: bool,
    pub is_keyup: bool,
    pub is_injected_by_keysor: bool,
}

pub enum HookResult {
    Pass,
    Block,
}

/// OS-dependent mouse and system control functions.
pub trait SystemController: Send + Sync + 'static {
    fn get_cursor_pos(&self) -> (i32, i32);
    fn set_cursor_pos(&self, x: i32, y: i32) -> bool;
    fn move_relative(&self, dx: i32, dy: i32);
    
    fn left_down(&self);
    fn left_up(&self);
    fn left_click(&self);
    fn left_double_click(&self);
    
    fn right_down(&self);
    fn right_up(&self);
    fn right_click(&self);
    
    #[allow(dead_code)]
    fn send_click(&self, button: MouseButton, press: bool);
    
    fn scroll(&self, amount: i32);
    fn scroll_horizontal(&self, amount: i32);
    
    fn get_dpi_scale(&self) -> f64;
    
    #[allow(dead_code)]
    fn register_startup(&self, active: bool) -> Result<(), String>;
    fn simulate_browser_navigation(&self, forward: bool);
    fn simulate_virtual_desktop_navigation(&self, forward: bool);
    fn simulate_page_jump(&self, top: bool);
    fn simulate_tab_navigation(&self, forward: bool);
    fn run_app(&self, app_path: &str) -> Result<(), String>;
    fn ensure_caps_lock_off(&self);
    fn inject_caps_lock_toggle(&self);
    fn beep(&self);
}

/// OS-dependent low-level keyboard hook interface.
pub trait KeyboardHook: Send + Sync + 'static {
    fn start_listening(&self, callback: Box<dyn Fn(KeyEvent) -> HookResult + Send + Sync + 'static>) -> Result<(), String>;
    fn stop_listening(&self);
    fn modifier_sync_guard(&self, is_mouse_mode: bool, is_toggle_mode: bool, on_deactivate: fn());
}

// OS-specific module selection
#[cfg(windows)]
pub mod windows;

#[cfg(target_os = "macos")]
pub mod macos;

static SYSTEM_CONTROLLER: OnceLock<Box<dyn SystemController>> = OnceLock::new();

/// Retrieve the global system controller instance.
pub fn get_system_controller() -> &'static dyn SystemController {
    SYSTEM_CONTROLLER.get_or_init(|| {
        #[cfg(windows)]
        {
            Box::new(windows::WindowsSystemController::new())
        }
        #[cfg(target_os = "macos")]
        {
            Box::new(macos::MacosSystemController::new())
        }
        #[cfg(not(any(windows, target_os = "macos")))]
        {
            panic!("Unsupported platform");
        }
    }).as_ref()
}

/// Factory function to create the active keyboard hook.
#[cfg(windows)]
pub fn create_keyboard_hook() -> Box<dyn KeyboardHook> {
    Box::new(windows::WindowsKeyboardHook::new())
}

#[cfg(target_os = "macos")]
pub fn create_keyboard_hook() -> Box<dyn KeyboardHook> {
    Box::new(macos::MacosKeyboardHook::new())
}
