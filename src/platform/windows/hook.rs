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
        let is_keyup = wparam == WM_KEYUP as usize || wparam == WM_SYSKEYUP as usize;
        let is_keydown = wparam == WM_KEYDOWN as usize || wparam == WM_SYSKEYDOWN as usize;

        // 콘솔 디버그 로그 추가
        println!("[Hook Debug] low_level_keyboard_proc: vk_code={}, is_keydown={}, is_keyup={}", vk_code, is_keydown, is_keyup);

        let is_injected = kbd_struct.dw_extra_info == KEYSOR_SIGNATURE;

        let event = KeyEvent {
            vk_code,
            is_keydown,
            is_keyup,
            is_injected_by_keysor: is_injected,
        };

        if let Some(cb) = HOOK_CALLBACK.get() {
            match cb(event) {
                HookResult::Block => {
                    println!("[Hook Debug] HookResult::Block returned for vk_code={}", vk_code);
                    return 1;
                }
                HookResult::Pass => {
                    println!("[Hook Debug] HookResult::Pass returned for vk_code={}", vk_code);
                }
            }
        } else {
            println!("[Hook Debug] HOOK_CALLBACK is None!");
        }
    }
    unsafe { CallNextHookEx(0, code, wparam, lparam) }
}

impl KeyboardHook for WindowsKeyboardHook {
    fn start_listening(
        &self,
        callback: Box<dyn Fn(KeyEvent) -> HookResult + Send + Sync + 'static>,
    ) -> Result<(), String> {
        HOOK_CALLBACK.set(callback)
            .map_err(|_| "KeyboardHook callback has already been registered".to_string())?;

        thread::spawn(move || unsafe {
            let instance = windows_sys::Win32::System::LibraryLoader::GetModuleHandleW(std::ptr::null());
            let hook = SetWindowsHookExW(
                WH_KEYBOARD_LL,
                Some(low_level_keyboard_proc),
                instance,
                0,
            );

            if hook == 0 {
                eprintln!("[Error] Failed to install WH_KEYBOARD_LL low-level keyboard hook.");
                return;
            }

            H_HOOK.set(std::sync::Mutex::new(hook)).ok();
            println!("[Hook Debug] SetWindowsHookExW success. hook={}", hook);

            let mut msg: MSG = std::mem::zeroed();
            while GetMessageW(&mut msg, 0, 0, 0) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        });

        Ok(())
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
                let phys_state = GetAsyncKeyState(VK_CAPITAL as i32);
                if (phys_state as u16 & 0x8000) == 0 {
                    on_deactivate();
                }
            }
        }
    }
}
