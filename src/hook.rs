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

/// CapsLock 이벤트 처리 후 인디케이터에 전달할 액션
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IndicatorAction {
    None,
    Show,
    Hide,
}

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
    pub space_scroll_combo: bool,
    pub is_right_down: bool,
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
    pub last_input_time: Instant,
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
            (settings.base_speed, 30.0, 1.5)
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
    fn release_tracked_key(&mut self, vk: u32) {
        self.active_movement_keys.remove(&vk);
        self.active_scroll_keys.remove(&vk);
        if self.active_movement_keys.is_empty() { self.movement_start_time = None; }
        if self.active_scroll_keys.is_empty() { self.scroll_start_time = None; }
        if self.vk_bindings.get(&vk).is_some_and(|a| a == "MouseRightClick") {
            self.is_right_down = false;
        }
    }

    fn begin_space_scroll_combo(&mut self) {
        self.space_scroll_combo = true;
        self.space_press_time = None;
        self.space_tap_count = 0;
        self.space_last_tap_time = None;
        if self.is_dragging {
            self.is_dragging = false;
            execute_pending_action(PendingMouseAction::LeftUp);
        }
    }

    pub fn deactivate_mouse_mode(&mut self) {
        self.is_mouse_mode = false;
        self.is_space_down = false;
        self.space_scroll_combo = false;
        self.space_press_time = None;
        self.space_tap_count = 0;
        self.space_last_tap_time = None;
        self.is_right_down = false;
        self.active_movement_keys.clear();
        self.movement_start_time = None;
        self.active_scroll_keys.clear();
        self.scroll_start_time = None;
        if self.is_dragging {
            execute_pending_action(PendingMouseAction::LeftUp);
            self.is_dragging = false;
        }
    }

    /// keysor.yaml의 실시간 변경 사항을 라이선스 인증 수준에 맞춰 안전하게 재로드 및 동적 갱신합니다.
    pub fn apply_file_configuration(&mut self, new_config: Config) {
        // HUD changes are already applied in memory before saving the file.
        // Do not deactivate input when the file watcher sees that same save.
        if self.config != new_config {
            self.reload_configuration(new_config);
        }
    }

    pub fn reload_configuration(&mut self, new_config: Config) {
        let is_pro_now = crate::license::check_local_license();
        let is_trial_now = crate::license::check_trial_status();
        self.is_pro = is_pro_now;
        self.is_trial = is_trial_now;

        let features_enabled = is_pro_now || is_trial_now;

        // 설정 파일 변경을 통한 신규 라이선스 키 기입 시 실시간 백그라운드 재활성화
        if let Some(ref new_key) = new_config.settings.license_key {
            if self.config.settings.license_key != new_config.settings.license_key
                && !new_key.trim().is_empty() && !crate::license::check_local_license() {
                crate::license::start_auto_activation_worker(new_key.clone());
            }
        }

        // Pro 버전 여부에 따른 강제 바인딩 정책
        self.vk_bindings = if features_enabled {
            config::get_vk_bindings(&new_config)
        } else {
            config::get_vk_bindings(&config::Config::default())
        };

        self.deactivate_mouse_mode();
        self.caps_lock_press_time = None;
        self.config = if features_enabled {
            new_config
        } else {
            // Free 버전인 경우 고급 기능(키바인딩, 최고속도/가속도)을 디폴트로 강제 오버라이드
            // 기본 시작 속도(base_speed) 및 자석 모드 조절은 모든 사용자에게 정상 반영 및 저장 허용
            let mut forced = new_config.clone();
            forced.bindings = config::Config::default().bindings;
            forced.settings.max_speed = 30.0;
            forced.settings.acceleration = 1.5;
            forced
        };
        println!("[Hot-Reload] New configuration evaluated and applied. Pro features: {}.", features_enabled);
    }
}

pub static APP_STATE: OnceLock<Arc<Mutex<AppState>>> = OnceLock::new();
static KEYBOARD_HOOK: OnceLock<Box<dyn KeyboardHook>> = OnceLock::new();

#[cfg(windows)]
#[link(name = "winmm")]
unsafe extern "system" {
    fn timeBeginPeriod(uPeriod: u32) -> u32;
    fn timeEndPeriod(uPeriod: u32) -> u32;
}

