use std::sync::OnceLock;
use std::thread;
use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, VK_CAPITAL,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage,
    UnhookWindowsHookEx, MSG, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN,
    WM_SYSKEYUP,
};
use crate::platform::{KeyboardHook, KeyEvent, HookResult};

const KEYSOR_SIGNATURE: usize = 0xFA17CAFE;
const CRD_SIGNATURE: usize = 0x43524430;
static CHROME_REMOTE_MODE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn chrome_remote_mode() -> bool {
    CHROME_REMOTE_MODE.load(std::sync::atomic::Ordering::Relaxed)
}

pub fn set_chrome_remote_mode(enabled: bool) {
    CHROME_REMOTE_MODE.store(enabled, std::sync::atomic::Ordering::Relaxed);
}

fn is_remote_lock_sync(vk: u32, scan: u32, flags: u32, extra: usize, remote_mode: bool) -> bool {
    // CRD sends lock-state synchronization as virtual-key events (scan=0),
    // whereas actual remote key presses use hardware scan codes.
    vk == VK_CAPITAL as u32 && scan == 0 && flags & 0x10 != 0
        && extra != KEYSOR_SIGNATURE && (remote_mode || extra == CRD_SIGNATURE)
}

static HOOK_CALLBACK: OnceLock<Box<dyn Fn(KeyEvent) -> HookResult + Send + Sync + 'static>> = OnceLock::new();
static H_HOOK: OnceLock<std::sync::Mutex<isize>> = OnceLock::new();

#[derive(Copy, Clone)]
#[repr(C)]
struct KBD_STRUCT_MINIMAL {
    vk_code: u32,
    scan_code: u32,
    flags: u32,
    time: u32,
    dw_extra_info: usize,
}

pub struct WindowsKeyboardHook;

impl WindowsKeyboardHook {
    pub fn new() -> Self {
        WindowsKeyboardHook
    }
}

unsafe extern "system" fn low_level_keyboard_proc(
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if code >= 0 {
        let kbd_struct = unsafe { *(lparam as *const KBD_STRUCT_MINIMAL) };
        let vk_code = kbd_struct.vk_code;
        if is_remote_lock_sync(vk_code, kbd_struct.scan_code, kbd_struct.flags,
            kbd_struct.dw_extra_info, CHROME_REMOTE_MODE.load(std::sync::atomic::Ordering::Relaxed)) {
            crate::diagnostics::record_remote_lock_sync();
            // CRD can send a virtual CapsLock transition after the physical key.
            let owns_caps_lock = crate::hook::APP_STATE.get()
                .and_then(|state| state.try_lock().ok()
                    .map(|state| crate::hook::uses_native_caps_lock(&state.config)))
                .unwrap_or(true);
            if owns_caps_lock {
                return 1;
            }
            return unsafe { CallNextHookEx(0, code, wparam, lparam) };
        }
        let is_keyup = wparam == WM_KEYUP as usize || wparam == WM_SYSKEYUP as usize;
        let is_keydown = wparam == WM_KEYDOWN as usize || wparam == WM_SYSKEYDOWN as usize;

        #[cfg(debug_assertions)]
        println!("[Hook Debug] low_level_keyboard_proc: vk_code={}, is_keydown={}, is_keyup={}", vk_code, is_keydown, is_keyup);

        let is_injected = kbd_struct.dw_extra_info == KEYSOR_SIGNATURE;

        let event = KeyEvent {
            vk_code,
            is_keydown,
            is_keyup,
            is_injected_by_keysor: is_injected,
        };

        if let Some(cb) = HOOK_CALLBACK.get() {
            let result = cb(event);
            crate::diagnostics::record_transition(vk_code, is_keydown, is_keyup,
                is_injected, kbd_struct.flags & 0x10 != 0,
                kbd_struct.dw_extra_info == CRD_SIGNATURE, matches!(result, HookResult::Pass));
            match result {
                HookResult::Block => {
                    #[cfg(debug_assertions)]
                    println!("[Hook Debug] HookResult::Block returned for vk_code={}", vk_code);
                    return 1;
                }
                HookResult::Pass => {
                    #[cfg(debug_assertions)]
                    println!("[Hook Debug] HookResult::Pass returned for vk_code={}", vk_code);
                }
            }
        } else {
            #[cfg(debug_assertions)]
            println!("[Hook Debug] HOOK_CALLBACK is None!");
        }
    }
    unsafe { CallNextHookEx(0, code, wparam, lparam) }
}

