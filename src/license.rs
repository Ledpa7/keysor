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
    #[cfg(windows)]
    let base = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME"));
    #[cfg(not(windows))]
    let base = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE"));

    let mut path = base.map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("."));
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
#[cfg(windows)]
pub fn get_machine_id() -> String {
    MACHINE_ID_CACHE.get_or_init(|| {
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

        let comp_name = std::env::var("COMPUTERNAME").unwrap_or_else(|_| "unknown_pc".to_string());
        let user_profile = std::env::var("USERPROFILE").unwrap_or_else(|_| "unknown_user".to_string());
        format!("{}_{}", comp_name, user_profile.len())
    }).clone()
}

fn extract_ioplatform_uuid(output: &str) -> Option<&str> {
    output.lines().find_map(|line| {
        let (_, value) = line.split_once("IOPlatformUUID")?.1.split_once('=')?;
        let value = value.trim().trim_matches('"');
        (value.len() == 36 && value.chars().enumerate().all(|(i, c)| {
            if matches!(i, 8 | 13 | 18 | 23) { c == '-' } else { c.is_ascii_hexdigit() }
        })).then_some(value)
    })
}

#[cfg(not(windows))]
pub fn get_machine_id() -> String {
    MACHINE_ID_CACHE.get_or_init(|| {
        if let Ok(output) = std::process::Command::new("ioreg")
            .args(["-rd1", "-c", "IOPlatformExpertDevice"]).output() {
            if let Some(uuid) = extract_ioplatform_uuid(&String::from_utf8_lossy(&output.stdout)) {
                return uuid.to_string();
            }
        }
        let path = get_keysor_dir().join(".machine-id");
        if let Ok(id) = fs::read_to_string(&path) {
            if let Some(id) = extract_ioplatform_uuid(&format!("IOPlatformUUID = \"{}\"", id.trim())) {
                return id.to_string();
            }
        }
        if let Ok(output) = std::process::Command::new("uuidgen").output() {
            if let Some(id) = extract_ioplatform_uuid(&format!("IOPlatformUUID = \"{}\"", String::from_utf8_lossy(&output.stdout).trim())) {
                if crate::config::atomic_write(&path, id.as_bytes()).is_ok() {
                    return id.to_string();
                }
            }
        }
        let hostname = std::process::Command::new("hostname").output().ok()
            .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
            .unwrap_or_default();
        format!("{}:{}", hostname, std::env::var("HOME").unwrap_or_default())
    }).clone()
}

fn machine_id_matches(cached: &str) -> bool {
    let current = get_machine_id();
    if cached == current { return true; }
    #[cfg(target_os = "macos")]
    if current.len() == 36 && cached == format!("IOPlatformUUID\" = \"{current}") {
        return true;
    }
    false
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
const KEYSOR_PRODUCT_ID: u64 = 1180943;
const PRODUCT_MISMATCH: &str = "License product mismatch";

const RECHECK_SECS: u64 = 14 * 24 * 60 * 60;
const OFFLINE_LIMIT_SECS: u64 = 2 * RECHECK_SECS;
static LICENSE_REQUEST: std::sync::Mutex<()> = std::sync::Mutex::new(());

struct CachedLicense {
    key: String,
    machine: String,
    timestamp: u64,
    instance: Option<String>,
    product_id: Option<u64>,
}

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
}

fn cached_license() -> Option<CachedLicense> {
    let data = fs::read(get_keysor_dir().join(".license")).ok()?;
    let plain = deobfuscate(&data)?;
    parse_cached_license(&plain)
}

fn parse_cached_license(plain: &str) -> Option<CachedLicense> {
    let parts: Vec<_> = plain.split('|').collect();
    if parts.len() < 4 || parts[2] != "active" || parts[0].is_empty() { return None; }
    Some(CachedLicense {
        key: parts[0].to_string(), machine: parts[1].to_string(), timestamp: parts[3].parse().ok()?,
        instance: parts.get(4).filter(|id| !id.is_empty()).map(|id| id.to_string()),
        product_id: match parts.get(5) { Some(id) => Some(id.parse().ok()?), None => None },
    })
}