/// 마우스 조작 모드 활성화 시 WASD 방향 입력을 누적하여 상대적 이동 델타를 갱신합니다.
/// 락 획득 최소화를 위해 pre-acquired 이동 설정을 인자로 받아 처리합니다.
fn process_mouse_movement(
    state_ptr: &Arc<Mutex<AppState>>,
    movement_settings: Option<MovementSettings>,
    needs_reset: bool,
    dpi_scale: f64,
    dt: f64,
    remainder_x: &mut f64,
    remainder_y: &mut f64,
) {
    if let Some(params) = movement_settings {
        let start_time = match params.movement_start_time {
            Some(t) => t,
            None => {
                // 이동 시작 첫 프레임에만 발생하는 예외 경로
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

        // 100Hz (10ms = 0.010s) 기준 시간 비율 정규화 (dt가 10ms일 때 1.0, 15ms일 때 1.5)
        let normalized_speed = speed * (dt * 100.0);

        let (move_x, move_y) = crate::math::calculate_movement_delta(
            normalized_speed,
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
        if needs_reset {
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
    scroll_settings: Option<ScrollSettings>,
    needs_reset: bool,
    dt: f64,
    remainder_scroll_x: &mut f64,
    remainder_scroll_y: &mut f64,
) {
    if let Some(params) = scroll_settings {
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

        // 50ms 지연 후 즉각적 폭발 가속 스크롤 활성화
        if elapsed >= 0.050 {
            let scroll_elapsed = elapsed - 0.050;
            
            let base_scroll = 840.0; // 초당 7 notches (840 delta/sec, 약 21줄/초로 차분하게 시작)
            let max_scroll = 7200.0; // 초당 60 notches (7200 delta/sec, 최대 180줄/초)
            let accel_factor = 4.0;
            let speed = (base_scroll + scroll_elapsed * accel_factor * 800.0).min(max_scroll);
            
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
        if needs_reset {
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
        let mut last_tick = Instant::now();
        loop {
            thread::sleep(Duration::from_millis(interval_ms));
            if STOPPING.load(Ordering::SeqCst) { break; }

            // 실제 프레임 경과 시간 측정 (최대 50ms로 클램핑하여 절전 해제 시 순간 이동 방지)
            let now = Instant::now();
            let dt = now.duration_since(last_tick).as_secs_f64().min(0.05);
            last_tick = now;

            // 단일 락 획득으로 모든 상태 및 제어 파라미터를 동시 추출하여 락 경쟁 최소화
            let (is_mouse_mode, hz, movement_settings, scroll_settings, needs_movement_reset, needs_scroll_reset) = {
                let state = state_ptr.lock().unwrap();
                (
                    state.is_mouse_mode,
                    state.config.settings.refresh_rate_hz.unwrap_or(100),
                    state.get_movement_settings(),
                    state.get_scroll_settings(),
                    state.movement_start_time.is_some(),
                    state.scroll_start_time.is_some(),
                )
            };

            let is_moving = movement_settings.is_some();

            // 1. 이동 연산 처리 (락이 이미 릴리즈된 상태에서 계산 및 API 호출, dt 정규화 적용)
            process_mouse_movement(
                &state_ptr,
                movement_settings,
                needs_movement_reset,
                dpi_scale,
                dt,
                &mut remainder_x,
                &mut remainder_y,
            );

            // 2. 인디케이터 스냅 및 실시간 위치 갱신 연산 복원
            if is_mouse_mode {
                crate::indicator::update_indicator_position();
                crate::indicator::check_magnetic_snapping();
                crate::indicator::check_global_magnetic_snapping(is_moving);

                was_mouse_mode = true;
            } else if was_mouse_mode {
                #[cfg(windows)]
                crate::ui::win_gdi::force_restore_system_cursor();
                was_mouse_mode = false;
            }

            // 마우스 모드가 비활성화 상태인 경우 불필요한 루프 오버헤드를 낮추기 위해 40Hz(25ms)로 동작
            let safe_hz = if is_mouse_mode { hz.max(10).min(1000) } else { 40 };
            interval_ms = 1000 / safe_hz;

            // 3. 마우스 스크롤 연산 처리
            process_mouse_scrolling(
                &state_ptr,
                scroll_settings,
                needs_scroll_reset,
                dt,
                &mut remainder_scroll_x,
                &mut remainder_scroll_y,
            );
        }
    });
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PendingMouseAction {
    None,
    LeftDown,
    LeftUp,
    LeftClick,
    RightClick,
    ScrollUp,
    ScrollDown,
    ScrollLeft,
    ScrollRight,
    PageJumpTop,
    PageJumpBottom,
    PageJumpLeft,
    PageJumpRight,
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
static STOPPING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[cfg(not(test))]
fn output_sender() -> &'static std::sync::mpsc::Sender<(PendingMouseAction, Option<std::sync::mpsc::SyncSender<()>>)> {
    static OUTPUT: OnceLock<std::sync::mpsc::Sender<(PendingMouseAction, Option<std::sync::mpsc::SyncSender<()>>)>> = OnceLock::new();
    OUTPUT.get_or_init(|| {
        let (tx, rx) = std::sync::mpsc::channel::<(PendingMouseAction, Option<std::sync::mpsc::SyncSender<()>>) >();
        thread::spawn(move || {
            for (action, completed) in rx {
                if !STOPPING.load(Ordering::SeqCst) || action == PendingMouseAction::LeftUp {
                    execute_pending_action_now(action);
                }
                if let Some(completed) = completed { let _ = completed.send(()); }
            }
        });
        tx
    })
}

// Queue under AppState when ordering matters; OS injection runs after releasing its lock.
fn execute_pending_action(action: PendingMouseAction) {
    if action == PendingMouseAction::None { return; }
    #[cfg(not(test))]
    { let _ = output_sender().send((action, None)); }
    #[cfg(test)]
    TEST_ACTIONS.with(|actions| actions.borrow_mut().push(action));
}

#[cfg(test)]
thread_local! {
    static TEST_ACTIONS: std::cell::RefCell<Vec<PendingMouseAction>> = const { std::cell::RefCell::new(Vec::new()) };
}

#[cfg(not(test))]
fn execute_pending_action_now(action: PendingMouseAction) {
    match action {
        PendingMouseAction::LeftDown => get_system_controller().left_down(),
        PendingMouseAction::LeftUp => get_system_controller().left_up(),
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
            get_system_controller().scroll(60);
            crate::indicator::trigger_click_motion(crate::indicator::ClickType::Scroll);
        }
        PendingMouseAction::ScrollDown => {
            get_system_controller().scroll(-60);
            crate::indicator::trigger_click_motion(crate::indicator::ClickType::Scroll);
        }
        PendingMouseAction::ScrollLeft => {
            get_system_controller().scroll_horizontal(-60);
            crate::indicator::trigger_click_motion(crate::indicator::ClickType::Scroll);
        }
        PendingMouseAction::ScrollRight => {
            get_system_controller().scroll_horizontal(60);
            crate::indicator::trigger_click_motion(crate::indicator::ClickType::Scroll);
        }
        PendingMouseAction::PageJumpTop => get_system_controller().simulate_page_jump(true),
        PendingMouseAction::PageJumpBottom => get_system_controller().simulate_page_jump(false),
        PendingMouseAction::PageJumpLeft => get_system_controller().scroll_horizontal(-10000),
        PendingMouseAction::PageJumpRight => get_system_controller().scroll_horizontal(10000),
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
                        execute_pending_action(action);
                        (true, PendingMouseAction::None)
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
    let state_arc = APP_STATE.get()?;
    let configured_vk = {
        let state = state_arc.lock().unwrap();
        config::key_name_to_vk(&state.config.settings.modifier_key).unwrap_or(VK_CAPITAL)
    };
    if !matches_modifier(configured_vk, event.vk_code) { return None; }

    if event.is_injected_by_keysor {
        return Some(HookResult::Pass);
    }

    #[cfg(windows)]
    crate::diagnostics::record_activation(event.is_keydown, event.is_keyup);

    let mut indicator_action = IndicatorAction::None;

    if let Some(state_arc) = APP_STATE.get() {
        let mut state = state_arc.lock().unwrap();
        let is_toggle_mode = state.config.settings.modifier_mode.eq_ignore_ascii_case("Toggle");

        if event.is_keydown {
            let should_process = begin_modifier_press(&mut state.caps_lock_press_time);

            if should_process {
                state.caps_lock_press_time = Some(Instant::now());

                if is_toggle_mode {
                    if state.is_mouse_mode {
                        println!("[Keysor] Caps Lock -> Mouse mode OFF (Toggle)");
                        state.deactivate_mouse_mode();
                        if configured_vk == VK_CAPITAL { get_system_controller().ensure_caps_lock_state(false); }
                        indicator_action = IndicatorAction::Hide;
                    } else {
                        println!("[Keysor] Caps Lock -> Mouse mode ON (Toggle)");
                        state.is_mouse_mode = true;
                        if configured_vk == VK_CAPITAL { get_system_controller().ensure_caps_lock_state(true); }
                        indicator_action = IndicatorAction::Show;
                    }
                } else {
                    println!("[Keysor] Caps Lock -> Mouse mode ON (Hold)");
                    state.is_mouse_mode = true;
                    if configured_vk == VK_CAPITAL { get_system_controller().ensure_caps_lock_state(true); }
                    indicator_action = IndicatorAction::Show;
                }
            }
        } else if event.is_keyup {
            state.caps_lock_press_time = None;

            if !is_toggle_mode {
                println!("[Keysor] Caps Lock Released -> Mouse mode OFF (Hold)");
                state.deactivate_mouse_mode();
                if configured_vk == VK_CAPITAL { get_system_controller().ensure_caps_lock_state(false); }
                indicator_action = IndicatorAction::Hide;
            }
        }
    }

    match indicator_action {
        IndicatorAction::Show => crate::indicator::show_indicator(),
        IndicatorAction::Hide => crate::indicator::hide_indicator(),
        IndicatorAction::None => {}
    }

    Some(HookResult::Block)
}

/// 비동기 스페이스 드래그 홀드 임계값 감지 스레드를 기동합니다.
fn begin_modifier_press(pressed: &mut Option<Instant>) -> bool {
    if pressed.is_some() { return false; }
    *pressed = Some(Instant::now());
    true
}

fn matches_modifier(configured: u32, actual: u32) -> bool {
    configured == actual || match configured {
        0x10 => matches!(actual, 0xA0 | 0xA1),
        0x11 => matches!(actual, 0xA2 | 0xA3),
        0x12 => matches!(actual, 0xA4 | 0xA5),
        _ => false,
    }
}

fn start_drag_if_due(state: &mut AppState, hold_ms: u64) {
    if state.is_mouse_mode && state.is_space_down && !state.space_scroll_combo && !state.is_dragging
        && state.space_press_time.is_some_and(|t| t.elapsed() >= Duration::from_millis(hold_ms)) {
        state.is_dragging = true;
        state.space_tap_count = 0;
        execute_pending_action(PendingMouseAction::LeftDown);
    }
}

fn spawn_drag_detection_thread(state_arc: Arc<Mutex<AppState>>, hold_ms: u64) {
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(hold_ms));

        let mut state = state_arc.lock().unwrap();
        start_drag_if_due(&mut state, hold_ms);

    });
}

/// 스페이스바를 활용한 좌클릭/더블클릭/드래그앤드롭 기능을 에뮬레이션합니다.
fn page_jump_for_scroll(action: &str) -> PendingMouseAction {
    match action {
        "MouseScrollUp" => PendingMouseAction::PageJumpTop,
        "MouseScrollDown" => PendingMouseAction::PageJumpBottom,
        "MouseScrollLeft" => PendingMouseAction::PageJumpLeft,
        "MouseScrollRight" => PendingMouseAction::PageJumpRight,
        _ => PendingMouseAction::None,
    }
}

fn handle_space_keydown(state: &mut AppState, state_arc: &Arc<Mutex<AppState>>) -> PendingMouseAction {
    if !state.is_space_down {
        state.is_space_down = true;
        if !state.active_scroll_keys.is_empty() {
            let jump = state.active_scroll_keys.iter()
                .filter_map(|vk| state.vk_bindings.get(vk))
                .map(|action| page_jump_for_scroll(action))
                .find(|action| *action != PendingMouseAction::None)
                .unwrap_or(PendingMouseAction::None);
            state.begin_space_scroll_combo();
            return jump;
        }
        state.space_press_time = Some(Instant::now());

        let hold_ms = state.config.settings.drag_hold_threshold_ms;
        spawn_drag_detection_thread(Arc::clone(state_arc), hold_ms);
    }
    PendingMouseAction::None
}

fn handle_space_keyup(
    state: &mut AppState,
    hold_ms: u64,
    should_start_release_handler: &mut bool,
) {
    state.is_space_down = false;
    if state.space_scroll_combo {
        state.space_scroll_combo = false;
        state.space_press_time = None;
        return;
    }
    let elapsed = state.space_press_time.map(|t| t.elapsed());
    state.space_press_time = None;

    if state.is_dragging {
        state.is_dragging = false;
        execute_pending_action(PendingMouseAction::LeftUp);
    } else if elapsed.is_some_and(|elapsed| elapsed < Duration::from_millis(hold_ms)) {
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
    let mut should_start_release_handler = false;
    let mut pending_action = PendingMouseAction::None;

    {
        let mut state = state_arc.lock().unwrap();
        state.caps_lock_used_as_modifier = true;

        if event.is_keydown {
            println!("[Debug] Space Keydown: is_space_down={}, is_dragging={}", state.is_space_down, state.is_dragging);
            pending_action = handle_space_keydown(&mut state, state_arc);
        } else if event.is_keyup {
            println!("[Debug] Space Keyup: space_tap_count={}, is_dragging={}", state.space_tap_count, state.is_dragging);
            let hold_ms = state.config.settings.drag_hold_threshold_ms;
            handle_space_keyup(&mut state, hold_ms, &mut should_start_release_handler);
        }
    }

    execute_pending_action(pending_action);
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

    // Shift+키 조합 바인딩 테이블 (vk_code → PendingMouseAction)
    if state.shift_pressed {
        let shift_action = match vk_code {
            VK_Q => Some(PendingMouseAction::BrowserBack),
            VK_E => Some(PendingMouseAction::BrowserForward),
            VK_J => Some(PendingMouseAction::VirtualDesktopLeft),
            VK_K => Some(PendingMouseAction::VirtualDesktopRight),
            _ => None,
        };
        if let Some(action) = shift_action {
            if is_keydown {
                pending_action = action;
            }
            return (HookResult::Block, pending_action);
        }
    } else {
        // J/K 탭 전환 (Pro 전용)
        let tab_action = match vk_code {
            VK_J => Some(PendingMouseAction::TabLeft),
            VK_K => Some(PendingMouseAction::TabRight),
            _ => None,
        };
        if let Some(action) = tab_action {
            if is_keydown {
                if state.is_pro || state.is_trial {
                    pending_action = action;
                } else {
                    get_system_controller().beep();
                    println!("[License] J/K Tab switching is a Pro-only feature.");
                }
            }
            return (HookResult::Block, pending_action);
        }
    }

    // 일반 이동 및 보조 클릭 가로채기 매칭
    if let Some(action) = state.vk_bindings.get(&vk_code).cloned() {
        state.caps_lock_used_as_modifier = true;
        #[cfg(windows)]
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
                    "MouseRightClick" => {
                        state.is_right_down = true;
                        pending_action = PendingMouseAction::RightClick;
                    }
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
            } else if action == "MouseRightClick" {
                state.is_right_down = false;
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
        let release_space = {
            let mut state = state_arc.lock().unwrap();
            if event.is_keyup { state.release_tracked_key(event.vk_code); }
            event.is_keyup && event.vk_code == VK_SPACE && state.is_space_down
        };
        if release_space { return process_space_click(state_arc, &event); }

        let is_modifier_key = {
            let mut state = state_arc.lock().unwrap();
            if event.is_keydown {
                state.last_input_time = Instant::now();
            }
            state.update_modifier_key_state(event.vk_code, event.is_keydown)
        };

        // 0. 비상 커서 복구 핫키 (Emergency Hotkey): Ctrl + Shift + Alt + Esc 또는 Ctrl + Shift + Alt + C
        if event.is_keydown && (event.vk_code == 0x1B || event.vk_code == 0x43) {
            if let Ok(mut state) = state_arc.try_lock() {
                if state.ctrl_pressed && state.alt_pressed && state.shift_pressed {
                    println!("[Emergency] Emergency cursor restore hotkey triggered.");
                    state.deactivate_mouse_mode();
                    get_system_controller().ensure_caps_lock_off();
                    #[cfg(windows)]
                    crate::ui::win_gdi::force_restore_system_cursor();
                    crate::indicator::hide_indicator();
                    return HookResult::Pass;
                }
            }
        }

        // 1. 키서 프로그램 완전 종료 핫키: Ctrl + Alt + K
        if event.is_keydown && event.vk_code == 0x4B /* VK_K */ {
            if let Ok(mut state) = state_arc.try_lock() {
                if state.ctrl_pressed && state.alt_pressed {
                    println!("[Shutdown] Ctrl + Alt + K triggered. Terminating Keysor cleanly...");
                    state.deactivate_mouse_mode();
                    get_system_controller().ensure_caps_lock_off();
                    #[cfg(windows)]
                    crate::ui::win_gdi::force_restore_system_cursor();
                    drop(state);
                    crate::hook::cleanup_hook();
                    std::process::exit(0);
                }
            }
        }

        // Win+Tab, Alt+Tab 단축키 시작 감지 및 강제 커서 복원
        if event.vk_code == 0x09 && event.is_keydown {
            if let Ok(state) = state_arc.try_lock() {
                if state.win_pressed || state.alt_pressed {
                    #[cfg(windows)]
                    {
                        crate::ui::win_gdi::SUSPEND_CURSOR_HIDE.store(true, Ordering::SeqCst);
                        unsafe {
                            crate::ui::win_gdi::force_restore_system_cursor();
                        }
                    }
                }
            }
        }

        // 일반 키 입력 시 화면 전환 일시정지 해제
        if event.is_keydown && event.vk_code != 0x09 
            && event.vk_code != 0x5B && event.vk_code != 0x5C 
            && event.vk_code != 0x12 && event.vk_code != 0xA4 && event.vk_code != 0xA5
            && event.vk_code != 0x11 && event.vk_code != 0xA2 && event.vk_code != 0xA3 {
            #[cfg(windows)]
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
            // Ctrl, Alt 단축키 조합이 활성화된 경우만 기존 OS 단축키 바이패스(Pass) 처리
            // Win 키 단독 눌림/시작 메뉴 활성화 시에는 Keysor WASD 마우스 이동 조작이 100% 작동함
            let is_ctrl_alt_shortcut = {
                let state = state_arc.lock().unwrap();
                state.ctrl_pressed || state.alt_pressed
            };
            if is_ctrl_alt_shortcut {
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

            // Shift + Space 단축키를 통한 Enter 키 에뮬레이션
            let is_shift_down = {
                let state = state_arc.lock().unwrap();
                state.shift_pressed
            };
            if event.vk_code == VK_SPACE && is_shift_down {
                if event.is_keydown {
                    println!("[Action] Shift + Space -> Enter simulated");
                    get_system_controller().simulate_enter_key();
                }
                return HookResult::Block;
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
fn initial_state(config: Config, is_pro: bool, is_trial: bool) -> AppState {
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

    AppState {
        config,
        vk_bindings,
        is_mouse_mode: false,
        is_pro,
        is_trial,
        caps_lock_press_time: None,
        caps_lock_used_as_modifier: false,
        space_press_time: None,
        is_space_down: false,
        space_scroll_combo: false,
        is_right_down: false,
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
        last_input_time: Instant::now(),
    }
}

pub fn start_hook(config: Config, is_pro: bool, is_trial: bool) -> Result<(), String> {
    let state = Arc::new(Mutex::new(initial_state(config, is_pro, is_trial)));

    APP_STATE.set(Arc::clone(&state)).ok();

    // 0. 초기 구동 시 Caps Lock 상태를 안전하게 OFF로 리셋
    get_system_controller().ensure_caps_lock_off();

    // Windows 고정밀 1ms 멀티미디어 타이머 활성화
    #[cfg(windows)]
    unsafe {
        timeBeginPeriod(1);
    }

    // 1. liquid smooth 커서 가속 이동용 스레드 시작
    start_movement_thread(Arc::clone(&state));

    // 2. OS 플랫폼 전역 키보드 훅 리스너 시작
    let hook = create_keyboard_hook();
    hook.start_listening(Box::new(handle_keyboard_event))?;
    KEYBOARD_HOOK.set(hook).ok();
    Ok(())
}

/// 훅 자원을 OS에 안전하게 반환하는 페일세이프 해제 함수
pub fn cleanup_hook() {
    STOPPING.store(true, Ordering::SeqCst);
    #[cfg(not(test))]
    {
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        let _ = output_sender().send((PendingMouseAction::LeftUp, Some(tx)));
        let _ = rx.recv_timeout(Duration::from_millis(500));
    }
    if let Some(hook) = KEYBOARD_HOOK.get() {
        hook.stop_listening();
    }
    get_system_controller().ensure_caps_lock_off();
    // Restore system cursor on exit if it was hidden
    #[cfg(windows)]
    crate::ui::win_gdi::restore_system_cursor();
    #[cfg(windows)]
    unsafe {
        timeEndPeriod(1);
    }
}

pub(crate) fn cancel_active_input() {
    if let Some(state_arc) = APP_STATE.get() {
        if let Ok(mut state) = state_arc.lock() {
            state.deactivate_mouse_mode();
            state.caps_lock_press_time = None;
        }
        get_system_controller().ensure_caps_lock_off();
        crate::indicator::hide_indicator();
    }
}

pub fn modifier_sync_guard() {
    if let Some(state_arc) = APP_STATE.get() {
        let (is_mouse_mode, is_toggle_mode) = {
            let state = state_arc.lock().unwrap();
            (state.is_mouse_mode, state.config.settings.modifier_mode.eq_ignore_ascii_case("Toggle"))
        };
        if is_mouse_mode && !is_toggle_mode {
            if let Some(hook) = KEYBOARD_HOOK.get() {
                hook.modifier_sync_guard(is_mouse_mode, is_toggle_mode, cancel_active_input);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn saved_magnet_toggle_does_not_deactivate_input() {
        let mut s = initial_state(Config::default(), true, false);
        s.is_mouse_mode = true;
        s.active_movement_keys.insert(0x57);
        s.caps_lock_press_time = Some(Instant::now());
        for enabled in [true, false, true] {
            s.config.settings.magnetic_mode = Some(enabled);
            s.config.settings.global_magnetic_mode = Some(enabled);
            s.apply_file_configuration(s.config.clone());
            assert!(s.is_mouse_mode);
            assert!(s.active_movement_keys.contains(&0x57));
            assert!(s.caps_lock_press_time.is_some());
        }
    }
    fn state() -> AppState {
        let mut s = initial_state(Config::default(), true, false);
        s.is_mouse_mode = true;
        s
    }
    fn actions() -> Vec<PendingMouseAction> {
        TEST_ACTIONS.with(|a| std::mem::take(&mut *a.borrow_mut()))
    }
    #[test]
    fn movement_and_scroll_release_survive_shortcut_modifiers() {
        let mut s = state();
        process_movement_and_actions(&mut s, 0x57, true, false);
        process_movement_and_actions(&mut s, 0x52, true, false);
        s.ctrl_pressed = true;
        s.release_tracked_key(0x57);
        s.release_tracked_key(0x52);
        assert!(s.get_movement_settings().is_none());
        assert!(s.get_scroll_settings().is_none());
    }
    #[test]
    fn drag_down_and_release_are_queued_in_order() {
        let mut s = state();
        s.is_space_down = true;
        s.space_press_time = Some(Instant::now() - Duration::from_millis(300));
        start_drag_if_due(&mut s, 250);
        let mut release = false;
        handle_space_keyup(&mut s, 250, &mut release);
        assert_eq!(actions(), vec![PendingMouseAction::LeftDown, PendingMouseAction::LeftUp]);
        assert!(!s.is_dragging);
    }
    #[test]
    fn deactivation_cancels_pending_drag_and_click() {
        let mut s = state();
        s.is_space_down = true;
        s.space_press_time = Some(Instant::now() - Duration::from_secs(1));
        s.space_tap_count = 1;
        s.deactivate_mouse_mode();
        start_drag_if_due(&mut s, 250);
        assert!(actions().is_empty());
        assert_eq!(s.space_tap_count, 0);
        assert!(s.space_press_time.is_none());
    }
    #[test]
    fn configured_modifier_matches_generic_and_sided_keys() {
        assert!(matches_modifier(0x10, 0xA0));
        assert!(matches_modifier(0x11, 0xA3));
        assert!(matches_modifier(0x41, 0x41));
        assert!(!matches_modifier(0x41, VK_CAPITAL));
    }
    #[test]
    #[cfg(not(target_os = "macos"))]
    fn modifier_repeat_never_toggles_until_release() {
        let mut pressed = None;
        assert!(begin_modifier_press(&mut pressed));
        pressed = Some(Instant::now() - Duration::from_secs(1));
        assert!(!begin_modifier_press(&mut pressed));
        pressed = None;
        assert!(begin_modifier_press(&mut pressed));
    }
    #[test]
    fn shift_or_ctrl_does_not_prevent_drag_release() {
        let mut s = state();
        s.is_space_down = true;
        s.is_dragging = true;
        s.ctrl_pressed = true;
        s.shift_pressed = true;
        let mut release = false;
        handle_space_keyup(&mut s, 250, &mut release);
        assert_eq!(actions(), vec![PendingMouseAction::LeftUp]);
        assert!(!s.is_space_down);
    }

    #[test]
    fn scroll_then_space_jumps_once_without_click_or_drag() {
        let state_arc = Arc::new(Mutex::new(state()));
        let mut s = state_arc.lock().unwrap();
        let (_, initial) = process_movement_and_actions(&mut s, 0x52, true, false);
        assert_eq!(initial, PendingMouseAction::ScrollUp);
        assert_eq!(handle_space_keydown(&mut s, &state_arc), PendingMouseAction::PageJumpTop);
        assert_eq!(handle_space_keydown(&mut s, &state_arc), PendingMouseAction::None);
        assert!(s.space_press_time.is_none());
        start_drag_if_due(&mut s, 250);
        let mut release = false;
        handle_space_keyup(&mut s, 250, &mut release);
        assert!(!release);
        assert_eq!(s.space_tap_count, 0);
        assert!(!s.space_scroll_combo);
        assert!(actions().is_empty());
    }

    #[test]
    fn space_then_scroll_keeps_normal_click_after_scroll_keyup() {
        let state_arc = Arc::new(Mutex::new(state()));
        let mut s = state_arc.lock().unwrap();
        assert_eq!(handle_space_keydown(&mut s, &state_arc), PendingMouseAction::None);
        let (_, scroll) = process_movement_and_actions(&mut s, 0x46, true, false);
        assert_eq!(scroll, PendingMouseAction::ScrollDown);
        assert!(!s.space_scroll_combo);
        let (_, repeat) = process_movement_and_actions(&mut s, 0x46, true, false);
        assert_eq!(repeat, PendingMouseAction::None);
        s.release_tracked_key(0x46);
        assert!(s.get_scroll_settings().is_none());
        let mut release = false;
        handle_space_keyup(&mut s, 250, &mut release);
        assert!(release);
        assert_eq!(s.space_tap_count, 1);
        assert!(actions().is_empty());
    }

    #[test]
    fn scroll_after_space_preserves_existing_drag() {
        let mut s = state();
        s.is_space_down = true;
        s.is_dragging = true;
        let (_, pending) = process_movement_and_actions(&mut s, 0x52, true, false);
        assert!(!s.space_scroll_combo);
        assert!(s.is_dragging);
        assert_eq!(pending, PendingMouseAction::ScrollUp);
        assert!(actions().is_empty());
        let mut release = false;
        handle_space_keyup(&mut s, 250, &mut release);
        assert!(!release);
        assert_eq!(actions(), vec![PendingMouseAction::LeftUp]);
    }

    #[test]
    fn horizontal_scroll_space_jumps_are_preserved() {
        for (vk, expected) in [(0x51, PendingMouseAction::PageJumpLeft), (0x45, PendingMouseAction::PageJumpRight)] {
            let state_arc = Arc::new(Mutex::new(state()));
            let mut s = state_arc.lock().unwrap();
            process_movement_and_actions(&mut s, vk, true, false);
            assert_eq!(handle_space_keydown(&mut s, &state_arc), expected);
            let mut release = false;
            handle_space_keyup(&mut s, 250, &mut release);
            assert!(!release);
        }
        assert!(actions().is_empty());
    }

    #[test]
    fn space_without_scroll_still_counts_as_click() {
        let mut s = state();
        s.is_space_down = true;
        s.space_press_time = Some(Instant::now());
        let mut release = false;
        handle_space_keyup(&mut s, 250, &mut release);
        assert!(release);
        assert_eq!(s.space_tap_count, 1);
        assert!(actions().is_empty());
    }

}
