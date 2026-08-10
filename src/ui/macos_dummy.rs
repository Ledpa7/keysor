use crate::ui::{KeysorUi, ClickType};
use crate::platform::get_system_controller;
use std::sync::atomic::{AtomicBool, Ordering};

unsafe extern "C" {
    fn keysor_macos_ui_init();
    fn keysor_macos_ui_show(visible: bool);
    fn keysor_macos_ui_update_pos(x: f64, y: f64);
    fn keysor_macos_ui_set_click_motion(clickType: i32);
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

        println!("[UI] macOS Native ObjC Custom Cursor Overlay Window initialized successfully.");
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

        // 120ms 후 기본 네온 그린 상태로 복구 스레드
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(120));
            unsafe {
                keysor_macos_ui_set_click_motion(0);
            }
        });
    }

    fn check_magnetic_snapping(&self) {}

    fn check_global_magnetic_snapping(&self) {}

    fn is_currently_snapped(&self) -> bool {
        false
    }
}
