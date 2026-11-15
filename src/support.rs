use std::sync::atomic::{AtomicIsize, Ordering};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

static WINDOW: AtomicIsize = AtomicIsize::new(0);
static OPENING: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
pub(crate) fn window_for_test() -> HWND { WINDOW.load(Ordering::SeqCst) }

fn wide(text: &str) -> Vec<u16> { text.encode_utf16().chain(Some(0)).collect() }

fn private_path(path: &std::path::Path) -> String {
    let mut text = path.to_string_lossy().into_owned();
    for (variable, label) in [("USERPROFILE", "%USERPROFILE%"), ("ProgramFiles", "%ProgramFiles%")] {
        if let Some(base) = std::env::var_os(variable) {
            text = text.replace(base.to_string_lossy().as_ref(), label);
        }
    }
    text
}

pub fn report_text() -> String {
    let executable = std::env::current_exe().ok();
    let guardian = executable.as_ref().map(|p| p.with_file_name("keysor-guardian.exe"));
    let mode = crate::hook::APP_STATE.get().and_then(|s| s.try_lock().ok())
        .map(|s| if s.is_mouse_mode { "on" } else { "off" }).unwrap_or("not available");
    let report_path = crate::config::get_config_path()
        .with_file_name(format!("watchdog-{}.json", std::process::id()));
    let watchdog = std::fs::read_to_string(report_path).ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok());
    let ready = watchdog.as_ref().and_then(|v| v["ready"].as_bool());
    format!("Keysor {}\r\nBuild: {} ({})\r\nOS: {} / {}\r\nProcess: {}\r\nExecutable: {}\r\nSettings: {}\r\nGuardian present: {}\r\nGuardian ready for this process: {}\r\nMouse mode: {}\r\nChrome Remote Desktop compatibility: {}\r\n\r\nThis report contains no license keys, typed text, window titles or URLs.\r\nReview before sharing. Nothing is sent automatically.",
        env!("CARGO_PKG_VERSION"), env!("KEYSOR_BUILD_ID"), env!("KEYSOR_BUILD_MODE"),
        std::env::consts::OS, std::env::consts::ARCH, std::process::id(),
        executable.as_deref().map(private_path).unwrap_or_else(|| "unknown".into()),
        private_path(&crate::config::get_config_path()), guardian.is_some_and(|p| p.is_file()), match ready { Some(true) => "yes", Some(false) => "no", None => "unknown" }, mode,
        std::env::args().any(|arg| arg == "--chrome-remote"))
}

// Clipboard ownership transfers to Windows only after SetClipboardData succeeds.
fn copy_text(hwnd: HWND, text: &str) -> bool {
    #[link(name = "user32")]
    unsafe extern "system" {
        fn OpenClipboard(owner: HWND) -> i32;
        fn EmptyClipboard() -> i32;
        fn SetClipboardData(format: u32, memory: isize) -> isize;
        fn CloseClipboard() -> i32;
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GlobalAlloc(flags: u32, bytes: usize) -> isize;
        fn GlobalLock(memory: isize) -> *mut u16;
        fn GlobalUnlock(memory: isize) -> i32;
        fn GlobalFree(memory: isize) -> isize;
    }
    let text = wide(text);
    unsafe {
        let memory = GlobalAlloc(0x2, text.len() * 2);
        if memory == 0 { return false; }
        let target = GlobalLock(memory);
        if target.is_null() { GlobalFree(memory); return false; }
        std::ptr::copy_nonoverlapping(text.as_ptr(), target, text.len());
        GlobalUnlock(memory);
        if OpenClipboard(hwnd) == 0 { GlobalFree(memory); return false; }
        let ok = EmptyClipboard() != 0 && SetClipboardData(13, memory) != 0;
        CloseClipboard();
        if !ok { GlobalFree(memory); }
        ok
    }
}

