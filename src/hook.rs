use crate::config::{self, Config};
use crate::platform::{get_system_controller, create_keyboard_hook, KeyboardHook, KeyEvent, HookResult};
use std::collections::{HashSet, HashMap};
use std::sync::{Arc, Mutex, OnceLock};
use std::sync::atomic::Ordering;
use std::thread;
use std::time::{Duration, Instant};

const VK_CAPITAL: u32 = 0x14;
const VK_SPACE: u32 = 0x20;
const VK_Q: u32 = 0x51;
const VK_E: u32 = 0x45;
const VK_J: u32 = 0x4A;
const VK_K: u32 = 0x4B;

/// 마우스 이동 제어를 위해 추출한 설정 파라미터 구조체
pub struct MovementSettings {
    pub dx: f64,
    pub dy: f64,
    pub movement_start_time: Option<Instant>,
    pub pixel_mode: bool,
    pub base_speed: f64,
    pub max_speed: f64,
    pub acceleration: f64,
    pub shift_pressed: bool,
}

/// 마우스 스크롤 제어를 위해 추출한 설정 파라미터 구조체
pub struct ScrollSettings {
    pub sdx: f64,
    pub sdy: f64,
    pub scroll_start_time: Option<Instant>,
}

/// 전역 공유 앱 상태
pub struct AppState {
    pub config: Config,
    pub vk_bindings: HashMap<u32, String>,
    pub is_mouse_mode: bool,
    pub is_pro: bool,
    pub is_trial: bool,
    // Caps Lock 홀드 및 대소문자 전환용 타이머
    pub caps_lock_press_time: Option<Instant>,
    pub caps_lock_used_as_modifier: bool,
    // 스페이스바 조작 제어용 상태
    pub space_press_time: Option<Instant>,
    pub is_space_down: bool,
    pub is_dragging: bool,
    pub space_tap_count: u32,
    pub space_last_tap_time: Option<Instant>,
    // 현재 눌려 있는 마우스 조작 이동 키 (이동 스레드용)
    pub active_movement_keys: HashSet<u32>,
    pub movement_start_time: Option<Instant>,
    // 현재 눌려 있는 마우스 스크롤 키 (스크롤 가속용)
    pub active_scroll_keys: HashSet<u32>,
    pub scroll_start_time: Option<Instant>,
    // 모디파이어 단축키 판별용 물리 키 상태
    pub ctrl_pressed: bool,
    pub alt_pressed: bool,
    pub win_pressed: bool,
    pub shift_pressed: bool,
}

impl AppState {
    /// 마우스 조작 모드가 활성화되어 있고 이동 키가 입력되었을 때, 이동 관련 파라미터를 파싱합니다.
    pub fn get_movement_settings(&self) -> Option<MovementSettings> {
        if !self.is_mouse_mode || self.active_movement_keys.is_empty() {
            return None;
        }

        let mut dx = 0.0;
        let mut dy = 0.0;
        for &vk in &self.active_movement_keys {
            if let Some(action) = self.vk_bindings.get(&vk) {
                match action.as_str() {
                    "MouseMoveUp" => dy -= 1.0,
                    "MouseMoveDown" => dy += 1.0,
                    "MouseMoveLeft" => dx -= 1.0,
                    "MouseMoveRight" => dx += 1.0,
                    _ => {}
                }
            }
        }

        let settings = &self.config.settings;
        let (base_speed, max_speed, acceleration) = if self.is_pro || self.is_trial {
            (settings.base_speed, settings.max_speed, settings.acceleration)
        } else {
            (1.5, 30.0, 1.5)
        };

        Some(MovementSettings {
            dx,
            dy,
            movement_start_time: self.movement_start_time,
            pixel_mode: settings.pixel_mode.unwrap_or(false),
            base_speed,
            max_speed,
            acceleration,
            shift_pressed: self.shift_pressed,
        })
    }

    /// 마우스 조작 모드가 활성화되어 있고 스크롤 키가 입력되었을 때, 스크롤 관련 파라미터를 파싱합니다.
    pub fn get_scroll_settings(&self) -> Option<ScrollSettings> {
        if !self.is_mouse_mode || self.active_scroll_keys.is_empty() {
            return None;
        }

        let mut sdx = 0.0;
        let mut sdy = 0.0;
        for &vk in &self.active_scroll_keys {
            if let Some(action) = self.vk_bindings.get(&vk) {
                match action.as_str() {
                    "MouseScrollUp" => sdy += 1.0,
                    "MouseScrollDown" => sdy -= 1.0,
                    "MouseScrollLeft" => sdx -= 1.0,
                    "MouseScrollRight" => sdx += 1.0,
                    _ => {}
                }
            }
        }

        Some(ScrollSettings {
            sdx,
            sdy,
            scroll_start_time: self.scroll_start_time,
        })
    }

