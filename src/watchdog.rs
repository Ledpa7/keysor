use std::fs;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::Duration;

use serde_json::json;
use windows_sys::Win32::System::Threading::{OpenProcess, WaitForSingleObject};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{mouse_event, MOUSEEVENTF_MOVE};
use windows_sys::Win32::UI::WindowsAndMessaging::{SystemParametersInfoW, SPI_SETCURSORS};

const SYNCHRONIZE: u32 = 0x0010_0000;
const INFINITE: u32 = 0xFFFF_FFFF;
const DETACHED_PROCESS: u32 = 0x0000_0008;
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;

fn guardian_binary(main_exe: &Path) -> PathBuf {
    main_exe.with_file_name("keysor-guardian.exe")
}

pub fn restore_cursor() -> bool {
    unsafe {
        let ok = SystemParametersInfoW(SPI_SETCURSORS, 0, std::ptr::null_mut(), 0) != 0;
        mouse_event(MOUSEEVENTF_MOVE, 0, 0, 0, 0);
        ok
    }
}

pub fn run(parent_pid: u32, report_path: &Path) {
    unsafe {
        let parent = OpenProcess(SYNCHRONIZE, 0, parent_pid);
        let ready = parent != 0;
        let guardian_pid = std::process::id();
        let _ = fs::write(report_path, json!({
            "parent_pid": parent_pid, "guardian_pid": guardian_pid, "ready": ready
        }).to_string());

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
        let _ = fs::write(report_path, json!({
            "parent_pid": parent_pid, "guardian_pid": guardian_pid,
            "ready": ready, "wait_result": wait_result,
            "restore_ok": restored
        }).to_string());
    }
}

fn start_and_wait(program: &Path, parent_pid: u32, report: &Path) -> (Option<u32>, bool, Option<String>) {
    let _ = fs::remove_file(&report);
    let started = Command::new(program)
        .args(["--watchdog", &parent_pid.to_string(), &report.to_string_lossy()])
        .creation_flags(DETACHED_PROCESS | CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP)
        .spawn();
    let guardian_pid = started.as_ref().map(|child| child.id()).ok();
    let spawn_error = started.err().map(|error| error.to_string());
    let mut ready = false;
    if guardian_pid.is_some() {
        for _ in 0..20 {
            if let Ok(contents) = fs::read_to_string(&report) {
                ready = serde_json::from_str::<serde_json::Value>(&contents)
                    .ok().and_then(|value| value["ready"].as_bool()).unwrap_or(false);
                if ready { break; }
            }
            thread::sleep(Duration::from_millis(25));
        }
    }
    (guardian_pid, ready, spawn_error)
}

pub fn spawn() {
    let Ok(main_exe) = std::env::current_exe() else { return; };
    let parent_pid = std::process::id();
    let guardian = guardian_binary(&main_exe);
    let independent = guardian.is_file();
    let program = if independent { &guardian } else { &main_exe };
    let report = crate::config::get_config_path()
        .with_file_name(format!("watchdog-{}.json", parent_pid));
    let (guardian_pid, ready, spawn_error) = start_and_wait(program, parent_pid, &report);

    let (fallback_pid, fallback_ready) = if independent && !ready {
        let fallback_report = report.with_file_name(format!("watchdog-fallback-{}.json", parent_pid));
        let (pid, ready, _) = start_and_wait(&main_exe, parent_pid, &fallback_report);
        (pid, ready)
    } else {
        (None, false)
    };

    let startup_report = report.with_file_name("watchdog-startup.json");
    let _ = fs::write(startup_report, json!({
        "parent_pid": parent_pid, "guardian_pid": guardian_pid,
        "independent": independent, "ready": ready,
        "spawn_error": spawn_error,
        "fallback_pid": fallback_pid, "fallback_ready": fallback_ready
    }).to_string());
    if !ready && !fallback_ready {
        eprintln!("[Watchdog] Guardian did not become ready");
    }
}

#[cfg(test)]
mod tests {
    use super::guardian_binary;
    use std::path::Path;

    #[test]
    fn guardian_has_distinct_process_name() {
        assert_eq!(guardian_binary(Path::new(r"C:\Program Files\Keysor\keysor.exe")),
            Path::new(r"C:\Program Files\Keysor\keysor-guardian.exe"));
    }
}