fn cache_is_current(timestamp: u64, now: u64) -> bool {
    now.checked_sub(timestamp).is_some_and(|elapsed| elapsed < OFFLINE_LIMIT_SECS)
}

fn cached_entitlement_is_current(license: &CachedLicense, now: u64) -> bool {
    // Legacy caches retain their existing, bounded offline grace while migration
    // is attempted online; a known foreign product is never accepted locally.
    license.product_id.is_none_or(|id| id == KEYSOR_PRODUCT_ID) && cache_is_current(license.timestamp, now)
}

fn license_needs_recheck(license: &CachedLicense, now: u64) -> bool {
    license.product_id != Some(KEYSOR_PRODUCT_ID)
        || now.checked_sub(license.timestamp).is_none_or(|age| age >= RECHECK_SECS)
}

pub fn check_local_license() -> bool {
    cached_license().is_some_and(|lic| machine_id_matches(&lic.machine) && cached_entitlement_is_current(&lic, now_secs()))
}

fn save_license(key: &str, instance: Option<&str>) -> Result<(), String> {
    let token = format!("{}|{}|active|{}|{}|{}", key, get_machine_id(), now_secs(), instance.unwrap_or(""), KEYSOR_PRODUCT_ID);
    crate::config::atomic_write(&get_keysor_dir().join(".license"), &obfuscate(&token))
        .map_err(|e| format!("Cannot save license cache: {}", e))
}

fn license_request(endpoint: &str, form: &[(&str, &str)]) -> Result<Value, String> {
    let agent = ureq::AgentBuilder::new().timeout(std::time::Duration::from_secs(10)).build();
    match agent.post(&format!("https://api.lemonsqueezy.com/v1/licenses/{}", endpoint))
        .set("Accept", "application/json").send_form(form) {
        Ok(response) => response.into_json().map_err(|e| format!("Invalid license response: {}", e)),
        Err(ureq::Error::Status(code, _)) => Err(format!("Status code {}", code)),
        Err(_) => Err("License network request failed".into()),
    }
}

fn validate_license_response(json: &Value, success_field: &str) -> Result<(), String> {
    match json.get(success_field).and_then(Value::as_bool) {
        Some(true) => {},
        Some(false) => return Err("License rejected".into()),
        None => return Err("Invalid license response".into()),
    }
    match json.get("meta").and_then(|meta| meta.get("product_id")).and_then(Value::as_u64) {
        Some(KEYSOR_PRODUCT_ID) => Ok(()),
        Some(_) => Err(PRODUCT_MISMATCH.into()),
        None => Err("Invalid license response".into()),
    }
}

fn validate_cached_license(license: &CachedLicense) -> Result<(), String> {
    let mut form = vec![("license_key", license.key.as_str())];
    if let Some(id) = license.instance.as_deref() { form.push(("instance_id", id)); }
    let json = license_request("validate", &form)?;
    if let Err(error) = validate_license_response(&json, "valid") {
        if (error == PRODUCT_MISMATCH || error == "License rejected")
            && cached_license().is_some_and(|current| current.key == license.key) {
            let _ = fs::remove_file(get_keysor_dir().join(".license"));
        }
        return Err(error);
    }
    save_license(&license.key, license.instance.as_deref())
}

fn request_product_activation(
    key: &str, machine: &str,
    mut request: impl FnMut(&str, &[(&str, &str)]) -> Result<Value, String>,
) -> Result<String, String> {
    // Preflight has no activation side effect; foreign keys must stop here.
    let preflight = request("validate", &[("license_key", key)])?;
    validate_license_response(&preflight, "valid")?;
    let json = request("activate", &[("license_key", key), ("instance_name", machine)])?;
    validate_license_response(&json, "activated")?;
    json["instance"]["id"].as_str().filter(|id| !id.is_empty())
        .map(str::to_owned).ok_or_else(|| "Activation response has no instance ID".into())
}