    /// 모디파이어 키의 눌림 상태를 갱신합니다.
    pub fn update_modifier_key_state(&mut self, vk_code: u32, is_keydown: bool) -> bool {
        match vk_code {
            0x10 | 0xA0 | 0xA1 => { // VK_SHIFT, VK_LSHIFT, VK_RSHIFT
                self.shift_pressed = is_keydown;
                !self.vk_bindings.contains_key(&vk_code)
            }
            0x11 | 0xA2 | 0xA3 => { // VK_CONTROL, VK_LCONTROL, VK_RCONTROL
                self.ctrl_pressed = is_keydown;
                true
            }
            0x12 | 0xA4 | 0xA5 => { // VK_MENU, VK_LMENU, VK_RMENU
                self.alt_pressed = is_keydown;
                true
            }
            0x5B | 0x5C => { // VK_LWIN, VK_RWIN
                self.win_pressed = is_keydown;
                true
            }
            _ => false,
        }
    }

    /// 시스템 기본 단축키 조합이 눌려 있는지 여부를 확인합니다.
    pub fn is_system_shortcut_active(&self) -> bool {
        self.ctrl_pressed || self.alt_pressed || self.win_pressed
    }

    /// 마우스 조작 모드를 비활성화하고 관련된 모든 상태(이동, 스크롤, 드래그)를 초기화합니다.
    pub fn deactivate_mouse_mode(&mut self) {
        self.is_mouse_mode = false;
        self.active_movement_keys.clear();
        self.movement_start_time = None;
        self.active_scroll_keys.clear();
        self.scroll_start_time = None;
        if self.is_dragging {
            get_system_controller().left_up();
            self.is_dragging = false;
        }
    }

    /// keysor.yaml의 실시간 변경 사항을 라이선스 인증 수준에 맞춰 안전하게 재로드 및 동적 갱신합니다.
    pub fn reload_configuration(&mut self, new_config: Config) {
        let is_pro_now = crate::license::check_local_license();
        let is_trial_now = crate::license::check_trial_status();
        self.is_pro = is_pro_now;
        self.is_trial = is_trial_now;

        let features_enabled = is_pro_now || is_trial_now;

        // 설정 파일 변경을 통한 신규 라이선스 키 기입 시 실시간 백그라운드 재활성화
        if let Some(ref new_key) = new_config.settings.license_key {
            if !new_key.trim().is_empty() && !crate::license::check_local_license() {
                crate::license::start_auto_activation_worker(new_key.clone());
            }
        }

        // Pro 버전 여부에 따른 강제 바인딩 정책
        self.vk_bindings = if features_enabled {
            config::get_vk_bindings(&new_config)
        } else {
            config::get_vk_bindings(&config::Config::default())
        };

        self.config = if features_enabled {
            new_config
        } else {
            // Free 버전인 경우 감도, 키바인딩, 자석 스냅 설정을 디폴트로 강제 오버라이드
            let mut forced = new_config.clone();
            forced.bindings = config::Config::default().bindings;
            forced.settings.base_speed = 1.5;
            forced.settings.max_speed = 30.0;
            forced.settings.acceleration = 1.5;
            forced.settings.magnetic_mode = Some(false);
            forced.settings.global_magnetic_mode = Some(false);
            forced
        };
        println!("[Hot-Reload] New configuration evaluated and applied. Pro features: {}.", features_enabled);
    }
}

pub static APP_STATE: OnceLock<Arc<Mutex<AppState>>> = OnceLock::new();
static KEYBOARD_HOOK: OnceLock<Box<dyn KeyboardHook>> = OnceLock::new();

/// 마우스 조작 모드 활성화 시 WASD 방향 입력을 누적하여 상대적 이동 델타를 갱신합니다.
fn process_mouse_movement(
    state_ptr: &Arc<Mutex<AppState>>,
    dpi_scale: f64,
    remainder_x: &mut f64,
    remainder_y: &mut f64,
) {
    let movement_params = {
        let state = state_ptr.lock().unwrap();
        state.get_movement_settings()
    };

    if let Some(params) = movement_params {
        let start_time = match params.movement_start_time {
            Some(t) => t,
            None => {
                let now = Instant::now();
                let mut state = state_ptr.lock().unwrap();
                if state.movement_start_time.is_none() {
                    state.movement_start_time = Some(now);
                }
                state.movement_start_time.unwrap_or(now)
            }
        };
        let elapsed = start_time.elapsed().as_secs_f64();

        let mut speed = crate::math::calculate_speed(params.base_speed, elapsed, params.acceleration, params.max_speed);
        if params.shift_pressed {
            speed /= 4.0;
        }
        let (move_x, move_y) = crate::math::calculate_movement_delta(
            speed,
            params.dx,
            params.dy,
            params.pixel_mode,
            dpi_scale,
            remainder_x,
            remainder_y,
        );

        if !crate::indicator::is_currently_snapped() {
            get_system_controller().move_relative(move_x, move_y);
        }
    } else {
        {
            let mut state = state_ptr.lock().unwrap();
            if state.movement_start_time.is_some() {
                state.movement_start_time = None;
            }
        }
        *remainder_x = 0.0;
        *remainder_y = 0.0;
    }
}

