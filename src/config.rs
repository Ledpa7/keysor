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
                double_click_threshold_ms: 100,
                drag_hold_threshold_ms: 250,
                refresh_rate_hz: Some(100),
                pixel_mode: Some(false),
                lang_en: Some(true),
                magnetic_mode: Some(false),
                global_magnetic_mode: Some(false),
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
                Binding { trigger: "RightShift".to_string(), action: "MouseRightClick".to_string() },
            ],
        }
    }
}

pub fn get_config_path() -> PathBuf {
    let mut path = std::env::var("USERPROFILE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
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
  double_click_threshold_ms: 100
  drag_hold_threshold_ms: 250
  refresh_rate_hz: 100         # 이동 주기 주사율 (기본값 100Hz, 원격 접속 시 60으로 낮추면 부드러워짐)
  magnetic_mode: false         # 자석 모드 활성화 여부 (HUD 버튼에 마우스가 다가가면 흡착)
  global_magnetic_mode: false  # 전역 UI 자석 모드 활성화 여부 (윈도우 내의 모든 버튼 및 웹페이지 내 버튼/링크 흡착)
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
  - trigger: "RightShift"
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

    match serde_yaml::from_str::<Config>(&content) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("[Warning] Config parse error: {}. Using default fallback configuration.", e);
            default_cfg
        }
    }
}

pub fn save_config(config: &Config) {
    let path = get_config_path();
    if let Ok(content) = serde_yaml::to_string(config) {
        std::fs::write(path, content).ok();
    }
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