pub fn activate_license(license_key: &str) -> Result<String, String> {
    let _guard = LICENSE_REQUEST.lock().map_err(|_| "License worker unavailable")?;
    let key = license_key.trim();
    if key.is_empty() || key.contains('|') { return Err("Invalid license key".into()); }
    // Reuse an existing instance; concurrent UI/config requests must not consume extra seats.
    if let Some(cached) = cached_license().filter(|c| c.key == key && machine_id_matches(&c.machine)) {
        validate_cached_license(&cached)?;
    } else {
        let instance = request_product_activation(key, &get_machine_id(), license_request)?;
        save_license(key, Some(&instance))?;
    }
    Ok("Pro License activated successfully!".into())
}

pub fn refresh_entitlements() {
    let is_pro = check_local_license();
    let is_trial = check_trial_status();
    let config = crate::config::try_load_config(&crate::config::get_config_path()).ok();
    if let Some(state_arc) = crate::hook::APP_STATE.get() {
        if let Ok(mut state) = state_arc.lock() {
            if state.is_pro != is_pro || state.is_trial != is_trial {
                let config = config.unwrap_or_else(|| state.config.clone());
                state.reload_configuration(config);
            }
        }
    }
}

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
                refresh_entitlements();
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

                if e == PRODUCT_MISMATCH {
                    show_alert(if lang_en { "Wrong Product License" } else { "다른 상품의 라이선스" },
                        if lang_en { "This license belongs to another product. Please enter a Keysor license key." }
                        else { "이 라이선스는 다른 상품용입니다. Keysor 라이선스 키를 입력해 주세요." });
                } else if e.contains("Status code 400") {
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
        while crate::hook::APP_STATE.get().is_none() {
            thread::sleep(std::time::Duration::from_millis(100));
        }
        let mut last_attempt = None::<std::time::Instant>;
        loop {
            refresh_entitlements();
            if last_attempt.is_none_or(|t| t.elapsed() >= std::time::Duration::from_secs(3600)) {
                if let Ok(_guard) = LICENSE_REQUEST.lock() {
                    if let Some(cached) = cached_license().filter(|c| machine_id_matches(&c.machine)) {
                        if license_needs_recheck(&cached, now_secs()) {
                            last_attempt = Some(std::time::Instant::now());
                            if let Err(error) = validate_cached_license(&cached) {
                                // Keep expired caches for recovery without creating another activation.
                                if error == "License rejected" || error == "Status code 404" || error == PRODUCT_MISMATCH {
                                    let _ = fs::remove_file(get_keysor_dir().join(".license"));
                                }
                                eprintln!("[License] {}", error);
                            }
                        }
                    }
                }
            }
            refresh_entitlements();
            thread::sleep(std::time::Duration::from_secs(60));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cache_rejects_expiry_and_clock_rollback() {
        assert!(cache_is_current(100, 100 + OFFLINE_LIMIT_SECS - 1));
        assert!(!cache_is_current(100, 100 + OFFLINE_LIMIT_SECS));
        assert!(!cache_is_current(100, 99));
        assert!(!cache_is_current(u64::MAX, 100));
    }
    #[test]
    fn ioplatform_uuid_parser_ignores_property_name() {
        let line = "    \"IOPlatformUUID\" = \"12345678-1234-1234-1234-123456789ABC\"";
        assert_eq!(extract_ioplatform_uuid(line), Some("12345678-1234-1234-1234-123456789ABC"));
        assert_eq!(extract_ioplatform_uuid("IOPlatformUUID = bad"), None);
    }

    #[test]
    fn only_keysor_product_is_accepted_for_activation_and_validation() {
        for field in ["valid", "activated"] {
            let mut response = serde_json::json!({"meta":{"product_id":1180943}});
            response[field] = Value::Bool(true);
            assert!(validate_license_response(&response, field).is_ok());
            response["meta"]["product_id"] = serde_json::json!(1180944);
            assert_eq!(validate_license_response(&response, field), Err(PRODUCT_MISMATCH.into()));
            response["meta"] = serde_json::json!({"variant_id":1180943});
            assert!(validate_license_response(&response, field).is_err(), "Variant ID must not stand in for product ID");
            response["meta"] = serde_json::json!({"product_id":1180943});
            response[field] = Value::Bool(false);
            assert!(validate_license_response(&response, field).is_err());
        }
    }

    #[test]
    fn malformed_product_metadata_fails_closed() {
        for meta in [Value::Null, serde_json::json!({}), serde_json::json!({"product_id":"1180943"}),
            serde_json::json!({"product_id":-1}), serde_json::json!({"product_id":1180943.5})] {
            assert!(validate_license_response(&serde_json::json!({"valid":true,"meta":meta}), "valid").is_err());
        }
        assert!(validate_license_response(&serde_json::json!({"meta":{"product_id":1180943}}), "valid").is_err());
    }

    #[test]
    fn legacy_cache_migrates_without_extending_offline_grace() {
        let legacy = parse_cached_license("test-key|test-machine|active|100|old-instance").unwrap();
        assert!(cached_entitlement_is_current(&legacy, 101));
        assert!(license_needs_recheck(&legacy, 101), "Legacy caches must be rechecked immediately");
        assert!(!cached_entitlement_is_current(&legacy, 100 + OFFLINE_LIMIT_SECS));
        let current = parse_cached_license("test-key|test-machine|active|100|old-instance|1180943").unwrap();
        assert!(cached_entitlement_is_current(&current, 101));
        assert!(!license_needs_recheck(&current, 101));
        assert!(license_needs_recheck(&current, 100 + RECHECK_SECS));
        let foreign = parse_cached_license("test-key|test-machine|active|100|old-instance|999").unwrap();
        assert!(!cached_entitlement_is_current(&foreign, 101));
        assert!(parse_cached_license("test-key|test-machine|active|100|old-instance|invalid").is_none());
    }

    #[test]
    fn foreign_product_never_consumes_an_activation_slot() {
        let mut calls = Vec::new();
        let result = request_product_activation("test-key", "test-machine", |endpoint, _| {
            calls.push(endpoint.to_owned());
            Ok(serde_json::json!({"valid":true,"meta":{"product_id":999}}))
        });
        assert_eq!(result, Err(PRODUCT_MISMATCH.into()));
        assert_eq!(calls, ["validate"]);
    }

    #[test]
    fn activation_requires_matching_product_again_and_a_device_instance() {
        for product in [1180943, 999] {
            let mut calls = Vec::new();
            let result = request_product_activation("test-key", "test-machine", |endpoint, form| {
                calls.push(endpoint.to_owned());
                assert_eq!(form[0], ("license_key", "test-key"));
                Ok(if endpoint == "validate" {
                    serde_json::json!({"valid":true,"meta":{"product_id":1180943}})
                } else {
                    serde_json::json!({"activated":true,"meta":{"product_id":product},"instance":{"id":"test-instance"}})
                })
            });
            assert_eq!(calls, ["validate", "activate"]);
            if product == KEYSOR_PRODUCT_ID { assert_eq!(result.unwrap(), "test-instance"); }
            else { assert_eq!(result, Err(PRODUCT_MISMATCH.into())); }
        }
        let mut count = 0;
        assert!(request_product_activation("test-key", "test-machine", |_, _| {
            count += 1;
            Err("License network request failed".into())
        }).is_err());
        assert_eq!(count, 1);
        assert!(request_product_activation("test-key", "test-machine", |endpoint, _| {
            Ok(serde_json::json!({"valid":true,"activated":endpoint == "activate","meta":{"product_id":1180943}}))
        }).is_err(), "A successful activation must supply its reusable instance ID");
    }

}