/// 마우스 조작 모드 활성화 시 스크롤 키 입력을 가속 처리하여 마우스 휠 동작을 에뮬레이션합니다.
fn process_mouse_scrolling(
    state_ptr: &Arc<Mutex<AppState>>,
    dt: f64,
    remainder_scroll_x: &mut f64,
    remainder_scroll_y: &mut f64,
) {
    let scroll_params = {
        let state = state_ptr.lock().unwrap();
        state.get_scroll_settings()
    };

    if let Some(params) = scroll_params {
        let start_time = match params.scroll_start_time {
            Some(t) => t,
            None => {
                let now = Instant::now();
                let mut state = state_ptr.lock().unwrap();
                if state.scroll_start_time.is_none() {
                    state.scroll_start_time = Some(now);
                }
                state.scroll_start_time.unwrap_or(now)
            }
        };
        let elapsed = start_time.elapsed().as_secs_f64();

        // 200ms 지연 후 지속적 가속 스크롤 활성화 (단일 탭 지원용)
        if elapsed >= 0.200 {
            let scroll_elapsed = elapsed - 0.200;
            
            let base_scroll = 720.0; // 초당 6 notches (720 delta/sec)
            let max_scroll = 7200.0; // 초당 60 notches (7200 delta/sec)
            let accel_factor = 2.0;
            let speed = (base_scroll + scroll_elapsed * accel_factor * 600.0).min(max_scroll);
            
            let scroll_amount_y = speed * params.sdy * dt + *remainder_scroll_y;
            let scroll_amount_x = speed * params.sdx * dt + *remainder_scroll_x;
            
            let sy = scroll_amount_y.round() as i32;
            let sx = scroll_amount_x.round() as i32;
            
            *remainder_scroll_y = scroll_amount_y - sy as f64;
            *remainder_scroll_x = scroll_amount_x - sx as f64;
            
            if sy != 0 {
                get_system_controller().scroll(sy);
            }
            if sx != 0 {
                get_system_controller().scroll_horizontal(sx);
            }
        } else {
            *remainder_scroll_y = 0.0;
            *remainder_scroll_x = 0.0;
        }
    } else {
        {
            let mut state = state_ptr.lock().unwrap();
            if state.scroll_start_time.is_some() {
                state.scroll_start_time = None;
            }
        }
        *remainder_scroll_y = 0.0;
        *remainder_scroll_x = 0.0;
    }
}

