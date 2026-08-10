use crate::platform::{SystemController, MouseButton};
use std::ffi::c_void;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct CGPoint {
    x: f64,
    y: f64,
}

// macOS CoreGraphics 프레임워크 동적 링킹 선언
#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGEventCreate(source: *mut c_void) -> *mut c_void;
    fn CGEventGetLocation(event: *mut c_void) -> CGPoint;
    fn CGEventCreateMouseEvent(
        source: *mut c_void,
        mouseType: u32,
        mouseCursorPosition: CGPoint,
        mouseButton: u32,
    ) -> *mut c_void;
    fn CGEventCreateScrollWheelEvent(
        source: *mut c_void,
        units: u32,
        wheelCount: u32,
        wheel1: i32,
        ...
    ) -> *mut c_void;
    fn CGEventPost(tap: u32, event: *mut c_void);
    fn CGWarpMouseCursorPosition(newCursorPosition: CGPoint) -> i32;
    fn CFRelease(obj: *mut c_void);
}

// CoreGraphics 마우스 이벤트 타입 상수
const K_CG_SESSION_EVENT_TAP: u32 = 1;

const K_CG_EVENT_LEFT_MOUSE_DOWN: u32 = 1;
const K_CG_EVENT_LEFT_MOUSE_UP: u32 = 2;
const K_CG_EVENT_RIGHT_MOUSE_DOWN: u32 = 3;
const K_CG_EVENT_RIGHT_MOUSE_UP: u32 = 4;

const K_CG_MOUSE_BUTTON_LEFT: u32 = 0;
const K_CG_MOUSE_BUTTON_RIGHT: u32 = 1;

pub struct MacosSystemController;

impl MacosSystemController {
    pub fn new() -> Self {
        MacosSystemController
    }

    // 특정 마우스 클릭 이벤트를 대상 좌표에 생성하여 전송하는 헬퍼 함수
    fn post_mouse_event(&self, mouse_type: u32, button: u32) {
        let (cx, cy) = self.get_cursor_pos();
        let pos = CGPoint { x: cx as f64, y: cy as f64 };
        unsafe {
            let event = CGEventCreateMouseEvent(std::ptr::null_mut(), mouse_type, pos, button);
            if !event.is_null() {
                CGEventPost(K_CG_SESSION_EVENT_TAP, event);
                CFRelease(event);
            }
        }
    }

    // 특정 키보드 입력 이벤트를 합성 전송하는 헬퍼 함수
    fn post_key_event(&self, keycode: u16, flags: u64) {
        unsafe {
            unsafe extern "C" {
                fn CGEventCreateKeyboardEvent(source: *mut c_void, virtualKey: u16, keyDown: bool) -> *mut c_void;
                fn CGEventSetFlags(event: *mut c_void, flags: u64);
                fn CGEventSetIntegerValueField(event: *mut c_void, field: u32, value: i64);
            }
            let down = CGEventCreateKeyboardEvent(std::ptr::null_mut(), keycode, true);
            if !down.is_null() {
                CGEventSetIntegerValueField(down, 42, 0x4B455953); // K_CG_EVENT_SOURCE_USER_DATA = 42, KEYSOR_MAGIC = 0x4B455953
                if flags != 0 {
                    CGEventSetFlags(down, flags);
                }
                CGEventPost(K_CG_SESSION_EVENT_TAP, down);
                CFRelease(down);
            }
            let up = CGEventCreateKeyboardEvent(std::ptr::null_mut(), keycode, false);
            if !up.is_null() {
                CGEventSetIntegerValueField(up, 42, 0x4B455953); // K_CG_EVENT_SOURCE_USER_DATA = 42, KEYSOR_MAGIC = 0x4B455953
                if flags != 0 {
                    CGEventSetFlags(up, flags);
                }
                CGEventPost(K_CG_SESSION_EVENT_TAP, up);
                CFRelease(up);
            }
        }
    }

    pub fn is_caps_lock_on(&self) -> bool {
        unsafe {
            unsafe extern "C" {
                fn CGEventSourceFlagsState(stateID: u32) -> u64;
            }
            // stateID 1 = K_CG_EVENT_SOURCE_STATE_COMBINED_SESSION_STATE
            // 0x00010000 = K_CG_EVENT_FLAG_MASK_ALPHA_SHIFT (Caps Lock)
            (CGEventSourceFlagsState(1) & 0x00010000) != 0
        }
    }
}

const K_CG_EVENT_MOUSE_MOVED: u32 = 5;

impl SystemController for MacosSystemController {
    fn get_cursor_pos(&self) -> (i32, i32) {
        unsafe {
            let event = CGEventCreate(std::ptr::null_mut());
            if !event.is_null() {
                let loc = CGEventGetLocation(event);
                CFRelease(event);
                (loc.x as i32, loc.y as i32)
            } else {
                (0, 0)
            }
        }
    }

    fn set_cursor_pos(&self, x: i32, y: i32) -> bool {
        unsafe {
            let pos = CGPoint { x: x as f64, y: y as f64 };
            let res = CGWarpMouseCursorPosition(pos) == 0;
            let event = CGEventCreateMouseEvent(std::ptr::null_mut(), K_CG_EVENT_MOUSE_MOVED, pos, K_CG_MOUSE_BUTTON_LEFT);
            if !event.is_null() {
                CGEventPost(K_CG_SESSION_EVENT_TAP, event);
                CFRelease(event);
            }
            res
        }
    }

    fn move_relative(&self, dx: i32, dy: i32) {
        let (cx, cy) = self.get_cursor_pos();
        self.set_cursor_pos(cx + dx, cy + dy);
    }

