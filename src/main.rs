#![windows_subsystem = "windows"]

mod config;
mod math;
mod hook;
mod platform;
mod license;
mod update;

mod indicator;
mod ui;

use std::fs;
use std::path::Path;
use std::thread;
use std::time::{Duration, SystemTime};

#[cfg(windows)]
fn encode_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(target_os = "macos")]
fn ensure_single_instance() -> bool {
    use std::os::unix::io::AsRawFd;
    unsafe extern "C" {
        fn flock(fd: i32, op: i32) -> i32;
    }

    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let lock_dir = std::path::PathBuf::from(home).join(".keysor");
    let _ = fs::create_dir_all(&lock_dir);
    let lock_path = lock_dir.join("keysor.lock");

    let file = match fs::OpenOptions::new().read(true).write(true).create(true).truncate(false).open(&lock_path) {
        Ok(f) => f,
        Err(_) => return true,
    };

    let fd = file.as_raw_fd();
    // LOCK_EX (2) | LOCK_NB (4) = 6
    let res = unsafe { flock(fd, 2 | 4) };
    if res != 0 {
        println!("[SingleInstance] Another Keysor process is already running. Requesting HUD popup.");
        unsafe {
            unsafe extern "C" {
                fn keysor_macos_post_show_hud_notification();
            }
            keysor_macos_post_show_hud_notification();
        }
        return false;
    }
    Box::leak(Box::new(file));
    true
}

#[cfg(target_os = "windows")]
fn check_toggle_and_single_instance() -> bool {
    // 1. Ctrl + Alt + K 종료 직후 Windows 쉘 바로가기가 프로세스를 재시작시키는 것을 억제
    let flag_path = std::env::temp_dir().join("keysor_toggle_off.flag");
    if let Ok(metadata) = std::fs::metadata(&flag_path) {
        if let Ok(modified) = metadata.modified() {
            if let Ok(elapsed) = modified.elapsed() {
                if elapsed < std::time::Duration::from_millis(2500) {
                    let _ = std::fs::remove_file(&flag_path);
                    println!("[Toggle] Suppression flag active (<2.5s). Exiting new instance (Toggle OFF successful).");
                    restore_windows_system_cursor();
                    return false;
                }
            }
        }
    }
    let _ = std::fs::remove_file(&flag_path);

    // 2. 이미 다른 Keysor 메인 프로세스가 구동 중인 경우 단일 인스턴스 보장
    use windows_sys::Win32::System::Threading::CreateMutexW;
    use windows_sys::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};

    let mutex_name = encode_wide("Local\\Keysor_Single_Instance_Mutex");
    unsafe {
        let mutex = CreateMutexW(std::ptr::null(), 0, mutex_name.as_ptr());
        if GetLastError() == ERROR_ALREADY_EXISTS {
            println!("[SingleInstance] Another Keysor instance is already running. Exiting duplicate instance.");
            return false;
        }
        Box::leak(Box::new(mutex));
    }

    true
}

#[cfg(target_os = "windows")]
pub fn restore_windows_system_cursor() {
    use windows_sys::Win32::UI::WindowsAndMessaging::{SystemParametersInfoW, SPIF_SENDCHANGE, SPI_SETCURSORS};
    unsafe {
        SystemParametersInfoW(SPI_SETCURSORS, 0, std::ptr::null_mut(), SPIF_SENDCHANGE);
    }
}

#[cfg(target_os = "windows")]
fn run_windows_watchdog(parent_pid: u32) {
    const SYNCHRONIZE: u32 = 0x00100000;
    use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, WaitForSingleObject};

    unsafe {
        // 1. 워치독 시작 즉시 이전 남아있을 수 있는 투명 커서 1차 복구 (Self-Healing)
        restore_windows_system_cursor();

        let h_process = OpenProcess(SYNCHRONIZE | PROCESS_QUERY_INFORMATION, 0, parent_pid);
        if h_process != 0 {
            // CPU 점유율 0.00% 완전 수면 상태로 부모(메인 키서) 프로세스 강제종료/사망 감시 대기
            WaitForSingleObject(h_process, 0xFFFFFFFF); // INFINITE
            windows_sys::Win32::Foundation::CloseHandle(h_process);
        } else {
            // 이미 부모 프로세스가 종료된 상태라면 잠시 대기
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        // 2. 메인 키서 종료/강제종료(taskkill /F, 크래시 등) 감지 시 마우스 커서 100% 즉시 복구
        for _ in 0..3 {
            restore_windows_system_cursor();
            std::thread::sleep(std::time::Duration::from_millis(30));
        }
    }
}