/// 모니터 주사율에 독립적인 극도로 부드러운 이동(100Hz) 및 스크롤 가속을 처리하는 백그라운드 스레드
fn start_movement_thread(state_ptr: Arc<Mutex<AppState>>) {
    thread::spawn(move || {
        let dpi_scale = get_system_controller().get_dpi_scale();
        let mut interval_ms = 10;
        let mut remainder_x = 0.0f64;
        let mut remainder_y = 0.0f64;
        let mut remainder_scroll_x = 0.0f64;
        let mut remainder_scroll_y = 0.0f64;
        let mut was_mouse_mode = false;
        loop {
            thread::sleep(Duration::from_millis(interval_ms));

            // 1. 마우스 이동 연산 분리 호출
            process_mouse_movement(&state_ptr, dpi_scale, &mut remainder_x, &mut remainder_y);

            let (is_mouse_mode, hz) = {
                let state = state_ptr.lock().unwrap();
                (state.is_mouse_mode, state.config.settings.refresh_rate_hz.unwrap_or(100))
            };

            // 2. 인디케이터 스냅 및 실시간 위치 갱신 연산 복원
            if is_mouse_mode {
                crate::indicator::update_indicator_position();
                crate::indicator::check_magnetic_snapping();
                crate::indicator::check_global_magnetic_snapping();
                was_mouse_mode = true;
            } else {
                if was_mouse_mode {
                    crate::ui::win_gdi::force_restore_system_cursor();
                    was_mouse_mode = false;
                }
            }

            let safe_hz = hz.max(10).min(1000);
            interval_ms = 1000 / safe_hz;
            let dt = interval_ms as f64 / 1000.0;

            // 3. 마우스 스크롤 연산 분리 호출
            process_mouse_scrolling(&state_ptr, dt, &mut remainder_scroll_x, &mut remainder_scroll_y);
        }
    });
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PendingMouseAction {
    None,
    LeftClick,
    RightClick,
    ScrollUp,
    ScrollDown,
    ScrollLeft,
    ScrollRight,
    LeftDoubleClick,
    BrowserBack,
    BrowserForward,
    VirtualDesktopLeft,
    VirtualDesktopRight,
    TabLeft,
    TabRight,
    RunApp(String),
}

/// 다양한 마우스 제어 액션 및 단축키 실행을 단일 창구로 모아 처리합니다.
fn execute_pending_action(action: PendingMouseAction) {
    match action {
        PendingMouseAction::LeftClick => {
            get_system_controller().left_click();
            crate::indicator::trigger_click_motion(crate::indicator::ClickType::Left);
            crate::indicator::FORCE_UIA_REFRESH.store(true, std::sync::atomic::Ordering::SeqCst);
        }
        PendingMouseAction::LeftDoubleClick => {
            get_system_controller().left_double_click();
            crate::indicator::trigger_click_motion(crate::indicator::ClickType::Left);
            crate::indicator::FORCE_UIA_REFRESH.store(true, std::sync::atomic::Ordering::SeqCst);
        }
        PendingMouseAction::RightClick => {
            get_system_controller().right_click();
            crate::indicator::trigger_click_motion(crate::indicator::ClickType::Right);
            crate::indicator::FORCE_UIA_REFRESH.store(true, std::sync::atomic::Ordering::SeqCst);
        }
        PendingMouseAction::ScrollUp => {
            get_system_controller().scroll(120);
            crate::indicator::trigger_click_motion(crate::indicator::ClickType::Scroll);
        }
        PendingMouseAction::ScrollDown => {
            get_system_controller().scroll(-120);
            crate::indicator::trigger_click_motion(crate::indicator::ClickType::Scroll);
        }
        PendingMouseAction::ScrollLeft => {
            get_system_controller().scroll_horizontal(-120);
            crate::indicator::trigger_click_motion(crate::indicator::ClickType::Scroll);
        }
        PendingMouseAction::ScrollRight => {
            get_system_controller().scroll_horizontal(120);
            crate::indicator::trigger_click_motion(crate::indicator::ClickType::Scroll);
        }
        PendingMouseAction::BrowserBack => get_system_controller().simulate_browser_navigation(false),
        PendingMouseAction::BrowserForward => get_system_controller().simulate_browser_navigation(true),
        PendingMouseAction::VirtualDesktopLeft => get_system_controller().simulate_virtual_desktop_navigation(false),
        PendingMouseAction::VirtualDesktopRight => get_system_controller().simulate_virtual_desktop_navigation(true),
        PendingMouseAction::TabLeft => get_system_controller().simulate_tab_navigation(false),
        PendingMouseAction::TabRight => get_system_controller().simulate_tab_navigation(true),
        PendingMouseAction::RunApp(app_path) => {
            get_system_controller().run_app(&app_path).ok();
        }
        _ => {}
    }
}

/// 스페이스바 3단 탭 및 홀드 감지 타이머 비동기 루프
fn handle_space_release(state_ptr: Arc<Mutex<AppState>>) {
    let state_clone = Arc::clone(&state_ptr);
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_millis(15));

            let (done, pending_action) = {
                let mut state = state_clone.lock().unwrap();
                if state.space_tap_count == 0 {
                    (true, PendingMouseAction::None)
                } else if let Some(last_time) = state.space_last_tap_time {
                    let threshold = state.config.settings.double_click_threshold_ms;
                    if last_time.elapsed() >= Duration::from_millis(threshold) {
                        println!("[Debug] handle_space_release: tap_count={}", state.space_tap_count);
                        let action = match state.space_tap_count {
                            1 => PendingMouseAction::LeftClick,
                            _ => PendingMouseAction::LeftDoubleClick,
                        };
                        state.space_tap_count = 0;
                        state.space_last_tap_time = None;
                        (true, action)
                    } else {
                        (false, PendingMouseAction::None)
                    }
                } else {
                    (true, PendingMouseAction::None)
                }
            };

            if done {
                execute_pending_action(pending_action);
                break;
            }
        }
    });
}

