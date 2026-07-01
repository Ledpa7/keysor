#![windows_subsystem = "windows"]

mod config;
mod math;
mod hook;
mod platform;
mod license;

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

fn main() {
    #[cfg(windows)]
    unsafe {
        // SetProcessDPIAware(); // 고해상도(DPI) 환경에서 창이 너무 작게 나오는 현상을 방지하기 위해 OS 자동 스케일링으로 위임
        
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
    
    println!("====================================================");
    println!("      키서 (Keysor) - 초안정성 마우스 제어 유틸리티");
    println!("====================================================");
    println!("설정 파일: C:\\Users\\wjdwl\\.keysor\\keysor.yaml");
    println!("조작 가이드:");
    println!("  - [Caps Lock] (지속 홀드)  : 마우스 제어 모드 활성화");
    println!("  - [Caps Lock] (단독 탭)     : 본래의 대소문자 고정(Toggle) 기능 작동");
    println!("  - [이동]                    : WASD 키 (왼손) 또는 방향키 (오른손)");
    println!("  - [클릭]                    : Space 1회(좌클릭), Space 2회 연타(더블클릭)");
    println!("  - [우클릭 홀드]              : Space 지속 홀드 (손 뗄 시 우클릭 해제)");
    println!("  - [보조 클릭/스크롤]         : R (휠 위로), F (휠 아래로), G / Right-Shift (우클릭)");
    println!("====================================================");

    let config_path = config::get_config_path();

    // 1. 초기 설정 로드 (오류 시 자동 디폴트 내장 설정 폴백 작동)
    let config = config::load_config(&config_path);
    println!("[Info] Configuration loaded successfully.");

    // 2. 라이선스 상태 검증 및 평가판(14일) 여부 확인
    let is_pro = license::check_local_license() || license::check_trial_status();
    if is_pro {
        println!("[Info] Keysor Pro mode enabled.");
    } else {
        println!("[Info] Keysor Free mode enabled (Standard key mappings active, Pro features locked).");
    }

    // 3. 설정 파일에 라이선스 키가 있고 로컬 인증이 안 된 경우 백그라운드 자동 활성화 구동
    if let Some(ref lic_key) = config.settings.license_key {
        if !lic_key.trim().is_empty() && !license::check_local_license() {
            license::start_auto_activation_worker(lic_key.clone());
        }
    }
    // 3-1. 14일 실시간 백그라운드 라이선스 만료 감지 스케줄러 시작
    license::start_license_verification_scheduler();

    // 4. 커서 시각 인디케이터 스레드 가동
    indicator::start_indicator();

    // 5. 백그라운드 윈도우 키보드 저수준 훅 스레드 구동
    hook::start_hook(config, is_pro);
    println!("[Info] Keysor active and listening to inputs in background.");

    // 6. 앱 비정상 종료 및 CTRL+C 시 훅 리소스를 OS에 강제 안전 반환하는 페일세이프 가드 등록
    ctrlc_shutdown_handler();

    // 7. 단축키 파일 실시간 변경 감지(Hot-Reloading) 및 포커스 락 방지 100ms 가드 루프
    let mut last_modified = get_modified_time(&config_path).unwrap_or(SystemTime::now());

    loop {
        thread::sleep(Duration::from_millis(100)); // 100ms 주기로 경량 폴링

        // A. 포커스 전환에 의한 키보드 오작동 엉킴 방지 실시간 동기화 가드
        hook::modifier_sync_guard();

        // B. keysor.yaml 변경 감지 핫리로드 처리
        if let Ok(current_modified) = get_modified_time(&config_path) {
            if current_modified != last_modified {
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
