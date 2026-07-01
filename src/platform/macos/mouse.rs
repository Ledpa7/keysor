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
extern "C" {
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
const K_CG_HID_EVENT_TAP: u32 = 0;

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
                CGEventPost(K_CG_HID_EVENT_TAP, event);
                CFRelease(event);
            }
        }
    }
}

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
            CGWarpMouseCursorPosition(pos) == 0
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
            let event = CGEventCreateScrollWheelEvent(
                std::ptr::null_mut(),
                0,
                1,
                amount / 10,
            );
            if !event.is_null() {
                CGEventPost(K_CG_HID_EVENT_TAP, event);
                CFRelease(event);
            }
        }
    }

    fn scroll_horizontal(&self, amount: i32) {
        unsafe {
            let event = CGEventCreateScrollWheelEvent(
                std::ptr::null_mut(),
                0,
                2,
                0,
                amount / 10,
            );
            if !event.is_null() {
                CGEventPost(K_CG_HID_EVENT_TAP, event);
                CFRelease(event);
            }
        }
    }

    fn get_dpi_scale(&self) -> f64 {
        1.0
    }

    fn register_startup(&self, _active: bool) -> Result<(), String> {
        Ok(())
    }

    fn simulate_browser_navigation(&self, _forward: bool) {}
    fn simulate_virtual_desktop_navigation(&self, _forward: bool) {}
    fn simulate_page_jump(&self, _top: bool) {}
    fn simulate_tab_navigation(&self, _forward: bool) {}

    fn run_app(&self, app_path: &str) -> Result<(), String> {
        std::process::Command::new("open")
            .arg(app_path)
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    fn ensure_caps_lock_off(&self) {}
    fn inject_caps_lock_toggle(&self) {}

    fn beep(&self) {
        print!("\x07");
    }
}
