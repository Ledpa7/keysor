use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

#[allow(dead_code)]
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Settings {
    pub modifier_mode: String,
    pub modifier_key: String,
    pub base_speed: f64,
    pub max_speed: f64,
    pub acceleration: f64,
    pub unified_space_click: bool,
    pub double_click_threshold_ms: u64,
    pub drag_hold_threshold_ms: u64,
    pub refresh_rate_hz: Option<u64>,
    pub pixel_mode: Option<bool>,
    pub lang_en: Option<bool>,
    pub magnetic_mode: Option<bool>,
    pub global_magnetic_mode: Option<bool>,
    pub magnetic_sensitivity: Option<f64>,
    pub license_key: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Binding {
    pub trigger: String,
    pub action: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    pub settings: Settings,
    pub bindings: Vec<Binding>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            settings: Settings {
                modifier_mode: "Toggle".to_string(),
                modifier_key: "CapsLock".to_string(),
                base_speed: 1.5,
                max_speed: 30.0,
                acceleration: 1.5,
                unified_space_click: true,
                double_click_threshold_ms: 280,
                drag_hold_threshold_ms: 250,
                refresh_rate_hz: Some(100),
                pixel_mode: Some(false),
                lang_en: Some(true),
                magnetic_mode: Some(false),
                global_magnetic_mode: Some(false),
                magnetic_sensitivity: Some(1.3),
                license_key: None,
            },
            bindings: vec![
                Binding { trigger: "W".to_string(), action: "MouseMoveUp".to_string() },
                Binding { trigger: "S".to_string(), action: "MouseMoveDown".to_string() },
                Binding { trigger: "A".to_string(), action: "MouseMoveLeft".to_string() },
                Binding { trigger: "D".to_string(), action: "MouseMoveRight".to_string() },
                Binding { trigger: "R".to_string(), action: "MouseScrollUp".to_string() },
                Binding { trigger: "F".to_string(), action: "MouseScrollDown".to_string() },
                Binding { trigger: "Q".to_string(), action: "MouseScrollLeft".to_string() },
                Binding { trigger: "E".to_string(), action: "MouseScrollRight".to_string() },
                Binding { trigger: "G".to_string(), action: "MouseRightClick".to_string() },
            ],
        }
    }
}

pub fn get_config_path() -> PathBuf {
    #[cfg(windows)]
    let base = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME"));
    #[cfg(not(windows))]
    let base = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE"));

    let mut path = base.map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("."));
    path.push(".keysor");
    if !path.exists() {
        std::fs::create_dir_all(&path).ok();
    }
    path.push("keysor.yaml");
    path
}