#[cfg(target_os = "windows")]
fn spawn_watchdog_subprocess() {
    if let Ok(exe_path) = std::env::current_exe() {
        use std::os::windows::process::CommandExt;
        const DETACHED_PROCESS: u32 = 0x00000008;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
        let parent_pid = std::process::id();
        let _ = std::process::Command::new(exe_path)
            .args(["--watchdog", &parent_pid.to_string()])
            .creation_flags(DETACHED_PROCESS | CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP)
            .spawn();
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    #[cfg(target_os = "windows")]
    {
        // 0. 비상 커서 복구 커맨드라인 인자 처리 (keysor --restore 또는 keysor -r)
        if args.len() >= 2 && (args[1] == "--restore" || args[1] == "--restore-cursor" || args[1] == "-r") {
            restore_windows_system_cursor();
            println!("[Keysor] Windows system cursor restored successfully.");
            return;
        }

        if args.len() >= 3 && args[1] == "--watchdog" {
            if let Ok(pid) = args[2].parse::<u32>() {
                run_windows_watchdog(pid);
                return;
            }
        }

        // 1. 단일 인스턴스 토글 스위치 (종료 직후 재실행 억제 및 중복 실행 방지)
        if !check_toggle_and_single_instance() {
            return;
        }
    }

    #[cfg(target_os = "windows")]
    {
        // 1. 키서 구동 즉시 커서 0.1초 원복 (Self-Healing)
        restore_windows_system_cursor();

        // 2. 튕김/패닉 크래시 예외 핸들러 등록 (비상 에어백)
        let default_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic_info| {
            restore_windows_system_cursor();
            default_hook(panic_info);
        }));

        unsafe {
            unsafe extern "system" fn win32_crash_exception_filter(_: *const windows_sys::Win32::System::Diagnostics::Debug::EXCEPTION_POINTERS) -> i32 {
                restore_windows_system_cursor();
                1 // EXCEPTION_EXECUTE_HANDLER
            }
            windows_sys::Win32::System::Diagnostics::Debug::SetUnhandledExceptionFilter(Some(win32_crash_exception_filter));
        }

        // 3. 커널 강제종료(taskkill /F) 감지 독립 워치독 보조 프로세스 가동 (CPU 0.00%, RAM < 1MB)
        spawn_watchdog_subprocess();
    }
    #[cfg(target_os = "macos")]
    if !ensure_single_instance() {
        println!("[Keysor] Duplicate process detected. Terminating duplicate instance.");
        std::process::exit(0);
    }


    #[cfg(windows)]
    unsafe {
        // Windows 10 (1607+) PerMonitorV2 다중 모니터 DPI 개별 연동 설정
        type SetProcessDpiAwarenessContextFn = unsafe extern "system" fn(isize) -> i32;
        let user32 = windows_sys::Win32::System::LibraryLoader::LoadLibraryA(b"user32.dll\0".as_ptr());
        if user32 != 0 {
            if let Some(proc) = windows_sys::Win32::System::LibraryLoader::GetProcAddress(user32, b"SetProcessDpiAwarenessContext\0".as_ptr()) {
                let set_dpi: SetProcessDpiAwarenessContextFn = std::mem::transmute(proc);
                set_dpi(-4); // DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2
            } else {
                windows_sys::Win32::UI::WindowsAndMessaging::SetProcessDPIAware();
            }
        } else {
            windows_sys::Win32::UI::WindowsAndMessaging::SetProcessDPIAware();
        }
        
        // 중복 실행 감지: 기존 실행 중인 키서 창이 있으면 WM_CLOSE를 전송하여 종료시키고 새 인스턴스는 종료 (Toggle 기능)
        let class_name = encode_wide("KeysorMainClass");
        let hwnd = windows_sys::Win32::UI::WindowsAndMessaging::FindWindowW(
            class_name.as_ptr(),
            std::ptr::null(),
        );
        if hwnd != 0 {
            windows_sys::Win32::UI::WindowsAndMessaging::SendMessageW(
                hwnd,
                windows_sys::Win32::UI::WindowsAndMessaging::WM_CLOSE,
                0,
                0,
            );
            return;
        }
    }
    
    let config_path = config::get_config_path();

    println!("====================================================");
    println!("      키서 (Keysor) - 초안정성 마우스 제어 유틸리티");
    println!("====================================================");
    println!("설정 파일: {}", config_path.display());
    println!("조작 가이드:");
    println!("  - [Caps Lock] (지속 홀드)  : 마우스 제어 모드 활성화");
    println!("  - [Caps Lock] (단독 탭)     : 본래의 대소문자 고정(Toggle) 기능 작동");
    println!("  - [이동]                    : WASD 키 (왼손) 또는 방향키 (오른손)");
    println!("  - [클릭]                    : Space 1회(좌클릭), Space 2회 연타(더블클릭)");
    println!("  - [우클릭 홀드]              : Space 지속 홀드 (손 뗄 시 우클릭 해제)");
    println!("  - [보조 클릭/스크롤]         : R (휠 위로), F (휠 아래로), G / Right-Shift (우클릭)");
    println!("====================================================");

    // 1. 초기 설정 로드 (오류 시 자동 디폴트 내장 설정 폴백 작동)
    let config = config::load_config(&config_path);
    println!("[Info] Configuration loaded successfully.");

    // 2. 라이선스 상태 검증 및 평가판(14일) 여부 확인
    let is_pro = license::check_local_license();
    let is_trial = license::check_trial_status();
    if is_pro {
        println!("[Info] Keysor Pro mode enabled.");
    } else if is_trial {
        println!("[Info] Keysor Trial mode enabled.");
    } else {
        println!("[Info] Keysor Free mode enabled (Standard key mappings active, Pro features locked).");
    }

    // 3. 설정 파일에 라이선스 키가 있고 로컬 인증이 안 된 경우 백그라운드 자동 활성화 구동
    if let Some(lic_key) = config.settings.license_key.as_ref()
        .filter(|key| !key.trim().is_empty() && !license::check_local_license()) {
        license::start_auto_activation_worker(lic_key.clone());
    }
    // 3-1. 14일 실시간 백그라운드 라이선스 만료 감지 스케줄러 시작
    license::start_license_verification_scheduler();

    // 3-2. 백그라운드 업데이트 확인 서비스 가동
    update::check_for_updates_async();

    // 4. 커서 시각 인디케이터 스레드 가동
    indicator::start_indicator();

    // 5. 백그라운드 윈도우 키보드 저수준 훅 스레드 구동
    hook::start_hook(config, is_pro, is_trial);
    println!("[Info] Keysor active and listening to inputs in background.");

    // 6. 앱 비정상 종료 및 CTRL+C 시 훅 리소스를 OS에 강제 안전 반환하는 페일세이프 가드 등록
    ctrlc_shutdown_handler();

    // 7. 단축키 파일 실시간 변경 감지(Hot-Reloading) 및 포커스 락 방지 100ms 가드 루프
    let mut last_modified = get_modified_time(&config_path).unwrap_or(SystemTime::now());

    loop {
        #[cfg(target_os = "macos")]
        unsafe {
            unsafe extern "C" {
                fn keysor_macos_pump_events();
            }
            keysor_macos_pump_events();
            thread::sleep(Duration::from_millis(30));
        }
        #[cfg(not(target_os = "macos"))]
        thread::sleep(Duration::from_millis(100));

        // A. 포커스 전환에 의한 키보드 오작동 엉킴 방지 실시간 동기화 가드
        hook::modifier_sync_guard();

        // B. keysor.yaml 변경 감지 핫리로드 처리
        if let Some(current_modified) = get_modified_time(&config_path).ok()
            .filter(|&modified| modified != last_modified) {
            last_modified = current_modified;
            println!("[Hot-Reload] keysor.yaml change detected. Reloading configuration...");
            
            let new_config = config::load_config(&config_path);
            if let Some(state_arc) = hook::APP_STATE.get() {
                let mut state = state_arc.lock().unwrap();
                state.reload_configuration(new_config);
            }
        }
    }
}

