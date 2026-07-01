use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use std::thread;
use std::sync::OnceLock;
use serde_json::Value;

const XOR_KEY: &[u8] = b"KEYSOR_SECURE_SALT_2026_PRO_EDITION";

#[cfg(windows)]
fn show_alert(title: &str, message: &str) {
    let wide_title: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
    let wide_msg: Vec<u16> = message.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        windows_sys::Win32::UI::WindowsAndMessaging::MessageBoxW(
            0,
            wide_msg.as_ptr(),
            wide_title.as_ptr(),
            0x00000010 | 0x00000000, // MB_ICONERROR | MB_OK
        );
    }
}

#[cfg(not(windows))]
fn show_alert(_title: &str, _message: &str) {}

/// .keysor 디렉토리 경로 획득
fn get_keysor_dir() -> PathBuf {
    let mut path = std::env::var("USERPROFILE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    path.push(".keysor");
    if !path.exists() {
        fs::create_dir_all(&path).ok();
    }
    path
}

/// 간단한 XOR 난독화 및 복호화
fn obfuscate(data: &str) -> Vec<u8> {
    data.as_bytes()
        .iter()
        .enumerate()
        .map(|(i, &b)| b ^ XOR_KEY[i % XOR_KEY.len()])
        .collect()
}

fn deobfuscate(data: &[u8]) -> Option<String> {
    let decrypted: Vec<u8> = data
        .iter()
        .enumerate()
        .map(|(i, &b)| b ^ XOR_KEY[i % XOR_KEY.len()])
        .collect();
    String::from_utf8(decrypted).ok()
}

static MACHINE_ID_CACHE: OnceLock<String> = OnceLock::new();

/// Windows 시스템 고유 머신 ID (MachineGuid) 획득
pub fn get_machine_id() -> String {
    MACHINE_ID_CACHE.get_or_init(|| {
        // reg query를 이용하여 Cryptography MachineGuid 추출
        let output = std::process::Command::new("reg")
            .args(&["query", "HKLM\\SOFTWARE\\Microsoft\\Cryptography", "/v", "MachineGuid"])
            .output();

        if let Ok(out) = output {
            let text = String::from_utf8_lossy(&out.stdout);
            for line in text.lines() {
                if line.contains("MachineGuid") {
                    if let Some(guid) = line.split_whitespace().last() {
                        return guid.trim().to_string();
                    }
                }
            }
        }

        // reg query 실패 시 폴백 (컴퓨터 명 + 사용자 경로 해시값 모방)
        let comp_name = std::env::var("COMPUTERNAME").unwrap_or_else(|_| "unknown_pc".to_string());
        let user_profile = std::env::var("USERPROFILE").unwrap_or_else(|_| "unknown_user".to_string());
        format!("{}_{}", comp_name, user_profile.len())
    }).clone()
}

/// 14일 무료 트라이얼 검증
pub fn check_trial_status() -> bool {
    let mut trial_path = get_keysor_dir();
    trial_path.push(".trial");

    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    if !trial_path.exists() {
        // 첫 실행인 경우 .trial 파일을 생성하고 현재 타임스탬프 기록
        let content = obfuscate(&now_secs.to_string());
        if fs::write(&trial_path, content).is_ok() {
            println!("[Trial] 14-day free trial started today.");
            return true;
        }
    }

    if let Ok(data) = fs::read(&trial_path) {
        if let Some(decrypted) = deobfuscate(&data) {
            if let Ok(start_secs) = decrypted.parse::<u64>() {
                // 시간 조작(미래에서 과거로 변경) 감지 방지 가드
                if now_secs < start_secs {
                    println!("[Trial] System clock rollback detected. Trial expired.");
                    return false;
                }

                let elapsed_secs = now_secs - start_secs;
                let trial_limit_secs = 14 * 24 * 60 * 60; // 14일

                if elapsed_secs < trial_limit_secs {
                    let remaining_days = (trial_limit_secs - elapsed_secs) / (24 * 60 * 60);
                    println!("[Trial] Trial is active. {} days remaining.", remaining_days);
                    return true;
                } else {
                    println!("[Trial] 14-day trial has expired.");
                    return false;
                }
            }
        }
    }

    false
}

/// 로컬 캐싱된 라이선스 유효성 검증
pub fn check_local_license() -> bool {
    let mut lic_path = get_keysor_dir();
    lic_path.push(".license");

    if !lic_path.exists() {
        return false;
    }

    if let Ok(data) = fs::read(&lic_path) {
        if let Some(decrypted) = deobfuscate(&data) {
            let parts: Vec<&str> = decrypted.split('|').collect();
            if parts.len() >= 3 {
                let license_key = parts[0];
                let machine_id = parts[1];
                let status = parts[2];

                // 현재 기기 ID와 토큰 기기 ID가 일치하며 status가 active인지 검증
                if machine_id == get_machine_id() && status == "active" {
                    println!("[License] Valid local license detected (Key: {}***). Pro features unlocked.", &license_key[..std::cmp::min(5, license_key.len())]);
                    return true;
                }
            }
        }
    }

    false
}

/// Lemon Squeezy API 라이선스 활성화 요청 및 로컬 캐시 기록
pub fn activate_license(license_key: &str) -> Result<String, String> {
    let machine_id = get_machine_id();
    println!("[License] Requesting activation for key: {}...", license_key);

    let url = "https://api.lemonsqueezy.com/v1/licenses/activate";
    
    // ureq를 사용한 HTTPS POST 요청
    let resp = ureq::post(url)
        .set("Accept", "application/json")
        .send_form(&[
            ("license_key", license_key),
            ("instance_name", &machine_id),
        ]);

    match resp {
        Ok(response) => {
            let json_str = match response.into_string() {
                Ok(s) => s,
                Err(e) => return Err(format!("Failed to read response body: {}", e)),
            };

            match serde_json::from_str::<Value>(&json_str) {
                Ok(json) => {
                    let activated = json["activated"].as_bool().unwrap_or(false);
                    let error = json["error"].as_str();

                    if activated {
                        // 활성화 성공 시 로컬 캐시 파일 작성
                        let mut lic_path = get_keysor_dir();
                        lic_path.push(".license");
                        
                        let token_data = format!("{}|{}|active|{}", license_key, machine_id, SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs());
                        let obfuscated_data = obfuscate(&token_data);

                        if fs::write(&lic_path, obfuscated_data).is_ok() {
                            Ok("Pro License activated successfully! Pro features unlocked.".to_string())
                        } else {
                            Err("Failed to save license token locally.".to_string())
                        }
                    } else if let Some(err_msg) = error {
                        Err(format!("Activation failed: {}", err_msg))
                    } else {
                        Err("Activation failed: Unknown server error.".to_string())
                    }
                }
                Err(e) => Err(format!("Failed to parse activation response JSON: {}", e)),
            }
        }
        Err(ureq::Error::Status(code, response)) => {
            let error_body = response.into_string().unwrap_or_else(|_| "Failed to read error body".to_string());
            Err(format!("Status code {}: {}", code, error_body))
        }
        Err(e) => {
            Err(format!("Network error during activation: {}", e))
        }
    }
}

/// 백그라운드에서 실시간 라이선스 키 활성화 자동 재인증 워커 구동
pub fn start_auto_activation_worker(license_key: String) {
    thread::spawn(move || {
        // 부팅 직후 네트워크 안정화를 위해 3초 대기 후 활성화 요청
        thread::sleep(std::time::Duration::from_secs(3));
        
        // 이미 로컬 라이선스가 유효하다면 별도의 중복 활성화 불필요
        if check_local_license() {
            return;
        }

        if license_key.trim().is_empty() {
            return;
        }

        match activate_license(&license_key) {
            Ok(msg) => {
                println!("[License Background Worker] {}", msg);
                // 실시간 상태 반영을 위해 전역 상태의 is_pro 갱신
                if let Some(state_arc) = crate::hook::APP_STATE.get() {
                    if let Ok(mut state) = state_arc.lock() {
                        state.is_pro = true;
                    }
                }
            }
            Err(e) => {
                println!("[License Background Worker] Auto-activation failed: {}. Reverting to trial mode.", e);
                
                let lang_en = {
                    if let Some(state_arc) = crate::hook::APP_STATE.get() {
                        state_arc.lock().map_or(true, |state| state.config.settings.lang_en.unwrap_or(true))
                    } else {
                        true
                    }
                };

                if e.contains("Status code 400") {
                    let title = if lang_en { "License Limit Exceeded" } else { "라이선스 기기 한도 초과" };
                    let msg = if lang_en {
                        "This license key has reached its activation limit (2 devices).\n\nPlease deactivate an existing device from the Lemon Squeezy portal and try again."
                    } else {
                        "라이선스 기기 활성화 한도(2대)를 초과했습니다.\n\nLemon Squeezy 구매 관리 화면에서 기존 기기 등록을 해제하신 후 다시 시도해 주세요."
                    };
                    show_alert(title, msg);
                } else if e.contains("Status code 404") {
                    let title = if lang_en { "Invalid License Key" } else { "잘못된 라이선스 키" };
                    let msg = if lang_en {
                        "The entered license key is invalid or does not exist.\n\nPlease double check your license key in keysor.yaml."
                    } else {
                        "입력하신 라이선스 키가 올바르지 않거나 존재하지 않습니다.\n\nkeysor.yaml 설정 파일 내의 라이선스 키를 다시 확인해 주세요."
                    };
                    show_alert(title, msg);
                }
            }
        }
    });
}

/// 남은 트라이얼 일수 반환 (유효할 경우 Some(days), 그 외 None)
pub fn get_remaining_trial_days() -> Option<u64> {
    let mut trial_path = get_keysor_dir();
    trial_path.push(".trial");

    if !trial_path.exists() {
        return Some(14); // 파일이 없으면 오늘 시작한 것이므로 14일
    }

    if let Ok(data) = fs::read(&trial_path) {
        if let Some(decrypted) = deobfuscate(&data) {
            if let Ok(start_secs) = decrypted.parse::<u64>() {
                let now_secs = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();

                if now_secs < start_secs {
                    return None; // 클럭 조작 감지
                }

                let elapsed_secs = now_secs - start_secs;
                let trial_limit_secs = 14 * 24 * 60 * 60; // 14일

                if elapsed_secs < trial_limit_secs {
                    let remaining_secs = trial_limit_secs - elapsed_secs;
                    // 남은 시간 올림 처리하여 최소 1일 이상으로 표기되게 함
                    let remaining_days = (remaining_secs + 24 * 60 * 60 - 1) / (24 * 60 * 60);
                    return Some(remaining_days);
                }
            }
        }
    }
    None
}

/// 14일마다 백그라운드에서 조용히 라이선스 유효성을 재인증하여 활성 만료를 추적합니다.
/// 네트워크 에러 등 일시적 장애 시에는 추가로 14일의 유예 기간(Grace Period)을 제공합니다.
pub fn start_license_verification_scheduler() {
    thread::spawn(move || {
        loop {
            // 1시간 주기로 라이선스 파일 타임스탬프를 감시합니다.
            thread::sleep(std::time::Duration::from_secs(3600));

            let mut lic_path = get_keysor_dir();
            lic_path.push(".license");
            if !lic_path.exists() {
                continue;
            }

            if let Ok(data) = fs::read(&lic_path) {
                if let Some(decrypted) = deobfuscate(&data) {
                    let parts: Vec<&str> = decrypted.split('|').collect();
                    if parts.len() >= 4 {
                        let license_key = parts[0].to_string();
                        if let Ok(timestamp) = parts[3].parse::<u64>() {
                            let now_secs = SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs();
                            
                            // 14일(1209600초)이 경과했다면 재발행 인증 요청
                            if now_secs >= timestamp + 1209600 {
                                println!("[License Scheduler] 14 days elapsed. Re-validating license in background...");
                                match activate_license(&license_key) {
                                    Ok(_) => {
                                        println!("[License Scheduler] License re-validation succeeded. Timestamp updated.");
                                    }
                                    Err(e) => {
                                        println!("[License Scheduler] License validation failed: {}.", e);
                                        
                                        // 서버가 명시적으로 400(한도 초과 등) 또는 404(키 없음) 에러를 반환한 경우 즉시 파기
                                        let is_hard_denial = e.contains("Status code 400") || e.contains("Status code 404");
                                        
                                        // 네트워크 에러나 서버 500대 등 일시적 오류일 경우, 마지막 성공일로부터 28일(14일 주기 + 14일 유예) 경과 시 최종 파기
                                        let is_grace_expired = now_secs >= timestamp + 1209600 + 1209600;
                                        
                                        if is_hard_denial || is_grace_expired {
                                            if is_hard_denial {
                                                println!("[License Scheduler] Hard denial from server. Deleting local license cache.");
                                            } else {
                                                println!("[License Scheduler] Grace period of 14 days expired. Deleting local license cache.");
                                            }
                                            
                                            let _ = fs::remove_file(&lic_path);
                                            if let Some(state_arc) = crate::hook::APP_STATE.get() {
                                                if let Ok(mut state) = state_arc.lock() {
                                                    state.is_pro = false;
                                                    let current_config = state.config.clone();
                                                    state.reload_configuration(current_config);
                                                }
                                            }
                                        } else {
                                            println!("[License Scheduler] Temporary network error. Grace period active (Pro features retained).");
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    });
}


