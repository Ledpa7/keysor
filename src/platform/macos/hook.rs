use crate::platform::{KeyboardHook, KeyEvent, HookResult};
use std::ffi::c_void;
use std::sync::{Arc, Mutex, atomic::{AtomicBool, AtomicPtr, AtomicU8, Ordering}};
use std::time::Duration;

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
    fn CFRunLoopRemoveSource(rl: *mut c_void, source: *mut c_void, mode: *const c_void);
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
    fn CGEventTapIsEnabled(tap: *mut c_void) -> bool;
    fn CGEventSourceFlagsState(stateID: u32) -> u64;
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
const K_CG_EVENT_TAP_DISABLED_BY_TIMEOUT: u32 = 0xFFFFFFFE;
const K_CG_EVENT_TAP_DISABLED_BY_USER_INPUT: u32 = 0xFFFFFFFF;

// CGEventField
const K_CG_KEYBOARD_EVENT_AUTOREPEAT: u32 = 8;
const K_CG_KEYBOARD_EVENT_KEYCODE: u32 = 9;

// RunLoop 모드 전역 상수 포인터
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    static kCFRunLoopCommonModes: *const c_void;
}

static GLOBAL_CALLBACK: Mutex<Option<Arc<dyn Fn(KeyEvent) -> HookResult + Send + Sync + 'static>>> = Mutex::new(None);
static RUN_LOOP_REF: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());
static EVENT_TAP_PORT: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());
static STOP_REQUESTED: AtomicBool = AtomicBool::new(false);
static HOOK_STATUS: AtomicU8 = AtomicU8::new(0);

pub fn hook_status() -> u8 { HOOK_STATUS.load(Ordering::SeqCst) }

thread_local! {
    static LAST_CAPS_FLAG: std::cell::Cell<Option<bool>> = const { std::cell::Cell::new(None) };
}

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
        54 => 0x5C,  // Right Cmd
        55 => 0x5B,  // Cmd
        56 | 60 => 0x10, // Shift
        57 => 0x14,  // CapsLock
        58 | 61 => 0x12,  // Option / Alt
        59 | 62 => 0x11,  // Ctrl
        123 => 0x25, // Left Arrow
        124 => 0x27, // Right Arrow
        125 => 0x28, // Down Arrow
        126 => 0x26, // Up Arrow
        code => code,
    }
}

fn dispatch_event(keycode: u32, is_keydown: bool, injected: bool) -> bool {
    let event = KeyEvent {
        vk_code: macos_keycode_to_vk(keycode),
        is_keydown,
        is_keyup: !is_keydown,
        is_injected_by_keysor: injected,
    };
    let callback = GLOBAL_CALLBACK.lock().ok().and_then(|callback| callback.clone());
    callback.is_some_and(|cb| matches!(cb(event), HookResult::Block))
}

fn physical_key_down(keycode: u16) -> bool {
    unsafe { CGEventSourceKeyState(0, keycode) }
}

fn configured_key_is_down(vk: u32) -> Option<bool> {
    if vk == 0x14 { return None; } // CapsLock is handled by flagsChanged events.
    let keys: &[u16] = match vk {
        0x10 => &[56, 60],
        0x11 => &[59, 62],
        0x12 => &[58, 61],
        _ => {
            let code = (0..=127).find(|&code| macos_keycode_to_vk(code) == vk)?;
            return Some(physical_key_down(code as u16));
        }
    };
    Some(keys.iter().any(|&key| physical_key_down(key)))
}

