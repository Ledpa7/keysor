#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
#[path = "../watchdog_core.rs"]
mod watchdog_core;

#[cfg(windows)]
fn main() {
    use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_ICONINFORMATION, MB_OK};
    // Independent of Keysor, its configuration, hooks, licence and Guardian.
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let report = match args.as_slice() {
        [] => None,
        [flag, path] if flag == "--report" => Some(std::path::PathBuf::from(path)),
        _ => std::process::exit(2),
    };
    let mut restored = false;
    for _ in 0..3 {
        restored |= watchdog_core::restore_cursor();
        std::thread::sleep(std::time::Duration::from_millis(30));
    }
    if let Some(path) = report {
        let data = serde_json::json!({"restore_ok":restored,"process_id":std::process::id()});
        if std::fs::write(path,data.to_string()).is_err() { std::process::exit(2); }
    } else {
        let text = if restored {
            "Windows 기본 커서를 복구했습니다. 키서가 실행 중이면 종료해 주세요."
        } else {
            "기본 커서를 복구하지 못했습니다. Windows 마우스 설정에서 포인터 기본값을 복원해 주세요."
        };
        let text: Vec<_> = text.encode_utf16().chain(Some(0)).collect();
        let title: Vec<_> = "Keysor Cursor Recovery".encode_utf16().chain(Some(0)).collect();
        unsafe { MessageBoxW(0,text.as_ptr(),title.as_ptr(),MB_OK | if restored {MB_ICONINFORMATION} else {MB_ICONERROR}); }
    }
    std::process::exit(if restored {0} else {1});
}

#[cfg(not(windows))]
fn main() {}
