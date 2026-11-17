use std::fs;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::Duration;

use serde_json::json;

const DETACHED_PROCESS: u32 = 0x0000_0008;
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;

pub(crate) fn guardian_alive(pid: u32) -> bool {
    use windows_sys::Win32::System::Threading::{OpenProcess, WaitForSingleObject};
    unsafe {
        let handle = OpenProcess(0x0010_0000, 0, pid);
        if handle == 0 { return false; }
        let alive = WaitForSingleObject(handle, 0) == 258; // WAIT_TIMEOUT
        windows_sys::Win32::Foundation::CloseHandle(handle);
        alive
    }
}

pub(crate) fn monitor_guardian(pid: u32) {
    use windows_sys::Win32::System::Threading::{OpenProcess, WaitForSingleObject};
    // Wait on the kernel handle, without adding a polling loop.
    let handle = unsafe { OpenProcess(0x0010_0000, 0, pid) };
    if handle == 0 {
        crate::cursor_scheme::restore();
        return;
    }
    thread::spawn(move || unsafe {
        WaitForSingleObject(handle, 0xffff_ffff);
        windows_sys::Win32::Foundation::CloseHandle(handle);
        crate::cursor_scheme::restore();
    });
}

fn guardian_binary(main_exe: &Path) -> PathBuf {
    main_exe.with_file_name("keysor-guardian.exe")
}

pub fn restore_cursor() -> bool {
    crate::watchdog_core::restore_cursor()
}

fn start_and_wait(
    program: &Path,
    parent_pid: u32,
    report: &Path,
) -> (Option<u32>, bool, Option<String>) {
    let _ = fs::remove_file(report);
    let started = Command::new(program)
        .args([
            "--watchdog",
            &parent_pid.to_string(),
            &report.to_string_lossy(),
        ])
        .creation_flags(DETACHED_PROCESS | CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP)
        .spawn();
    let guardian_pid = started.as_ref().map(|child| child.id()).ok();
    let spawn_error = started.err().map(|error| error.to_string());
    let mut ready = false;
    if guardian_pid.is_some() {
        for _ in 0..80 {
            if let Ok(contents) = fs::read_to_string(report) {
                ready = serde_json::from_str::<serde_json::Value>(&contents)
                    .ok()
                    .map(|value| value["ready"] == true
                        && value["parent_pid"].as_u64() == Some(parent_pid as u64)
                        && value["guardian_pid"].as_u64() == guardian_pid.map(u64::from))
                    .unwrap_or(false);
                if ready {
                    break;
                }
            }
            thread::sleep(Duration::from_millis(25));
        }
    }
    ready &= guardian_pid.is_some_and(guardian_alive);
    (guardian_pid, ready, spawn_error)
}

pub fn spawn() {
    let Ok(main_exe) = std::env::current_exe() else {
        return;
    };
    let parent_pid = std::process::id();
    let guardian = guardian_binary(&main_exe);
    let companion_present = guardian.is_file();
    let report =
        crate::config::get_config_path().with_file_name(format!("watchdog-{}.json", parent_pid));
    // Never reuse the main image as its own recovery program: file blocking
    // and process-name termination would affect both copies alike.
    let (guardian_pid, ready, spawn_error) = if companion_present {
        start_and_wait(&guardian, parent_pid, &report)
    } else {
        (None, false, Some("Independent keysor-guardian.exe is missing".to_owned()))
    };
    if ready {
        monitor_guardian(guardian_pid.unwrap());
    } else {
        let _ = fs::remove_file(&report);
        restore_cursor();
    }

    let startup_report = report.with_file_name("watchdog-startup.json");
    let _ = fs::write(
        startup_report,
        json!({
            "parent_pid": parent_pid, "guardian_pid": guardian_pid,
            "independent": companion_present && ready, "ready": ready,
            "spawn_error": spawn_error,
            "fallback_pid": null, "fallback_ready": false
        })
        .to_string(),
    );
    if !ready {
        eprintln!("[Watchdog] Guardian did not become ready");
    }
}

#[cfg(test)]
mod tests {
    use super::guardian_binary;
    use std::path::Path;

    #[test]
    fn guardian_has_distinct_process_name() {
        assert_eq!(
            guardian_binary(Path::new(r"C:\Program Files\Keysor\keysor.exe")),
            Path::new(r"C:\Program Files\Keysor\keysor-guardian.exe")
        );
    }
}