unsafe extern "system" fn window_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        match msg {
            WM_COMMAND => {
                match w & 0xffff {
                    101 => {
                        let edit = GetDlgItem(hwnd, 201);
                        let mut text = vec![0u16; GetWindowTextLengthW(edit) as usize + 1];
                        let count = GetWindowTextW(edit, text.as_mut_ptr(), text.len() as i32);
                        let ok = copy_text(hwnd, &String::from_utf16_lossy(&text[..count as usize]));
                        SetWindowTextW(GetDlgItem(hwnd, 204), wide(if ok { "Copied / 복사 완료" } else { "Clipboard busy. Try again. / 다시 시도해 주세요." }).as_ptr());
                    }
                    102 => crate::update::check_for_updates_manually(),
                    103 => { DestroyWindow(hwnd); }
                    _ => {}
                }
                0
            }
            WM_CLOSE => { DestroyWindow(hwnd); 0 }
            WM_DESTROY => { WINDOW.store(0, Ordering::SeqCst); PostQuitMessage(0); 0 }
            _ => DefWindowProcW(hwnd, msg, w, l),
        }
    }
}

pub fn show(lang_en: bool) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        // Keep one message loop and one support window per process.
        let Ok(_guard) = OPENING.try_lock() else {
            let hwnd = WINDOW.load(Ordering::SeqCst);
            if hwnd != 0 { unsafe { ShowWindow(hwnd, SW_RESTORE); SetForegroundWindow(hwnd); } }
            return;
        };
        unsafe {
            let instance = windows_sys::Win32::System::LibraryLoader::GetModuleHandleW(std::ptr::null());
            let class = wide("KeysorSupportClass");
            let mut wc: WNDCLASSW = std::mem::zeroed();
            wc.lpfnWndProc = Some(window_proc);
            wc.hInstance = instance;
            wc.lpszClassName = class.as_ptr();
            wc.hCursor = LoadCursorW(0, IDC_ARROW);
            wc.hbrBackground = (windows_sys::Win32::Graphics::Gdi::COLOR_WINDOW + 1) as isize;
            RegisterClassW(&wc);
            let scale = crate::platform::get_system_controller().get_dpi_scale();
            let px = |v: i32| (v as f64 * scale).round() as i32;
            let hwnd = CreateWindowExW(WS_EX_APPWINDOW, class.as_ptr(),
                wide(if lang_en { "Keysor — Info / Support" } else { "Keysor — 정보 / 지원" }).as_ptr(),
                WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX,
                CW_USEDEFAULT, CW_USEDEFAULT, px(680), px(455), 0, 0, instance, std::ptr::null());
            if hwnd == 0 { return; }
            WINDOW.store(hwnd, Ordering::SeqCst);
            let control = |class: &str, text: &str, style: u32, x, y, width, height, id| {
                let child = CreateWindowExW(0, wide(class).as_ptr(), wide(text).as_ptr(),
                    WS_CHILD | WS_VISIBLE | style, px(x), px(y), px(width), px(height), hwnd, id, instance, std::ptr::null());
                // WM_SETFONT / DEFAULT_GUI_FONT
                SendMessageW(child, 0x30, windows_sys::Win32::Graphics::Gdi::GetStockObject(17) as usize, 1);
                child
            };
            let title = format!("Keysor v{}  ·  Build {}", env!("CARGO_PKG_VERSION"), env!("KEYSOR_BUILD_ID"));
            control("STATIC", &title, 0, 20, 15, 630, 24, 200);
            control("EDIT", &report_text(), WS_BORDER | WS_VSCROLL | ES_MULTILINE as u32 | ES_READONLY as u32 | ES_AUTOVSCROLL as u32, 20, 48, 625, 270, 201);
            control("STATIC", if lang_en { "Review before sharing. No information is sent automatically." } else { "공유 전 내용을 확인하세요. 정보는 자동 전송되지 않습니다." }, 0, 20, 328, 625, 22, 204);
            control("BUTTON", if lang_en { "Copy diagnostics" } else { "진단 정보 복사" }, WS_TABSTOP, 20, 365, 190, 32, 101);
            control("BUTTON", if lang_en { "Check for updates" } else { "업데이트 확인" }, WS_TABSTOP, 220, 365, 190, 32, 102);
            control("BUTTON", if lang_en { "Close" } else { "닫기" }, WS_TABSTOP, 520, 365, 125, 32, 103);
            // Explicit user action must override a launcher STARTUPINFO/SW_HIDE hint.
            SetWindowPos(hwnd, HWND_TOP, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW);
            SetForegroundWindow(hwnd);
            let mut msg: MSG = std::mem::zeroed();
            while GetMessageW(&mut msg, 0, 0, 0) > 0 {
                if IsDialogMessageW(hwnd, &msg) == 0 { TranslateMessage(&msg); DispatchMessageW(&msg); }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn support_report_has_build_identity_without_loading_license_or_input_logs() {
        let report = report_text();
        assert!(report.contains(env!("CARGO_PKG_VERSION")));
        assert!(report.contains(env!("KEYSOR_BUILD_ID")));
        assert!(report.contains("Guardian ready for this process:"));
        if let Ok(profile) = std::env::var("USERPROFILE") {
            assert!(!report.contains(&profile));
        }
    }
    #[test]
    #[ignore = "Requires the interactive Windows desktop; copies the support report to the clipboard"]
    fn native_support_controls_copy_exact_report() {
        #[link(name = "user32")]
        unsafe extern "system" {
            fn OpenClipboard(owner: HWND) -> i32;
            fn GetClipboardData(format: u32) -> isize;
            fn CloseClipboard() -> i32;
        }
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn GlobalLock(memory: isize) -> *const u16;
            fn GlobalUnlock(memory: isize) -> i32;
        }
        let thread = show(true);
        let mut hwnd = 0;
        for _ in 0..100 {
            hwnd = WINDOW.load(Ordering::SeqCst);
            if hwnd != 0 && unsafe { GetDlgItem(hwnd, 103) } != 0 { break; }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert_ne!(hwnd, 0, "Support window was not created");
        let read = |control| unsafe {
            let mut text = vec![0u16; GetWindowTextLengthW(control) as usize + 1];
            let count = GetWindowTextW(control, text.as_mut_ptr(), text.len() as i32);
            String::from_utf16_lossy(&text[..count as usize])
        };
        unsafe {
            let expected = read(GetDlgItem(hwnd, 201));
            assert!(expected.contains(env!("KEYSOR_BUILD_ID")));
            let mut client = std::mem::zeroed();
            assert_ne!(GetClientRect(hwnd, &mut client), 0);
            for id in [200, 201, 204, 101, 102, 103] {
                let child = GetDlgItem(hwnd, id);
                assert_ne!(child, 0);
                let mut rect: windows_sys::Win32::Foundation::RECT = std::mem::zeroed();
                assert_ne!(GetWindowRect(child, &mut rect), 0);
                windows_sys::Win32::Graphics::Gdi::MapWindowPoints(0, hwnd, &mut rect as *mut _ as *mut _, 2);
                assert!(rect.left >= 0 && rect.top >= 0 && rect.right <= client.right && rect.bottom <= client.bottom,
                    "Control {id} is clipped by the client area");
            }
            // The real BUTTON control emits WM_COMMAND to the production window proc.
            SendMessageW(GetDlgItem(hwnd, 101), BM_CLICK, 0, 0);
            assert!(read(GetDlgItem(hwnd, 204)).contains("Copied"));
            assert_ne!(OpenClipboard(hwnd), 0);
            let memory = GetClipboardData(13);
            assert_ne!(memory, 0);
            let data = GlobalLock(memory);
            assert!(!data.is_null());
            let expected_wide: Vec<u16> = expected.encode_utf16().chain(Some(0)).collect();
            assert_eq!(std::slice::from_raw_parts(data, expected_wide.len()), expected_wide);
            GlobalUnlock(memory);
            CloseClipboard();
            SendMessageW(hwnd, WM_CLOSE, 0, 0);
        }
        thread.join().unwrap();
    }

}