/// Caps Lock 관련 모드 전환 및 동기화 이벤트를 처리합니다.
fn process_caps_lock(event: &KeyEvent) -> Option<HookResult> {
    if event.vk_code != VK_CAPITAL {
        return None;
    }

    if event.is_injected_by_keysor {
        return Some(HookResult::Pass);
    }

    let mut should_inject = false;
    let mut action_to_take = 0; // 0: None, 1: Show, 2: Hide

    if let Some(state_arc) = APP_STATE.get() {
        let mut state = state_arc.lock().unwrap();
        let is_toggle_mode = state.config.settings.modifier_mode.eq_ignore_ascii_case("Toggle");

        if event.is_keydown {
            let should_process = match state.caps_lock_press_time {
                None => true,
                Some(t) => t.elapsed() > Duration::from_millis(200),
            };

            if should_process {
                state.caps_lock_press_time = Some(Instant::now());
                state.caps_lock_used_as_modifier = false;
                
                if is_toggle_mode {
                    if state.is_mouse_mode {
                        state.deactivate_mouse_mode();
                        action_to_take = 2; // Hide
                    } else {
                        state.is_mouse_mode = true;
                        action_to_take = 1; // Show
                    }
                } else {
                    state.is_mouse_mode = true;
                    action_to_take = 1; // Show
                }
            }
        } else if event.is_keyup {
            let elapsed = state.caps_lock_press_time.map_or(Duration::ZERO, |t| t.elapsed());
            let used_modifier = state.caps_lock_used_as_modifier;
            
            state.caps_lock_press_time = None;

            if !is_toggle_mode {
                state.deactivate_mouse_mode();
                action_to_take = 2; // Hide
            }

            if elapsed < Duration::from_millis(250) && !used_modifier {
                should_inject = true;
            }
        }
    }

    match action_to_take {
        1 => crate::indicator::show_indicator(),
        2 => crate::indicator::hide_indicator(),
        _ => {}
    }

    if should_inject {
        get_system_controller().inject_caps_lock_toggle();
    }

    Some(HookResult::Block)
}

/// 비동기 스페이스 드래그 홀드 임계값 감지 스레드를 기동합니다.
fn spawn_drag_detection_thread(state_arc: Arc<Mutex<AppState>>, hold_ms: u64) {
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(hold_ms));

        let mut trigger_drag = false;
        {
            let mut s = state_arc.lock().unwrap();
            if s.is_space_down && !s.is_dragging {
                if let Some(press_t) = s.space_press_time {
                    if press_t.elapsed() >= Duration::from_millis(hold_ms) {
                        s.is_dragging = true;
                        s.space_tap_count = 0;
                        trigger_drag = true;
                    }
                }
            }
        }
        if trigger_drag {
            println!("[Debug] Space drag hold threshold reached. Triggering left_down()");
            get_system_controller().left_down();
        }
    });
}

/// 스페이스바를 활용한 좌클릭/더블클릭/드래그앤드롭 기능을 에뮬레이션합니다.
fn handle_space_keydown(state: &mut AppState, state_arc: &Arc<Mutex<AppState>>) {
    if !state.is_space_down {
        state.is_space_down = true;
        state.space_press_time = Some(Instant::now());

        let hold_ms = state.config.settings.drag_hold_threshold_ms;
        spawn_drag_detection_thread(Arc::clone(state_arc), hold_ms);
    }
}

fn handle_space_keyup(
    state: &mut AppState,
    hold_ms: u64,
    trigger_up: &mut bool,
    should_start_release_handler: &mut bool,
) {
    state.is_space_down = false;
    let elapsed = state.space_press_time.map_or(Duration::ZERO, |t| t.elapsed());
    state.space_press_time = None;

    if state.is_dragging {
        state.is_dragging = false;
        *trigger_up = true;
    } else if elapsed < Duration::from_millis(hold_ms) {
        state.space_tap_count += 1;
        state.space_last_tap_time = Some(Instant::now());
        if state.space_tap_count == 1 {
            *should_start_release_handler = true;
        }
    }
}

