use std::thread;
use std::process::Command;
use std::time::Duration;

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const VERSION_CHECK_URL: &str = "https://api.github.com/repos/Ledpa7/keysor/releases/latest";
const DOWNLOAD_PAGE: &str = "https://www.keysor.lepa7.com";

/// 비동기로 버전 체크를 수행합니다.
pub fn check_for_updates_async() {
    thread::spawn(move || {
        // 프로그램 시작 직후 네트워크 지연 및 프로세스 안정화를 위해 5초 대기 후 시작
        thread::sleep(Duration::from_secs(5));
        
        println!("[Update] Checking for updates...");
        if let Some(latest_version) = fetch_latest_version() {
            println!("[Update] Latest version on GitHub: v{}, Current version: v{}", latest_version, CURRENT_VERSION);
            if is_newer_version(&latest_version, CURRENT_VERSION) {
                println!("[Update] New version detected! Prompting user...");
                prompt_update_dialog(&latest_version);
            } else {
                println!("[Update] Keysor is up to date.");
            }
        } else {
            println!("[Update] Failed to fetch latest version info.");
        }
    });
}

/// GitHub API를 통해 최신 버전을 획득합니다.
fn fetch_latest_version() -> Option<String> {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(5))
        .build();
        
    let response = agent.get(VERSION_CHECK_URL)
        .set("User-Agent", "Keysor-App")
        .call()
        .ok()?;
        
    let json: serde_json::Value = response.into_json().ok()?;
    let tag = json.get("tag_name")?.as_str()?;
    Some(tag.trim_start_matches('v').to_string())
}

/// SemVer 형식의 버전 정보를 정수 튜플로 파싱합니다. (예: "0.1.0" -> (0, 1, 0))
fn parse_version(v: &str) -> Option<(u32, u32, u32)> {
    let clean = v.trim_start_matches('v');
    let parts: Vec<&str> = clean.split('.').collect();
    if parts.len() >= 3 {
        let major = parts[0].parse::<u32>().ok()?;
        let minor = parts[1].parse::<u32>().ok()?;
        let patch = parts[2].parse::<u32>().ok()?;
        Some((major, minor, patch))
    } else {
        None
    }
}

/// 최신 버전이 현재 버전보다 더 높은 버전인지 비교합니다.
fn is_newer_version(latest: &str, current: &str) -> bool {
    let latest_parsed = parse_version(latest);
    let current_parsed = parse_version(current);
    
    match (latest_parsed, current_parsed) {
        (Some((l_maj, l_min, l_pat)), Some((c_maj, c_min, c_pat))) => {
            if l_maj != c_maj {
                l_maj > c_maj
            } else if l_min != c_min {
                l_min > c_min
            } else {
                l_pat > c_pat
            }
        }
        _ => false,
    }
}

/// 윈도우 환경에서 MessageBox를 띄워 사용자에게 업데이트를 제안합니다.
#[cfg(windows)]
fn prompt_update_dialog(new_version: &str) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        MessageBoxW, MB_YESNO, MB_ICONINFORMATION, IDYES, MB_TOPMOST, MB_SETFOREGROUND
    };

    let text = format!(
        "키서(Keysor)의 새로운 버전(v{})이 출시되었습니다!\n\n업데이트 다운로드 페이지로 이동하시겠습니까?", 
        new_version
    );
    let title = "Keysor 업데이트 알림";

    let wide_text: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    let wide_title: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
    
    unsafe {
        let result = MessageBoxW(
            0,
            wide_text.as_ptr(),
            wide_title.as_ptr(),
            MB_YESNO | MB_ICONINFORMATION | MB_TOPMOST | MB_SETFOREGROUND,
        );
        
        if result == IDYES {
            let _ = Command::new("cmd")
                .args(["/c", "start", DOWNLOAD_PAGE])
                .spawn();
        }
    }
}

/// 비윈도우 환경용 폴백
#[cfg(not(windows))]
fn prompt_update_dialog(new_version: &str) {
    println!("[Update] New version v{} is available. Please download from: {}", new_version, DOWNLOAD_PAGE);
}
