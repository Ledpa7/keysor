use crate::platform::{KeyboardHook, KeyEvent, HookResult};
use std::ffi::c_void;
use std::sync::Mutex;

// macOS CoreFoundation / CoreGraphics 바인딩
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
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
unsafe extern "C" {
    fn CGEventTapCreate(
        tap: u32,
        place: u32,
        options: u32,
        eventsOfInterest: u64,
        callback: CGEventTapCallBack,
        refcon: *mut c_void,
    ) -> *mut c_void;
    fn CGEventTapEnable(tap: *mut c_void, enable: bool);
    fn CGEventGetIntegerValueField(event: *mut c_void, field: u32) -> i64;
    fn CGEventSourceKeyState(stateID: u32, key: u16) -> bool;
    fn CGEventGetFlags(event: *mut c_void) -> u64;
}

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn AXIsProcessTrusted() -> bool;
}

type CGEventTapCallBack = extern "C" fn(
    proxy: *mut c_void,
    etype: u32,
    event: *mut c_void,
    refcon: *mut c_void,
) -> *mut c_void;

const K_CG_HID_EVENT_TAP: u32 = 0;
const K_CG_SESSION_EVENT_TAP: u32 = 1;
const K_CG_HEAD_INSERT_EVENT_TAP: u32 = 0;
const K_CG_EVENT_TAP_OPTION_DEFAULT: u32 = 0;

// 이벤트 종류
const K_CG_EVENT_KEY_DOWN: u32 = 10;
const K_CG_EVENT_KEY_UP: u32 = 11;
const K_CG_EVENT_FLAGS_CHANGED: u32 = 12;

// CGEventField
const K_CG_KEYBOARD_EVENT_AUTOREPEAT: u32 = 8;
const K_CG_KEYBOARD_EVENT_KEYCODE: u32 = 9;

// RunLoop 모드 전역 상수 포인터
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    static kCFRunLoopCommonModes: *const c_void;
}

static GLOBAL_CALLBACK: Mutex<Option<Box<dyn Fn(KeyEvent) -> HookResult + Send + Sync + 'static>>> = Mutex::new(None);
static mut RUN_LOOP_REF: *mut c_void = std::ptr::null_mut();
static mut EVENT_TAP_PORT: *mut c_void = std::ptr::null_mut();

/// macOS 키코드를 표준 Windows Virtual-Key(VK) 코드로 매핑
fn macos_keycode_to_vk(keycode: u32) -> u32 {
    match keycode {
        0 => 0x41,   // A
        1 => 0x53,   // S
        2 => 0x44,   // D
        3 => 0x46,   // F
        4 => 0x48,   // H
        5 => 0x47,   // G
        6 => 0x5A,   // Z
        7 => 0x58,   // X
        8 => 0x43,   // C
        9 => 0x56,   // V
        11 => 0x42,  // B
        12 => 0x51,  // Q
        13 => 0x57,  // W
        14 => 0x45,  // E
        15 => 0x52,  // R
        16 => 0x59,  // Y
        17 => 0x54,  // T
        18 => 0x31,  // 1
        19 => 0x32,  // 2
        20 => 0x33,  // 3
        21 => 0x34,  // 4
        22 => 0x36,  // 6
        23 => 0x35,  // 5
        24 => 0xBB,  // =
        25 => 0x39,  // 9
        26 => 0x37,  // 7
        27 => 0xBD,  // -
        28 => 0x38,  // 8
        29 => 0x30,  // 0
        30 => 0xDD,  // ]
        31 => 0x4F,  // O
        32 => 0x55,  // U
        33 => 0xDB,  // [
        34 => 0x49,  // I
        35 => 0x50,  // P
        36 => 0x0D,  // Return / Enter
        37 => 0x4C,  // L
        38 => 0x4A,  // J
        39 => 0xDE,  // '
        40 => 0x4B,  // K
        41 => 0xBA,  // ;
        42 => 0xDC,  // \
        43 => 0xBC,  // ,
        44 => 0xBF,  // /
        45 => 0x4E,  // N
        46 => 0x4D,  // M
        47 => 0xBE,  // .
        48 => 0x09,  // Tab
        49 => 0x20,  // Space
        50 => 0xC0,  // `
        51 => 0x08,  // Delete / Backspace
        53 => 0x1B,  // Escape
        55 => 0x5B,  // Cmd
        56 | 60 => 0x10, // Shift
        57 => 0x14,  // CapsLock
        58 => 0x12,  // Option / Alt
        59 => 0x11,  // Ctrl
        123 => 0x25, // Left Arrow
        124 => 0x27, // Right Arrow
        125 => 0x28, // Down Arrow
        126 => 0x26, // Up Arrow
        code => code,
    }
}