/// 스페이스바를 활용한 좌클릭/더블클릭/드래그앤드롭 기능을 에뮬레이션합니다.
fn process_space_click(
    state_arc: &Arc<Mutex<AppState>>,
    event: &KeyEvent,
) -> HookResult {
    let mut trigger_up = false;
    let mut should_start_release_handler = false;

    {
        let mut state = state_arc.lock().unwrap();
        state.caps_lock_used_as_modifier = true;

        if event.is_keydown {
            println!("[Debug] Space Keydown: is_space_down={}, is_dragging={}", state.is_space_down, state.is_dragging);
            handle_space_keydown(&mut state, state_arc);
        } else if event.is_keyup {
            println!("[Debug] Space Keyup: space_tap_count={}, is_dragging={}", state.space_tap_count, state.is_dragging);
            let hold_ms = state.config.settings.drag_hold_threshold_ms;
            handle_space_keyup(&mut state, hold_ms, &mut trigger_up, &mut should_start_release_handler);
        }
    }

    if trigger_up {
        println!("[Debug] Space drag hold released. Triggering left_up()");
        get_system_controller().left_up();
    }

    if should_start_release_handler {
        handle_space_release(Arc::clone(state_arc));
    }

    HookResult::Block
}

/// 마우스 조작 모드 활성화 상태에서 일반 방향 이동 및 클릭 액션을 수행합니다.
fn process_movement_and_actions(
    state: &mut AppState,
    vk_code: u32,
    is_keydown: bool,
    is_keyup: bool,
) -> (HookResult, PendingMouseAction) {
    let mut pending_action = PendingMouseAction::None;

    // Shift key combinations (Shift + Q = BrowserBack, Shift + E = BrowserForward)
    if state.shift_pressed {
        if vk_code == VK_Q {
            if is_keydown {
                pending_action = PendingMouseAction::BrowserBack;
            }
            return (HookResult::Block, pending_action);
        } else if vk_code == VK_E {
            if is_keydown {
                pending_action = PendingMouseAction::BrowserForward;
            }
            return (HookResult::Block, pending_action);
        } else if vk_code == VK_J {
            if is_keydown {
                pending_action = PendingMouseAction::VirtualDesktopLeft;
            }
            return (HookResult::Block, pending_action);
        } else if vk_code == VK_K {
            if is_keydown {
                pending_action = PendingMouseAction::VirtualDesktopRight;
            }
            return (HookResult::Block, pending_action);
        }
    } else {
        if vk_code == VK_J {
            if is_keydown {
                if state.is_pro || state.is_trial {
                    pending_action = PendingMouseAction::TabLeft;
                } else {
                    get_system_controller().beep();
                    println!("[License] J/K Tab switching is a Pro-only feature.");
                }
            }
            return (HookResult::Block, pending_action);
        } else if vk_code == VK_K {
            if is_keydown {
                if state.is_pro || state.is_trial {
                    pending_action = PendingMouseAction::TabRight;
                } else {
                    get_system_controller().beep();
                    println!("[License] J/K Tab switching is a Pro-only feature.");
                }
            }
            return (HookResult::Block, pending_action);
        }
    }

    // 일반 이동 및 보조 클릭 가로채기 매칭
    if let Some(action) = state.vk_bindings.get(&vk_code) {
        state.caps_lock_used_as_modifier = true;
        crate::ui::win_gdi::SUSPEND_CURSOR_HIDE.store(false, Ordering::SeqCst);

        if is_keydown {
            if action.starts_with("MouseMove") {
                state.active_movement_keys.insert(vk_code);
            } else if action.starts_with("MouseScroll") {
                if state.active_scroll_keys.insert(vk_code) {
                    if state.scroll_start_time.is_none() {
                        state.scroll_start_time = Some(Instant::now());
                    }
                    match action.as_str() {
                        "MouseScrollUp" => pending_action = PendingMouseAction::ScrollUp,
                        "MouseScrollDown" => pending_action = PendingMouseAction::ScrollDown,
                        "MouseScrollLeft" => pending_action = PendingMouseAction::ScrollLeft,
                        "MouseScrollRight" => pending_action = PendingMouseAction::ScrollRight,
                        _ => {}
                    }
                }
            } else {
                match action.as_str() {
                    "MouseLeftClick" => pending_action = PendingMouseAction::LeftClick,
                    "MouseRightClick" => pending_action = PendingMouseAction::RightClick,
                    other => {
                        if other.starts_with("RunApp:") {
                            if state.is_pro || state.is_trial {
                                let app_path = other.trim_start_matches("RunApp:").trim().to_string();
                                pending_action = PendingMouseAction::RunApp(app_path);
                            } else {
                                get_system_controller().beep();
                                println!("[License] RunApp shortcut is a Pro-only feature.");
                            }
                        }
                    }
                }
            }
        } else if is_keyup {
            if action.starts_with("MouseMove") {
                state.active_movement_keys.remove(&vk_code);
            } else if action.starts_with("MouseScroll") {
                state.active_scroll_keys.remove(&vk_code);
                if state.active_scroll_keys.is_empty() {
                    state.scroll_start_time = None;
                }
            }
        }
        (HookResult::Block, pending_action)
    } else {
        (HookResult::Block, PendingMouseAction::None)
    }
}