extern "C" fn event_tap_callback(
    _proxy: *mut c_void,
    etype: u32,
    event: *mut c_void,
    _refcon: *mut c_void,
) -> *mut c_void {
    if matches!(etype, K_CG_EVENT_TAP_DISABLED_BY_TIMEOUT | K_CG_EVENT_TAP_DISABLED_BY_USER_INPUT) {
        crate::hook::cancel_active_input();
        HOOK_STATUS.store(3, Ordering::SeqCst);
        let port = EVENT_TAP_PORT.load(Ordering::SeqCst);
        unsafe {
            if !port.is_null() {
                CGEventTapEnable(port, true);
                if CGEventTapIsEnabled(port) {
                    HOOK_STATUS.store(2, Ordering::SeqCst);
                } else {
                    let run_loop = RUN_LOOP_REF.load(Ordering::SeqCst);
                    if !run_loop.is_null() { CFRunLoopStop(run_loop); }
                }
            }
        }
        return event;
    }

    if !matches!(etype, K_CG_EVENT_KEY_DOWN | K_CG_EVENT_KEY_UP | K_CG_EVENT_FLAGS_CHANGED) {
        return event;
    }
    let keycode = unsafe { CGEventGetIntegerValueField(event, K_CG_KEYBOARD_EVENT_KEYCODE) } as u32;
    let injected = unsafe { CGEventGetIntegerValueField(event, 42) } == 0x4B455953;
    let flags = unsafe { CGEventGetFlags(event) };

    if keycode == 57 && etype != K_CG_EVENT_FLAGS_CHANGED {
        return event;
    }
    if keycode == 57 {
        let caps_enabled = flags & 0x10000 != 0;
        let changed = LAST_CAPS_FLAG.with(|last| {
            let changed = last.get() != Some(caps_enabled);
            last.set(Some(caps_enabled));
            changed
        });
        let toggle_mode = crate::hook::APP_STATE.get()
            .and_then(|state| state.lock().ok())
            .is_some_and(|state| state.config.settings.modifier_mode.eq_ignore_ascii_case("Toggle"));
        if toggle_mode {
            if !changed { return event; }
            let blocked = dispatch_event(keycode, true, injected);
            let released = dispatch_event(keycode, false, injected);
            return if blocked || released { std::ptr::null_mut() } else { event };
        }
        // CapsLock changes its flag on press, then emits an unchanged flag on release.
        return if dispatch_event(keycode, changed, injected) { std::ptr::null_mut() } else { event };
    }

    let is_keydown = if etype == K_CG_EVENT_FLAGS_CHANGED {
        let mask = match keycode {
            56 | 60 => 0x20000,  // Shift
            59 | 62 => 0x40000, // Control
            58 | 61 => 0x80000, // Option
            55 | 54 => 0x100000, // Command
            _ => 0,
        };
        if mask != 0 { flags & mask != 0 } else { physical_key_down(keycode as u16) }
    } else {
        etype == K_CG_EVENT_KEY_DOWN
    };
    if is_keydown && unsafe { CGEventGetIntegerValueField(event, K_CG_KEYBOARD_EVENT_AUTOREPEAT) } != 0
        && !matches!(keycode, 3 | 15 | 49) {
        return event;
    }
    if dispatch_event(keycode, is_keydown, injected) { std::ptr::null_mut() } else { event }
}

pub struct MacosKeyboardHook;

impl MacosKeyboardHook {
    pub fn new() -> Self { Self }
}