// C 스타일 EventTap 콜백 함수
extern "C" fn event_tap_callback(
    proxy: *mut c_void,
    etype: u32,
    event: *mut c_void,
    _refcon: *mut c_void,
) -> *mut c_void {
    // macOS 타임아웃/입력 전환으로 탭 비활성화 시 (14: Timeout, 15: UserInput) 즉시 자동 재활성화
    if etype == 14 || etype == 15 || etype == 0x0FFFFFFF || etype == 0xFFFFFFFF {
        unsafe {
            if !EVENT_TAP_PORT.is_null() {
                CGEventTapEnable(EVENT_TAP_PORT, true);
            }
            CGEventTapEnable(proxy, true);
        }
        return event;
    }

    if etype == K_CG_EVENT_KEY_DOWN || etype == K_CG_EVENT_KEY_UP || etype == K_CG_EVENT_FLAGS_CHANGED {
        let keycode = unsafe { CGEventGetIntegerValueField(event, K_CG_KEYBOARD_EVENT_KEYCODE) } as u32;
        let is_autorepeat = unsafe { CGEventGetIntegerValueField(event, K_CG_KEYBOARD_EVENT_AUTOREPEAT) } != 0;
        let flags = unsafe { CGEventGetFlags(event) };

        let is_keydown = if keycode == 57 {
            // Caps Lock (57): macOS 키보드는 Caps Lock 누름 시마다 FLAGS_CHANGED 토글 이벤트를 발생시키므로
            // 항상 is_keydown = true로 설정하여 100% 토글 토글 토글 안정적 작동
            true
        } else if etype == K_CG_EVENT_FLAGS_CHANGED {
            let mask = match keycode {
                56 | 60 => 0x20000, // Shift
                59 => 0x40000,      // Control
                58 | 61 => 0x80000, // Option / Alt
                55 | 54 => 0x100000,// Command / Win
                _ => 0,
            };
            if mask != 0 {
                (flags & mask) != 0
            } else {
                unsafe { CGEventSourceKeyState(0, keycode as u16) }
            }
        } else {
            etype == K_CG_EVENT_KEY_DOWN
        };

        if keycode == 57 || keycode == 13 || keycode == 0 || keycode == 1 || keycode == 2 {
            println!("[Debug EventTap] etype={}, keycode={}, flags=0x{:X}, computed_is_keydown={}", 
                etype, keycode, flags, is_keydown);
        }

        // autorepeat 이벤트 무시
        if is_autorepeat && is_keydown {
            return event;
        }

        let user_data = unsafe { CGEventGetIntegerValueField(event, 42) };
        let is_injected_by_keysor = user_data == 0x4B455953;

        // Keysor KeyEvent 매핑
        let key_event = KeyEvent {
            vk_code: macos_keycode_to_vk(keycode),
            is_keydown,
            is_keyup: !is_keydown,
            is_injected_by_keysor,
        };

        if let Ok(callback_guard) = GLOBAL_CALLBACK.try_lock() {
            if let Some(ref cb) = *callback_guard {
                match cb(key_event) {
                    HookResult::Block => {
                        // 이벤트를 가로채고 OS 전송 차단 (NULL 반환하여 입력 텍스트 방지)
                        return std::ptr::null_mut();
                    }
                    HookResult::Pass => {}
                }
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
            let event_mask = (1u64 << K_CG_EVENT_KEY_DOWN) | (1u64 << K_CG_EVENT_KEY_UP) | (1u64 << K_CG_EVENT_FLAGS_CHANGED);
            
            // 원격 데스크톱(VNC, Screen Sharing, AnyDesk 등) 및 세션 이벤트를 모두 가로채기 위해
            // K_CG_SESSION_EVENT_TAP (1) 우선 생성
            let mut port = CGEventTapCreate(
                K_CG_SESSION_EVENT_TAP,
                K_CG_HEAD_INSERT_EVENT_TAP,
                K_CG_EVENT_TAP_OPTION_DEFAULT,
                event_mask,
                event_tap_callback,
                std::ptr::null_mut(),
            );

            if port.is_null() {
                // Session Tap 실패 시 HID Tap으로 폴백
                port = CGEventTapCreate(
                    K_CG_HID_EVENT_TAP,
                    K_CG_HEAD_INSERT_EVENT_TAP,
                    K_CG_EVENT_TAP_OPTION_DEFAULT,
                    event_mask,
                    event_tap_callback,
                    std::ptr::null_mut(),
                );
            }

            if port.is_null() {
                eprintln!("[Error] Failed to create CGEventTap despite trusted status.");
                return;
            }

            EVENT_TAP_PORT = port;

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

            println!("[Info] macOS CFRunLoop starting for Keyboard Hook (Session & Remote Desktop)...");
            CFRunLoopRun();
        });

        Ok(())
    }

    fn stop_listening(&self) {
        unsafe {
            if !RUN_LOOP_REF.is_null() {
                CFRunLoopStop(RUN_LOOP_REF);
                RUN_LOOP_REF = std::ptr::null_mut();
                EVENT_TAP_PORT = std::ptr::null_mut();
            }
        }
        let mut cb = GLOBAL_CALLBACK.lock().unwrap();
        *cb = None;
    }

    fn modifier_sync_guard(&self, is_mouse_mode: bool, is_toggle_mode: bool, on_deactivate: fn()) {
        if is_mouse_mode && !is_toggle_mode {
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