    fn left_down(&self) {
        self.post_mouse_event(K_CG_EVENT_LEFT_MOUSE_DOWN, K_CG_MOUSE_BUTTON_LEFT);
    }

    fn left_up(&self) {
        self.post_mouse_event(K_CG_EVENT_LEFT_MOUSE_UP, K_CG_MOUSE_BUTTON_LEFT);
    }

    fn left_click(&self) {
        self.left_down();
        std::thread::sleep(std::time::Duration::from_millis(10));
        self.left_up();
    }

    fn left_double_click(&self) {
        self.left_click();
        std::thread::sleep(std::time::Duration::from_millis(100));
        self.left_click();
    }

    fn right_down(&self) {
        self.post_mouse_event(K_CG_EVENT_RIGHT_MOUSE_DOWN, K_CG_MOUSE_BUTTON_RIGHT);
    }

    fn right_up(&self) {
        self.post_mouse_event(K_CG_EVENT_RIGHT_MOUSE_UP, K_CG_MOUSE_BUTTON_RIGHT);
    }

    fn right_click(&self) {
        self.right_down();
        std::thread::sleep(std::time::Duration::from_millis(10));
        self.right_up();
    }

    fn send_click(&self, button: MouseButton, press: bool) {
        match (button, press) {
            (MouseButton::Left, true) => self.left_down(),
            (MouseButton::Left, false) => self.left_up(),
            (MouseButton::Right, true) => self.right_down(),
            (MouseButton::Right, false) => self.right_up(),
            _ => {}
        }
    }

    fn scroll(&self, amount: i32) {
        unsafe {
            // R/F 키 스크롤 이동 속도 3배 가속 적용
            let scaled_amount = if amount > 0 {
                (amount * 3 / 10).max(1)
            } else if amount < 0 {
                (amount * 3 / 10).min(-1)
            } else {
                0
            };
            let event = CGEventCreateScrollWheelEvent(
                std::ptr::null_mut(),
                0,
                1,
                scaled_amount,
            );
            if !event.is_null() {
                CGEventPost(K_CG_SESSION_EVENT_TAP, event);
                CFRelease(event);
            }
        }
    }

    fn scroll_horizontal(&self, amount: i32) {
        unsafe {
            let scaled_amount = if amount > 0 {
                (amount * 3 / 10).max(1)
            } else if amount < 0 {
                (amount * 3 / 10).min(-1)
            } else {
                0
            };
            let event = CGEventCreateScrollWheelEvent(
                std::ptr::null_mut(),
                0,
                2,
                0,
                scaled_amount,
            );
            if !event.is_null() {
                CGEventPost(K_CG_SESSION_EVENT_TAP, event);
                CFRelease(event);
            }
        }
    }

    fn get_dpi_scale(&self) -> f64 {
        1.0
    }

    fn register_startup(&self, active: bool) -> Result<(), String> {
        let mut plist_path = std::env::var("HOME")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::path::PathBuf::from("."));
        plist_path.push("Library");
        plist_path.push("LaunchAgents");
        plist_path.push("com.keysor.app.plist");

        if active {
            if let Ok(exe_path) = std::env::current_exe() {
                if let Some(parent) = plist_path.parent() {
                    std::fs::create_dir_all(parent).ok();
                }
                let content = format!(
                    r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.keysor.app</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
</dict>
</plist>"#,
                    exe_path.to_string_lossy()
                );
                std::fs::write(&plist_path, content).map_err(|e| e.to_string())
            } else {
                Err("Cannot retrieve current executable path".to_string())
            }
        } else {
            if plist_path.exists() {
                std::fs::remove_file(&plist_path).map_err(|e| e.to_string())
            } else {
                Ok(())
            }
        }
    }

    fn simulate_browser_navigation(&self, forward: bool) {
        // macOS: Cmd + ] (Forward, keycode 30) / Cmd + [ (Back, keycode 33)
        let keycode = if forward { 30 } else { 33 };
        self.post_key_event(keycode, 0x00100000); // K_CG_EVENT_FLAG_MASK_COMMAND
    }

    fn simulate_virtual_desktop_navigation(&self, forward: bool) {
        // macOS Spaces: Ctrl + Right Arrow (keycode 124) / Ctrl + Left Arrow (keycode 123)
        let keycode = if forward { 124 } else { 123 };
        self.post_key_event(keycode, 0x00040000); // K_CG_EVENT_FLAG_MASK_CONTROL
    }

    fn simulate_page_jump(&self, top: bool) {
        let amount = if top { 1000 } else { -1000 };
        self.scroll(amount);
    }

    fn simulate_tab_navigation(&self, forward: bool) {
        // macOS: Ctrl + Tab (keycode 48) / Ctrl + Shift + Tab (keycode 48 with Ctrl+Shift flags)
        if forward {
            self.post_key_event(48, 0x00040000);
        } else {
            self.post_key_event(48, 0x00040000 | 0x00020000);
        }
    }

    fn run_app(&self, app_path: &str) -> Result<(), String> {
        std::process::Command::new("open")
            .arg(app_path)
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    fn ensure_caps_lock_off(&self) {
        if self.is_caps_lock_on() {
            self.inject_caps_lock_toggle();
        }
    }

    fn is_caps_lock_on(&self) -> bool {
        Self::is_caps_lock_on(self)
    }

    fn inject_caps_lock_toggle(&self) {
        self.post_key_event(57, 0);
    }

    fn beep(&self) {
        print!("\x07");
    }
}