#[cfg(test)]
mod remote_tests {
    use super::*;
    #[test]
    fn remote_sync_is_blocked_before_reaching_windows() {
        set_chrome_remote_mode(true);
        let event = KBD_STRUCT_MINIMAL {
            vk_code: VK_CAPITAL as u32,
            scan_code: 0,
            flags: 0x10,
            time: 0,
            dw_extra_info: CRD_SIGNATURE,
        };
        let result = unsafe {
            low_level_keyboard_proc(0, WM_KEYDOWN as usize, &event as *const _ as LPARAM)
        };
        set_chrome_remote_mode(false);
        assert_eq!(result, 1);
    }
    #[test]
    fn remote_sync_does_not_toggle_mouse_mode() {
        assert!(is_remote_lock_sync(0x14, 0, 0x10, CRD_SIGNATURE, false));
        assert!(is_remote_lock_sync(0x14, 0, 0x90, 0, true));
        assert!(!is_remote_lock_sync(0x14, 0x3a, 0x10, CRD_SIGNATURE, true));
        assert!(!is_remote_lock_sync(0x14, 0, 0x10, 0, false));
        assert!(!is_remote_lock_sync(0x14, 0, 0x10, KEYSOR_SIGNATURE, true));
        assert!(!is_remote_lock_sync(0x14, 0x3a, 0, 0, true));
        assert!(!is_remote_lock_sync(0x57, 0, 0x10, CRD_SIGNATURE, true));
    }
}

impl KeyboardHook for WindowsKeyboardHook {
    fn start_listening(
        &self,
        callback: Box<dyn Fn(KeyEvent) -> HookResult + Send + Sync + 'static>,
    ) -> Result<(), String> {
        HOOK_CALLBACK.set(callback)
            .map_err(|_| "KeyboardHook callback has already been registered".to_string())?;

        let (ready_tx, ready_rx) = std::sync::mpsc::sync_channel(1);
        thread::spawn(move || unsafe {
            let instance = windows_sys::Win32::System::LibraryLoader::GetModuleHandleW(std::ptr::null());
            let hook = SetWindowsHookExW(
                WH_KEYBOARD_LL,
                Some(low_level_keyboard_proc),
                instance,
                0,
            );

            if hook == 0 {
                let _ = ready_tx.send(Err(format!("WH_KEYBOARD_LL installation failed: {}", std::io::Error::last_os_error())));
                return;
            }

            H_HOOK.set(std::sync::Mutex::new(hook)).ok();
            let _ = ready_tx.send(Ok(()));
            #[cfg(debug_assertions)]
            println!("[Hook Debug] SetWindowsHookExW success. hook={}", hook);

            let mut msg: MSG = std::mem::zeroed();
            while GetMessageW(&mut msg, 0, 0, 0) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        });

        ready_rx.recv().map_err(|_| "Keyboard hook thread stopped during startup".to_string())?
    }

    fn stop_listening(&self) {
        if let Some(hook_mutex) = H_HOOK.get() {
            let hook = *hook_mutex.lock().unwrap();
            if hook != 0 {
                unsafe {
                    UnhookWindowsHookEx(hook);
                }
            }
        }
    }

    fn modifier_sync_guard(&self, is_mouse_mode: bool, is_toggle_mode: bool, on_deactivate: fn()) {
        if is_mouse_mode && !is_toggle_mode {
            unsafe {
                let modifier = crate::hook::APP_STATE.get().and_then(|s| {
                    s.lock().ok().and_then(|s| crate::config::key_name_to_vk(&s.config.settings.modifier_key))
                }).unwrap_or(VK_CAPITAL as u32);
                let phys_state = GetAsyncKeyState(modifier as i32);
                if (phys_state as u16 & 0x8000) == 0 {
                    on_deactivate();
                }
            }
        }
    }
}
