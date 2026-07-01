use crate::platform::{KeyboardHook, KeyEvent, HookResult};
use std::ffi::c_void;

// macOS CoreFoundation / CoreGraphics 바인딩
#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFRunLoopGetCurrent() -> *mut c_void;
    fn CFRunLoopRun();
    fn CFRunLoopStop(rl: *mut c_void);
    fn CFMachPortCreateRunLoopSource(
        allocator: *mut c_void,
        port: *mut c_void,
        order: isize,
    ) -> *mut c_void;
    fn CFRunLoopAddSource(rl: *mut c_void, source: *mut c_void, mode: *const c_void);
    fn CFRelease(obj: *mut c_void);
}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventTapCreate(
        tap: u32,
        place: u32,
        options: u32,
        eventsOfInterest: u64,
        callback: CGEventTapCallBack,
        refcon: *mut c_void,
    ) -> *mut c_void;
    fn CGEventGetIntegerValueField(event: *mut c_void, field: u32) -> i64;
    fn CGEventSourceKeyState(stateID: u32, key: u16) -> bool;
}

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> bool;
}

type CGEventTapCallBack = extern "C" fn(
    proxy: *mut c_void,
    etype: u32,
    event: *mut c_void,
    refcon: *mut c_void,
) -> *mut c_void;

// CoreGraphics Event Tap 관련 상수
const K_CG_HID_EVENT_TAP: u32 = 0;
const K_CG_HEAD_INSERT_EVENT_TAP: u32 = 0;
const K_CG_EVENT_TAP_OPTION_DEFAULT: u32 = 0;

// 이벤트 종류
const K_CG_EVENT_KEY_DOWN: u32 = 10;
const K_CG_EVENT_KEY_UP: u32 = 11;

// CGEventField
const K_CG_KEYBOARD_EVENT_AUTOREPEAT: u32 = 8;
const K_CG_KEYBOARD_EVENT_KEYCODE: u32 = 9;

// RunLoop 모드 전역 상수 포인터
#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    static kCFRunLoopCommonModes: *const c_void;
}

use std::sync::Mutex;
static GLOBAL_CALLBACK: Mutex<Option<Box<dyn Fn(KeyEvent) -> HookResult + Send + Sync + 'static>>> = Mutex::new(None);
static mut RUN_LOOP_REF: *mut c_void = std::ptr::null_mut();

// C 스타일 EventTap 콜백 함수
extern "C" fn event_tap_callback(
    _proxy: *mut c_void,
    etype: u32,
    event: *mut c_void,
    _refcon: *mut c_void,
) -> *mut c_void {
    if etype == K_CG_EVENT_KEY_DOWN || etype == K_CG_EVENT_KEY_UP {
        let keycode = unsafe { CGEventGetIntegerValueField(event, K_CG_KEYBOARD_EVENT_KEYCODE) } as u32;
        let is_autorepeat = unsafe { CGEventGetIntegerValueField(event, K_CG_KEYBOARD_EVENT_AUTOREPEAT) } != 0;
        let is_keydown = etype == K_CG_EVENT_KEY_DOWN;

        // autorepeat 이벤트는 무시
        if is_autorepeat && is_keydown {
            return event;
        }

        // Keysor KeyEvent 매핑
        let mut key_event = KeyEvent {
            vk_code: keycode,
            is_keydown,
            is_keyup: !is_keydown,
            is_injected_by_keysor: false,
        };

        // Mac 가상 키 매핑 조율 (예: macOS CapsLock = 57)
        if keycode == 57 {
            key_event.vk_code = 0x14; // VK_CAPITAL 매핑하여 공통 비즈니스 로직 연동
        }

        let callback_guard = GLOBAL_CALLBACK.lock().unwrap();
        if let Some(ref cb) = *callback_guard {
            match cb(key_event) {
                HookResult::Block => {
                    // 이벤트를 가로채고 OS 전송을 차단 (NULL 반환)
                    return std::ptr::null_mut();
                }
                HookResult::Pass => {}
            }
        }
    }
    
    event
}

pub struct MacosKeyboardHook;

impl MacosKeyboardHook {
    pub fn new() -> Self {
        MacosKeyboardHook
    }
}

impl KeyboardHook for MacosKeyboardHook {
    fn start_listening(
        &self,
        callback: Box<dyn Fn(KeyEvent) -> HookResult + Send + Sync + 'static>,
    ) -> Result<(), String> {
        // macOS Accessibility(손쉬운 사용) 권한 체크
        unsafe {
            if !AXIsProcessTrusted() {
                eprintln!("[Error] Keysor does not have Accessibility permissions. Prompting user...");
                // macOS 개인정보 보호 및 보안 -> 손쉬운 사용 탭 바로 열기
                let _ = std::process::Command::new("open")
                    .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
                    .spawn();
                return Err("Accessibility permission is required. Opened System Preferences.".to_string());
            }
        }

        {
            let mut cb = GLOBAL_CALLBACK.lock().unwrap();
            *cb = Some(callback);
        }

        std::thread::spawn(|| unsafe {
            let event_mask = (1u64 << K_CG_EVENT_KEY_DOWN) | (1u64 << K_CG_EVENT_KEY_UP);
            
            let port = CGEventTapCreate(
                K_CG_HID_EVENT_TAP,
                K_CG_HEAD_INSERT_EVENT_TAP,
                K_CG_EVENT_TAP_OPTION_DEFAULT,
                event_mask,
                event_tap_callback,
                std::ptr::null_mut(),
            );

            if port.is_null() {
                eprintln!("[Error] Failed to create CGEventTap despite trusted status.");
                return;
            }

            let source = CFMachPortCreateRunLoopSource(std::ptr::null_mut(), port, 0);
            if source.is_null() {
                CFRelease(port);
                eprintln!("[Error] Failed to create CFRunLoopSource.");
                return;
            }

            let run_loop = CFRunLoopGetCurrent();
            RUN_LOOP_REF = run_loop;

            CFRunLoopAddSource(run_loop, source, kCFRunLoopCommonModes);
            CFRelease(port);
            CFRelease(source);

            println!("[Info] macOS CFRunLoop starting for Keyboard Hook...");
            CFRunLoopRun();
        });

        Ok(())
    }

    fn stop_listening(&self) {
        unsafe {
            if !RUN_LOOP_REF.is_null() {
                CFRunLoopStop(RUN_LOOP_REF);
                RUN_LOOP_REF = std::ptr::null_mut();
            }
        }
        let mut cb = GLOBAL_CALLBACK.lock().unwrap();
        *cb = None;
    }

    fn modifier_sync_guard(&self, is_mouse_mode: bool, is_toggle_mode: bool, on_deactivate: fn()) {
        if is_mouse_mode && !is_toggle_mode {
            // 홀드 모드에서 물리 Caps Lock(57) 키에서 손이 떼어졌는지 하드웨어 실시간 체크
            unsafe {
                let physical_caps_pressed = CGEventSourceKeyState(0, 57);
                if !physical_caps_pressed {
                    println!("[Sync Guard] Caps Lock physical key released in macOS. Resetting mouse mode.");
                    on_deactivate();
                }
            }
        }
    }
}