pub fn load_config<P: AsRef<Path>>(path: P) -> Config {
    let default_cfg = Config::default();
    
    // 파일이 존재하지 않는 경우 자동으로 기본 파일 생성
    if !path.as_ref().exists() {
        let yaml_content = r#"# keysor.yaml - 키서 단축키 및 동작 설정 파일
settings:
  modifier_mode: "Toggle"        # "Hold" (Caps Lock을 누르는 동안에만 조작) 또는 "Toggle" (켜고 끄기)
  modifier_key: "CapsLock"     # 활성화 단축키 (기본값 CapsLock)

  base_speed: 1.5              # 마우스 시작 속도 (픽셀 단위)
  max_speed: 30.0              # 마우스 최대 속도 (픽셀 단위)
  acceleration: 1.5            # 누르고 있을 때의 가속도 비율

  # 스페이스바 단일 키 클릭 연동 특수 기능 활성화
  unified_space_click: true
  double_click_threshold_ms: 280
  drag_hold_threshold_ms: 250
  refresh_rate_hz: 100         # 이동 주기 주사율 (기본값 100Hz, 원격 접속 시 60으로 낮추면 부드러워짐)
  magnetic_mode: false         # 자석 모드 활성화 여부 (HUD 버튼에 마우스가 다가가면 흡착)
  global_magnetic_mode: false  # 전역 UI 자석 모드 활성화 여부 (윈도우 내의 모든 버튼 및 웹페이지 내 버튼/링크 흡착)
  magnetic_sensitivity: 1.3    # 1920 해상도 등에서 적용할 자석 감도 스케일 배율 (기본값 1.3)
  lang_en: true                # 기본 언어 영어 설정 (true: 영어, false: 한국어)
  license_key: ""              # 프로 라이선스 키 (구입 후 발급받은 키 입력)

bindings:
  # ==========================================
  # 1. 이동 바인딩 (하이브리드 지원)
  # ==========================================
  # 왼손 단독 제어 (WASD)
  - trigger: "W"
    action: "MouseMoveUp"
  - trigger: "S"
    action: "MouseMoveDown"
  - trigger: "A"
    action: "MouseMoveLeft"
  - trigger: "D"
    action: "MouseMoveRight"

  # ==========================================
  # 2. 보조 클릭 및 스크롤 바인딩
  # ==========================================
  - trigger: "R"
    action: "MouseScrollUp"
  - trigger: "F"
    action: "MouseScrollDown"
  - trigger: "Q"
    action: "MouseScrollLeft"
  - trigger: "E"
    action: "MouseScrollRight"
  - trigger: "G"
    action: "MouseRightClick"
"#;
        std::fs::write(&path, yaml_content).ok();
        return default_cfg;
    }

    let mut file = match File::open(&path) {
        Ok(f) => f,
        Err(_) => {
            eprintln!("[Warning] Failed to open config file. Using defaults.");
            return default_cfg;
        }
    };

    let mut content = String::new();
    if file.read_to_string(&mut content).is_err() {
        eprintln!("[Warning] Failed to read config file. Using defaults.");
        return default_cfg;
    }

    match parse_config(&content) {
        Ok(cfg) => cfg,
        Err(error) => {
            eprintln!("[Config] {}. Keeping the file unchanged; using defaults at startup.", error);
            default_cfg
        }
    }
}

pub fn parse_config(content: &str) -> Result<Config, String> {
    let cfg: Config = serde_yaml::from_str(content).map_err(|e| e.to_string())?;
    let s = &cfg.settings;
    if !s.modifier_mode.eq_ignore_ascii_case("Toggle") && !s.modifier_mode.eq_ignore_ascii_case("Hold") {
        return Err("modifier_mode must be Toggle or Hold".into());
    }
    if key_name_to_vk(&s.modifier_key).is_none() { return Err("Unsupported modifier_key".into()); }
    if !s.base_speed.is_finite() || !s.max_speed.is_finite() || !s.acceleration.is_finite()
        || s.base_speed <= 0.0 || s.max_speed < s.base_speed || s.acceleration < 0.0 {
        return Err("Speeds must be finite: 0 < base_speed <= max_speed, acceleration >= 0".into());
    }
    if s.refresh_rate_hz.is_some_and(|hz| !(10..=1000).contains(&hz))
        || !(1..=5000).contains(&s.double_click_threshold_ms)
        || !(1..=5000).contains(&s.drag_hold_threshold_ms)
        || s.magnetic_sensitivity.is_some_and(|v| !v.is_finite() || v <= 0.0) {
        return Err("Invalid refresh rate, click delay, or magnetic sensitivity".into());
    }
    let mut keys = std::collections::HashSet::new();
    for binding in &cfg.bindings {
        let vk = key_name_to_vk(&binding.trigger).ok_or("Unsupported binding key")?;
        if !keys.insert(vk) { return Err("Duplicate binding key".into()); }
        if !matches!(binding.action.as_str(), "MouseMoveUp" | "MouseMoveDown" | "MouseMoveLeft"
            | "MouseMoveRight" | "MouseScrollUp" | "MouseScrollDown" | "MouseScrollLeft"
            | "MouseScrollRight" | "MouseLeftClick" | "MouseRightClick")
            && !binding.action.strip_prefix("RunApp:").is_some_and(|path| !path.trim().is_empty()) {
            return Err("Unsupported binding action".into());
        }
    }
    Ok(cfg)
}

pub fn try_load_config(path: &Path) -> Result<Config, String> {
    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    parse_config(&content)
}

