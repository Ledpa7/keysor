use std::fs;
use std::path::Path;
use std::thread;
use std::time::Duration;

use serde_json::json;
use windows_sys::Win32::System::Threading::{OpenProcess, WaitForSingleObject};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{MOUSEEVENTF_MOVE, mouse_event};
use windows_sys::Win32::UI::WindowsAndMessaging::{SPI_SETCURSORS, SystemParametersInfoW};

const SYNCHRONIZE: u32 = 0x0010_0000;
const INFINITE: u32 = 0xFFFF_FFFF;

pub fn restore_cursor() -> bool {
    unsafe {
        let restored = SystemParametersInfoW(SPI_SETCURSORS, 0, std::ptr::null_mut(), 0) != 0;
        mouse_event(MOUSEEVENTF_MOVE, 0, 0, 0, 0);
        restored
    }
}

pub fn run(parent_pid: u32, report_path: &Path) {
    unsafe {
        let parent = OpenProcess(SYNCHRONIZE, 0, parent_pid);
        let ready = parent != 0;
        let guardian_pid = std::process::id();
        let _ = fs::write(
            report_path,
            json!({
                "parent_pid": parent_pid, "guardian_pid": guardian_pid, "ready": ready
            })
            .to_string(),
        );

        let wait_result = if ready {
            let result = WaitForSingleObject(parent, INFINITE);
            windows_sys::Win32::Foundation::CloseHandle(parent);
            Some(result)
        } else {
            None
        };

        let mut restored = false;
        for _ in 0..3 {
            restored |= restore_cursor();
            thread::sleep(Duration::from_millis(30));
        }
        let _ = fs::write(
            report_path,
            json!({
                "parent_pid": parent_pid, "guardian_pid": guardian_pid,
                "ready": ready, "wait_result": wait_result, "restore_ok": restored
            })
            .to_string(),
        );
    }
}