impl KeyboardHook for MacosKeyboardHook {
    fn start_listening(
        &self,
        callback: Box<dyn Fn(KeyEvent) -> HookResult + Send + Sync + 'static>,
    ) -> Result<(), String> {
        *GLOBAL_CALLBACK.lock().map_err(|e| e.to_string())? = Some(Arc::from(callback));
        STOP_REQUESTED.store(false, Ordering::SeqCst);
        HOOK_STATUS.store(0, Ordering::SeqCst);
        std::thread::Builder::new().name("keysor-event-tap".into()).spawn(|| unsafe {
            let event_mask = (1u64 << K_CG_EVENT_KEY_DOWN)
                | (1u64 << K_CG_EVENT_KEY_UP)
                | (1u64 << K_CG_EVENT_FLAGS_CHANGED);
            let mut prompt_opened = false;
            while !STOP_REQUESTED.load(Ordering::SeqCst) {
                if !AXIsProcessTrusted() {
                    HOOK_STATUS.store(1, Ordering::SeqCst);
                    if !prompt_opened {
                        prompt_opened = true;
                        let _ = std::process::Command::new("open")
                            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
                            .spawn();
                    }
                    std::thread::sleep(Duration::from_secs(1));
                    continue;
                }
                prompt_opened = false;
                HOOK_STATUS.store(3, Ordering::SeqCst);
                let mut port = CGEventTapCreate(K_CG_SESSION_EVENT_TAP,
                    K_CG_HEAD_INSERT_EVENT_TAP, K_CG_EVENT_TAP_OPTION_DEFAULT,
                    event_mask, event_tap_callback, std::ptr::null_mut());
                if port.is_null() {
                    port = CGEventTapCreate(K_CG_HID_EVENT_TAP,
                        K_CG_HEAD_INSERT_EVENT_TAP, K_CG_EVENT_TAP_OPTION_DEFAULT,
                        event_mask, event_tap_callback, std::ptr::null_mut());
                }
                if port.is_null() {
                    HOOK_STATUS.store(4, Ordering::SeqCst);
                    eprintln!("[Keysor] Event tap creation failed; retrying.");
                    std::thread::sleep(Duration::from_secs(1));
                    continue;
                }
                let source = CFMachPortCreateRunLoopSource(std::ptr::null_mut(), port, 0);
                if source.is_null() {
                    HOOK_STATUS.store(4, Ordering::SeqCst);
                    eprintln!("[Keysor] Event tap run loop source creation failed; retrying.");
                    CFRelease(port);
                    std::thread::sleep(Duration::from_secs(1));
                    continue;
                }
                let run_loop = CFRunLoopGetCurrent();
                LAST_CAPS_FLAG.with(|last| last.set(Some(CGEventSourceFlagsState(0) & 0x10000 != 0)));
                EVENT_TAP_PORT.store(port, Ordering::SeqCst);
                RUN_LOOP_REF.store(run_loop, Ordering::SeqCst);
                CFRunLoopAddSource(run_loop, source, kCFRunLoopCommonModes);
                if !STOP_REQUESTED.load(Ordering::SeqCst) {
                    HOOK_STATUS.store(2, Ordering::SeqCst);
                    CFRunLoopRun();
                }
                HOOK_STATUS.store(3, Ordering::SeqCst);
                RUN_LOOP_REF.store(std::ptr::null_mut(), Ordering::SeqCst);
                EVENT_TAP_PORT.store(std::ptr::null_mut(), Ordering::SeqCst);
                crate::hook::cancel_active_input();
                CGEventTapEnable(port, false);
                CFRunLoopRemoveSource(run_loop, source, kCFRunLoopCommonModes);
                CFRelease(source);
                CFRelease(port);
                if !STOP_REQUESTED.load(Ordering::SeqCst) {
                    std::thread::sleep(Duration::from_secs(1));
                }
            }
            HOOK_STATUS.store(0, Ordering::SeqCst);
        }).map_err(|e| {
            *GLOBAL_CALLBACK.lock().unwrap() = None;
            format!("Could not start macOS event tap thread: {e}")
        })?;
        Ok(())
    }

    fn stop_listening(&self) {
        STOP_REQUESTED.store(true, Ordering::SeqCst);
        let run_loop = RUN_LOOP_REF.load(Ordering::SeqCst);
        if !run_loop.is_null() { unsafe { CFRunLoopStop(run_loop); } }
        *GLOBAL_CALLBACK.lock().unwrap() = None;
    }

    fn modifier_sync_guard(&self, is_mouse_mode: bool, is_toggle_mode: bool, on_deactivate: fn()) {
        if !is_mouse_mode || is_toggle_mode { return; }
        let configured_vk = crate::hook::APP_STATE.get()
            .and_then(|state| state.lock().ok())
            .and_then(|state| crate::config::key_name_to_vk(&state.config.settings.modifier_key));
        if configured_vk.and_then(configured_key_is_down) == Some(false) {
            on_deactivate();
        }
    }
}