pub fn save_config(config: &Config) {
    let result = serde_yaml::to_string(config).map_err(|e| e.to_string()).and_then(|content| {
        parse_config(&content)?;
        atomic_write(&get_config_path(), content.as_bytes()).map_err(|e| e.to_string())
    });
    if let Err(e) = result { eprintln!("[Config] Save failed: {}", e); }
}

pub(crate) fn atomic_write(path: &Path, data: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let temp = path.with_extension(format!("tmp-{}-{}", std::process::id(), SEQUENCE.fetch_add(1, Ordering::Relaxed)));
    let result = (|| {
        let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(&temp)?;
        file.write_all(data)?;
        file.sync_all()?;
        drop(file);
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            #[link(name = "kernel32")]
            unsafe extern "system" { fn MoveFileExW(from: *const u16, to: *const u16, flags: u32) -> i32; }
            let from: Vec<u16> = temp.as_os_str().encode_wide().chain(Some(0)).collect();
            let to: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
            if unsafe { MoveFileExW(from.as_ptr(), to.as_ptr(), 0x1 | 0x8) } == 0 {
                return Err(std::io::Error::last_os_error());
            }
        }
        #[cfg(not(windows))]
        std::fs::rename(&temp, path)?;
        Ok(())
    })();
    if result.is_err() { let _ = std::fs::remove_file(temp); }
    result
}

/// 문자열로 된 키 이름을 Windows 가상 키 코드(VK Code)로 변환합니다.
pub fn key_name_to_vk(name: &str) -> Option<u32> {
    let lower = name.to_lowercase();
    match lower.as_str() {
        "capslock" => Some(0x14),     // VK_CAPITAL
        "space" => Some(0x20),        // VK_SPACE
        "enter" | "return" => Some(0x0D), // VK_RETURN
        "rightshift" => Some(0xA1),   // VK_RSHIFT
        "leftshift" | "shift" => Some(0x10), // VK_SHIFT
        "ctrl" | "control" => Some(0x11), // VK_CONTROL
        "alt" | "menu" => Some(0x12),  // VK_MENU
        "up" => Some(0x26),           // VK_UP
        "down" => Some(0x28),         // VK_DOWN
        "left" => Some(0x25),         // VK_LEFT
        "right" => Some(0x27),        // VK_RIGHT
        "w" => Some(0x57),
        "a" => Some(0x41),
        "s" => Some(0x53),
        "d" => Some(0x44),
        "f" => Some(0x46),
        "e" => Some(0x45),
        "g" => Some(0x47),
        "r" => Some(0x52),
        "q" => Some(0x51),
        _ => {
            // 한 글자짜리 알파벳 매핑 지원
            if name.len() == 1 {
                let c = name.chars().next().unwrap().to_ascii_uppercase();
                if c.is_ascii_alphabetic() || c.is_ascii_digit() {
                    return Some(c as u32);
                }
            }
            None
        }
    }
}

pub fn get_vk_bindings(config: &Config) -> HashMap<u32, String> {
    let mut map = HashMap::new();
    for binding in &config.bindings {
        if let Some(vk) = key_name_to_vk(&binding.trigger) {
            map.insert(vk, binding.action.clone());
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_round_trip() {
        assert!(parse_config(&serde_yaml::to_string(&Config::default()).unwrap()).is_ok());
    }
    #[test]
    fn invalid_numeric_settings_are_rejected() {
        for value in [f64::NAN, f64::INFINITY, -1.0, 0.0] {
            let mut cfg = Config::default();
            cfg.settings.base_speed = value;
            assert!(parse_config(&serde_yaml::to_string(&cfg).unwrap()).is_err());
        }
    }
    #[test]
    fn malformed_reload_preserves_original_file() {
        let path = std::env::temp_dir().join(format!("keysor-config-test-{}.yaml", std::process::id()));
        let broken = "settings: [unfinished";
        std::fs::write(&path, broken).unwrap();
        assert!(try_load_config(&path).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), broken);
        atomic_write(&path, b"replacement").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "replacement");
        std::fs::remove_file(path).unwrap();
    }
}
