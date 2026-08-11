use crate::ui::{KeysorUi, ClickType};
use crate::platform::get_system_controller;
use std::sync::atomic::{AtomicBool, Ordering};

unsafe extern "C" {
    fn keysor_macos_ui_init();
    fn keysor_macos_ui_show(visible: bool);
    fn keysor_macos_ui_update_pos(x: f64, y: f64);
    fn keysor_macos_ui_set_click_motion(clickType: i32);
    fn keysor_macos_ui_update_speed_display(speed: f64);
}

#[unsafe(no_mangle)]
pub extern "C" fn keysor_macos_on_lang_toggle() {
    let mut config_to_save = None;
    if let Some(state_arc) = crate::hook::APP_STATE.get() {
        if let Ok(mut state) = state_arc.lock() {
            let cur = state.config.settings.lang_en.unwrap_or(false);
            state.config.settings.lang_en = Some(!cur);
            config_to_save = Some(state.config.clone());
        }
    }
    if let Some(cfg) = config_to_save {
        crate::config::save_config(&cfg);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn keysor_macos_get_base_speed() -> f64 {
    if let Some(state_arc) = crate::hook::APP_STATE.get() {
        if let Ok(state) = state_arc.lock() {
            return state.config.settings.base_speed;
        }
    }
    1.5
}

#[unsafe(no_mangle)]
pub extern "C" fn keysor_macos_on_speed_change(delta: f64) -> f64 {
    let mut config_to_save = None;
    let mut new_spd = 1.5;
    if let Some(state_arc) = crate::hook::APP_STATE.get() {
        if let Ok(mut state) = state_arc.lock() {
            let mut speed = state.config.settings.base_speed + delta;
            if speed < 0.1 { speed = 0.1; }
            if speed > 10.0 { speed = 10.0; }
            speed = (speed * 10.0).round() / 10.0;
            state.config.settings.base_speed = speed;
            new_spd = speed;
            config_to_save = Some(state.config.clone());
        }
    }
    if let Some(cfg) = config_to_save {
        crate::config::save_config(&cfg);
    }
    unsafe {
        keysor_macos_ui_update_speed_display(new_spd);
    }
    new_spd
}

#[unsafe(no_mangle)]
pub extern "C" fn keysor_macos_check_autostart_status() -> bool {
    let mut plist_path = std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("."));
    plist_path.push("Library");
    plist_path.push("LaunchAgents");
    plist_path.push("com.keysor.app.plist");
    plist_path.exists()
}

#[unsafe(no_mangle)]
pub extern "C" fn keysor_macos_on_autostart_toggle() -> bool {
    let current_status = keysor_macos_check_autostart_status();
    let new_status = !current_status;
    let _ = get_system_controller().register_startup(new_status);
    new_status
}

#[unsafe(no_mangle)]
pub extern "C" fn keysor_macos_check_is_snapped() -> bool {
    crate::platform::macos::magnet::is_currently_snapped()
}

#[unsafe(no_mangle)]
pub extern "C" fn keysor_macos_check_magnet_status() -> bool {
    if let Some(state_arc) = crate::hook::APP_STATE.get() {
        if let Ok(state) = state_arc.lock() {
            return state.config.settings.magnetic_mode.unwrap_or(false);
        }
    }
    false
}

#[unsafe(no_mangle)]
pub extern "C" fn keysor_macos_check_grid_status() -> bool {
    if let Some(state_arc) = crate::hook::APP_STATE.get() {
        if let Ok(state) = state_arc.lock() {
            return state.config.settings.pixel_mode.unwrap_or(false);
        }
    }
    false
}

#[unsafe(no_mangle)]
pub extern "C" fn keysor_macos_on_grid_toggle() -> bool {
    let mut new_status = false;
    let mut config_to_save = None;
    if let Some(state_arc) = crate::hook::APP_STATE.get() {
        if let Ok(mut state) = state_arc.lock() {
            let cur = state.config.settings.pixel_mode.unwrap_or(false);
            new_status = !cur;
            state.config.settings.pixel_mode = Some(new_status);
            config_to_save = Some(state.config.clone());
        }
    }
    if let Some(cfg) = config_to_save {
        crate::config::save_config(&cfg);
    }
    new_status
}

#[unsafe(no_mangle)]
pub extern "C" fn keysor_macos_on_magnet_toggle() -> bool {
    let mut new_status = false;
    let mut config_to_save = None;
    if let Some(state_arc) = crate::hook::APP_STATE.get() {
        if let Ok(mut state) = state_arc.lock() {
            let cur = state.config.settings.magnetic_mode.unwrap_or(false);
            new_status = !cur;
            state.config.settings.magnetic_mode = Some(new_status);
            state.config.settings.global_magnetic_mode = Some(new_status);
            config_to_save = Some(state.config.clone());
        }
    }
    if let Some(cfg) = config_to_save {
        crate::config::save_config(&cfg);
    }
    new_status
}

static IS_VISIBLE: AtomicBool = AtomicBool::new(false);

pub struct MacosDummyUi;

impl MacosDummyUi {
    pub fn new() -> Self {
        MacosDummyUi
    }
}

impl KeysorUi for MacosDummyUi {
    fn start(&self) -> Result<(), String> {
        unsafe {
            keysor_macos_ui_init();
        }

        // 60Hz(16ms) 주기로 커서 오버레이 위치를 실시간 갱신하는 루프 스레드 기동
        std::thread::spawn(|| loop {
            std::thread::sleep(std::time::Duration::from_millis(16));
            crate::indicator::update_indicator_position();
        });

        println!("[UI] macOS Native ObjC Custom Cursor & HUD Popup Window initialized successfully.");
        Ok(())
    }

    fn show(&self, visible: bool) {
        IS_VISIBLE.store(visible, Ordering::SeqCst);
        unsafe {
            keysor_macos_ui_show(visible);
        }
        if visible {
            self.update_position();
        }
    }

    fn update_position(&self) {
        if !IS_VISIBLE.load(Ordering::Relaxed) {
            return;
        }
        let (cx, cy) = get_system_controller().get_cursor_pos();
        unsafe {
            keysor_macos_ui_update_pos(cx as f64, cy as f64);
        }
    }

    fn trigger_click_motion(&self, click_type: ClickType) {
        if !IS_VISIBLE.load(Ordering::Relaxed) {
            return;
        }

        let state_val = match click_type {
            ClickType::Left => 1,
            ClickType::Right => 2,
            ClickType::Scroll => 3,
            ClickType::None => 0,
        };

        unsafe {
            keysor_macos_ui_set_click_motion(state_val);
        }
    }

    fn check_magnetic_snapping(&self) {
        crate::platform::macos::magnet::check_macos_magnetic_snapping(false);
    }

    fn check_global_magnetic_snapping(&self, is_moving: bool) {
        crate::platform::macos::magnet::check_macos_global_magnetic_snapping(is_moving);
    }

    fn is_currently_snapped(&self) -> bool {
        crate::platform::macos::magnet::is_currently_snapped()
    }
}