/// 키서 모드 중에도 강제 차단하지 않고 OS에 투과(Pass)할 키들을 판별합니다.
/// 숫자키 전체, 기능키 전체, 탐색 및 편집키가 포함됩니다.
fn is_allowed_pass_through_key(vk_code: u32) -> bool {
    match vk_code {
        // Enter (0x0D), Backspace (0x08)
        0x0D | 0x08 => true,
        // Tab (0x09), Esc (0x1B), Pause (0x13)
        0x09 | 0x1B | 0x13 => true,
        // Page Up (0x21), Page Down (0x22), End (0x23), Home (0x24), Left (0x25), Up (0x26), Right (0x27), Down (0x28)
        0x21..=0x28 => true,
        // Print Screen (0x2C), Insert (0x2D), Delete (0x2E)
        0x2C | 0x2D | 0x2E => true,
        // 숫자 키 0-9 (0x30..=0x39)
        0x30..=0x39 => true,
        // 키패드 숫자 키 0-9 (0x60..=0x69)
        0x60..=0x69 => true,
        // 기능 키 F1-F12 (0x70..=0x7B)
        0x70..=0x7B => true,
        // Scroll Lock (0x91)
        0x91 => true,
        // 특수 키: , (0xBC), . (0xBE), / (0xBF), ; (0xBA), ' (0xDE), [ (0xDB), ] (0xDD)
        0xBA | 0xBC | 0xBE | 0xBF | 0xDB | 0xDD | 0xDE => true,
        _ => false,
    }
}

/// 공통 키보드 훅 이벤트 리시버 콜백
fn handle_keyboard_event(event: KeyEvent) -> HookResult {
    if event.is_injected_by_keysor {
        return HookResult::Pass;
    }

    if let Some(result) = process_caps_lock(&event) {
        return result;
    }

    if let Some(state_arc) = APP_STATE.get() {
        let is_modifier_key = {
            let mut state = state_arc.lock().unwrap();
            state.update_modifier_key_state(event.vk_code, event.is_keydown)
        };

        // Win+Tab, Alt+Tab 단축키 시작 감지 및 강제 커서 복원
        if event.vk_code == 0x09 && event.is_keydown {
            if let Ok(state) = state_arc.try_lock() {
                if state.win_pressed || state.alt_pressed {
                    crate::ui::win_gdi::SUSPEND_CURSOR_HIDE.store(true, Ordering::SeqCst);
                    unsafe {
                        crate::ui::win_gdi::force_restore_system_cursor();
                    }
                }
            }
        }

        // 일반 키 입력 시 화면 전환 일시정지 해제
        if event.is_keydown && event.vk_code != 0x09 
            && event.vk_code != 0x5B && event.vk_code != 0x5C 
            && event.vk_code != 0x12 && event.vk_code != 0xA4 && event.vk_code != 0xA5
            && event.vk_code != 0x11 && event.vk_code != 0xA2 && event.vk_code != 0xA3 {
            crate::ui::win_gdi::SUSPEND_CURSOR_HIDE.store(false, Ordering::SeqCst);
        }

        if is_modifier_key {
            return HookResult::Pass;
        }

        let (is_mouse_mode, is_shortcut) = {
            let state = state_arc.lock().unwrap();
            (state.is_mouse_mode, state.is_system_shortcut_active())
        };

        if is_mouse_mode {
            // Ctrl, Alt, Win 단축키 조합이 활성화된 경우 키서의 모든 조작을 바이패스(Pass)하여 기존 단축키가 원활히 작동하도록 합니다.
            if is_shortcut {
                return HookResult::Pass;
            }

            let is_bound = {
                let state = state_arc.lock().unwrap();
                state.vk_bindings.contains_key(&event.vk_code)
            };

            // 키서 단축키로 등록되지 않은 경우, 허용된 패스스루 키(숫자, 기능키, 편집키 등)라면 통과시킵니다.
            if !is_bound && is_allowed_pass_through_key(event.vk_code) {
                return HookResult::Pass;
            }

            // Space + Scroll (R/F/Q/E) 단축키 조합을 통한 맨 위/아래/좌/우 Page Jump 처리
            if event.vk_code == VK_SPACE && event.is_keydown {
                let mut page_jump_action = None; // Some(0): Up, Some(1): Down, Some(2): Left, Some(3): Right
                {
                    let state = state_arc.lock().unwrap();
                    for &vk in &state.active_scroll_keys {
                        if let Some(action) = state.vk_bindings.get(&vk) {
                            match action.as_str() {
                                "MouseScrollUp" => {
                                    page_jump_action = Some(0);
                                    break;
                                }
                                "MouseScrollDown" => {
                                    page_jump_action = Some(1);
                                    break;
                                }
                                "MouseScrollLeft" => {
                                    page_jump_action = Some(2);
                                    break;
                                }
                                "MouseScrollRight" => {
                                    page_jump_action = Some(3);
                                    break;
                                }
                                _ => {}
                            }
                        }
                    }
                }
                
                if let Some(action_type) = page_jump_action {
                    match action_type {
                        0 => {
                            println!("[Action] Space + Scroll Page Jump to Top");
                            get_system_controller().simulate_page_jump(true);
                        }
                        1 => {
                            println!("[Action] Space + Scroll Page Jump to Bottom");
                            get_system_controller().simulate_page_jump(false);
                        }
                        2 => {
                            println!("[Action] Space + Scroll Page Jump to Left");
                            get_system_controller().scroll_horizontal(-10000);
                        }
                        3 => {
                            println!("[Action] Space + Scroll Page Jump to Right");
                            get_system_controller().scroll_horizontal(10000);
                        }
                        _ => {}
                    }
                    return HookResult::Block;
                }
            }

            let unified_space = {
                let state = state_arc.lock().unwrap();
                state.config.settings.unified_space_click
            };
            if event.vk_code == VK_SPACE && unified_space {
                return process_space_click(state_arc, &event);
            }

            let (result, pending_action) = {
                let mut state = state_arc.lock().unwrap();
                process_movement_and_actions(&mut state, event.vk_code, event.is_keydown, event.is_keyup)
            };

            execute_pending_action(pending_action);

            return result;
        }
    }

    HookResult::Pass
}