/// 파일 수정 시간 획득 유틸리티
fn get_modified_time<P: AsRef<Path>>(path: P) -> std::io::Result<SystemTime> {
    fs::metadata(path).and_then(|m| m.modified())
}

/// 프로세스 강제 강제 종료 감지 및 페일세이프 훅 회수 등록
#[cfg(windows)]
fn ctrlc_shutdown_handler() {
    unsafe {
        // Rust ctrlc 크레이트 없이 표준 std::sync 신호 처리를 모방
        // Win32 Console Control Handler 등록을 통해 CMD 창 닫기 및 콘솔 종료 감지
        extern "system" fn console_ctrl_handler(ctrl_type: u32) -> i32 {
            // CTRL_C_EVENT(0), CTRL_BREAK_EVENT(1), CTRL_CLOSE_EVENT(2) 등 모든 닫기 신호 수신
            if ctrl_type <= 2 {
                println!("\n[Shutdown] Cleaning up low-level hook resources safely before exit...");
                hook::cleanup_hook();
                ui::win_gdi::force_restore_system_cursor();
                std::process::exit(0);
            }
            0
        }
        windows_sys::Win32::System::Console::SetConsoleCtrlHandler(Some(console_ctrl_handler), 1);
    }
}

#[cfg(not(windows))]
fn ctrlc_shutdown_handler() {
    // macOS/Unix shutdown handling
}