/// 전역 키보드 훅 루프 시작 함수 (별도 독립 스레드에서 구동)
pub fn start_hook(config: Config, is_pro: bool, is_trial: bool) {
    let features_enabled = is_pro || is_trial;
    let vk_bindings = if features_enabled {
        config::get_vk_bindings(&config)
    } else {
        config::get_vk_bindings(&config::Config::default())
    };

    let ctrl_pressed = false;
    let alt_pressed = false;
    let win_pressed = false;
    let shift_pressed = false;

    let state = Arc::new(Mutex::new(AppState {
        config,
        vk_bindings,
        is_mouse_mode: false,
        is_pro,
        is_trial,
        caps_lock_press_time: None,
        caps_lock_used_as_modifier: false,
        space_press_time: None,
        is_space_down: false,
        is_dragging: false,
        space_tap_count: 0,
        space_last_tap_time: None,
        active_movement_keys: HashSet::new(),
        movement_start_time: None,
        active_scroll_keys: HashSet::new(),
        scroll_start_time: None,
        ctrl_pressed,
        alt_pressed,
        win_pressed,
        shift_pressed,
    }));

    APP_STATE.set(Arc::clone(&state)).ok();

    // 1. liquid smooth 커서 가속 이동용 스레드 시작
    start_movement_thread(Arc::clone(&state));

    // 2. OS 플랫폼 전역 키보드 훅 리스너 시작
    let hook = create_keyboard_hook();
    if let Err(e) = hook.start_listening(Box::new(handle_keyboard_event)) {
        eprintln!("[Error] Failed to start keyboard hook listener: {}", e);
    } else {
        KEYBOARD_HOOK.set(hook).ok();
    }
}

/// 훅 자원을 OS에 안전하게 반환하는 페일세이프 해제 함수
pub fn cleanup_hook() {
    if let Some(hook) = KEYBOARD_HOOK.get() {
        hook.stop_listening();
    }
    // Restore system cursor on exit if it was hidden
    crate::ui::win_gdi::restore_system_cursor();
}

/// 포커스 전환 및 강제 상황 시 CapsLock 물리 눌림 상태를 확인하는 동기화 가드
pub fn modifier_sync_guard() {
    if let Some(state_arc) = APP_STATE.get() {
        let (is_mouse_mode, is_toggle_mode) = {
            let state = state_arc.lock().unwrap();
            (state.is_mouse_mode, state.config.settings.modifier_mode.eq_ignore_ascii_case("Toggle"))
        };

        if let Some(hook) = KEYBOARD_HOOK.get() {
            let on_deactivate = || {
                if let Some(state_arc) = APP_STATE.get() {
                    let mut state = state_arc.lock().unwrap();
                    state.deactivate_mouse_mode();
                }
                crate::indicator::hide_indicator();
            };
            hook.modifier_sync_guard(is_mouse_mode, is_toggle_mode, on_deactivate);
        }
    }
}
