use std::sync::{OnceLock, Mutex};
use std::thread;
use std::sync::atomic::Ordering;
use uiautomation::core::UIAutomation;
use uiautomation::types::Handle;
use windows_sys::Win32::Foundation::{HWND, RECT};
use windows_sys::Win32::UI::WindowsAndMessaging::{GetClassNameW, GetWindowRect};

use crate::ui::win_gdi::{
    HUD_HWND, INDICATOR_HWND, MAIN_HWND, HUD_LAST_SNAPPED, FORCE_UIA_REFRESH
};

// =========================================================================
// 자석 모드 및 전역 UIA 점프/이탈 누적 연산 엔진 (시각적 잔상 피드백 적용)
// =========================================================================

static GLOBAL_SNAP_TARGETS: OnceLock<Mutex<Vec<(i32, i32)>>> = OnceLock::new();
static LAST_GLOBAL_SNAPPED_POS: OnceLock<Mutex<Option<(i32, i32, bool)>>> = OnceLock::new();
static LAST_JUMP_TIME: OnceLock<Mutex<std::time::Instant>> = OnceLock::new();
static HUD_ESCAPE_ACCUM: OnceLock<Mutex<f64>> = OnceLock::new();
static GLOBAL_ESCAPE_ACCUM: OnceLock<Mutex<f64>> = OnceLock::new();

#[cfg(debug_assertions)]
fn log_debug(msg: &str) {
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("keysor_debug.log")
    {
        use std::io::Write;
        let _ = writeln!(file, "[{:?}] {}", std::time::Instant::now(), msg);
    }
}

#[cfg(not(debug_assertions))]
#[inline(always)]
fn log_debug(_msg: &str) {}

/// 마우스 조작 모드에서의 이동 상태(기본 속도, 이탈 여부, 이동 방향, 키 누름 경과 시간)를 획득합니다.
fn get_movement_status() -> (f64, bool, Option<String>, std::time::Duration) {
    let mut current_speed = 1.0;
    let mut should_release = false;
    let mut new_dir = None;
    let mut hold_duration = std::time::Duration::from_secs(0);
    
    if let Some(state_arc) = crate::hook::APP_STATE.get() {
        let state = state_arc.lock().unwrap();
        let base_speed = state.config.settings.base_speed;
        let max_speed = state.config.settings.max_speed;
        let acceleration = state.config.settings.acceleration;
        
        if !state.active_movement_keys.is_empty() {
            should_release = true;
            for key in &state.active_movement_keys {
                if let Some(action) = state.vk_bindings.get(key) {
                    match action.as_str() {
                        "MouseMoveUp" => new_dir = Some("Up".to_string()),
                        "MouseMoveDown" => new_dir = Some("Down".to_string()),
                        "MouseMoveLeft" => new_dir = Some("Left".to_string()),
                        "MouseMoveRight" => new_dir = Some("Right".to_string()),
                        _ => {}
                    }
                }
                if new_dir.is_some() {
                    break;
                }
            }
        }
        if let Some(start) = state.movement_start_time {
            hold_duration = start.elapsed();
        }
        
        // 실시간 가속 속도 계산
        let elapsed = hold_duration.as_secs_f64();
        current_speed = crate::math::calculate_speed(base_speed, elapsed, acceleration, max_speed);
    }
    
    (current_speed, should_release, new_dir, hold_duration)
}

/// 누른 시간(hold_duration)에 비례하여 동적 쿨다운 제한 시간(ms)을 반환합니다.
fn calculate_cooldown_limit(hold_duration: std::time::Duration) -> u64 {
    let hold_ms = hold_duration.as_millis();
    if hold_ms < 400 {
        300
    } else if hold_ms < 1000 {
        let progress = (hold_ms - 400) as f64 / 600.0;
        (300.0 - progress * 220.0) as u64
    } else {
        80
    }
}

fn find_adjacent_target(
    sx: i32,
    sy: i32,
    dir: &str,
    targets: &[(i32, i32)],
) -> Option<(i32, i32)> {
    let mut best_target: Option<(i32, i32)> = None;
    let mut best_dist = f64::MAX;
    
    let scale = crate::platform::get_system_controller().get_dpi_scale();
    let max_dir_dist = (120.0 * scale) as i32; 
    let max_cross_dist = (40.0 * scale) as i32;

    for &(tx, ty) in targets {
        match dir {
            "Right" => {
                let dx = tx - sx;
                let dy = (ty - sy).abs();
                if dx > 8 && dx <= max_dir_dist && dy < max_cross_dist {
                    let dist = (dx as f64).powi(2) + (dy as f64).powi(2);
                    if dist < best_dist {
                        best_dist = dist;
                        best_target = Some((tx, ty));
                    }
                }
            }
            "Left" => {
                let dx = sx - tx;
                let dy = (ty - sy).abs();
                if dx > 8 && dx <= max_dir_dist && dy < max_cross_dist {
                    let dist = (dx as f64).powi(2) + (dy as f64).powi(2);
                    if dist < best_dist {
                        best_dist = dist;
                        best_target = Some((tx, ty));
                    }
                }
            }
            "Down" => {
                let dy = ty - sy;
                let dx = (tx - sx).abs();
                if dy > 8 && dy <= max_dir_dist && dx < max_cross_dist {
                    let dist = (dx as f64).powi(2) + (dy as f64).powi(2);
                    if dist < best_dist {
                        best_dist = dist;
                        best_target = Some((tx, ty));
                    }
                }
            }
            "Up" => {
                let dy = sy - ty;
                let dx = (tx - sx).abs();
                if dy > 8 && dy <= max_dir_dist && dx < max_cross_dist {
                    let dist = (dx as f64).powi(2) + (dy as f64).powi(2);
                    if dist < best_dist {
                        best_dist = dist;
                        best_target = Some((tx, ty));
                    }
                }
            }
            _ => {}
        }
    }
    best_target
}

pub fn check_magnetic_snapping() {
    let state_arc = crate::hook::APP_STATE.get();
    let (enabled, is_dragging, features_enabled) = state_arc.map_or((false, false, false), |arc| {
        let state = arc.lock().unwrap();
        (
            state.config.settings.magnetic_mode.unwrap_or(false),
            state.is_dragging,
            state.is_pro || state.is_trial,
        )
    });
    if !enabled || is_dragging || !features_enabled {
        HUD_LAST_SNAPPED.store(0, std::sync::atomic::Ordering::SeqCst);
        return;
    }
    
    if let Some(&hud_hwnd) = HUD_HWND.get() {
        unsafe {
            if windows_sys::Win32::UI::WindowsAndMessaging::IsWindowVisible(hud_hwnd) == 0 {
                return;
            }
            
            let mut hud_rect = std::mem::zeroed();
            windows_sys::Win32::UI::WindowsAndMessaging::GetWindowRect(hud_hwnd, &mut hud_rect);
            
            let mut cursor_pt = std::mem::zeroed();
            windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut cursor_pt);
            
            let targets = [
                (1, 687, 40),
                (2, 754, 20),
                (3, 785, 20),
                (4, 678, 186),
                (5, 738, 186),
                (6, 708, 226),
                (7, 708, 266),
                (8, 708, 306),
            ];
            
            static HUD_LANDED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
            static HUD_COOLDOWN: OnceLock<Mutex<Option<(u32, std::time::Instant)>>> = OnceLock::new();
            
            let snap_threshold = 25.0;
            let release_threshold = 15.0;
            
            let last_id = HUD_LAST_SNAPPED.load(std::sync::atomic::Ordering::SeqCst);
            
            if last_id != 0 {
                let (base_speed, should_release, new_dir, hold_duration) = get_movement_status();
                let cooldown_limit = calculate_cooldown_limit(hold_duration);
                
                let accum_lock = HUD_ESCAPE_ACCUM.get_or_init(|| Mutex::new(0.0));
                let mut accum = accum_lock.lock().unwrap();
                
                static LAST_HUD_INPUT: OnceLock<Mutex<std::time::Instant>> = OnceLock::new();
                let last_input_lock = LAST_HUD_INPUT.get_or_init(|| Mutex::new(std::time::Instant::now() - std::time::Duration::from_secs(5)));
                if should_release {
                    if let Ok(mut li) = last_input_lock.lock() {
                        *li = std::time::Instant::now();
                    }
                } else {
                    let elapsed = last_input_lock.lock().unwrap().elapsed();
                    if elapsed >= std::time::Duration::from_millis(300) {
                        *accum = 0.0;
                    }
                }
                
                if should_release && new_dir.is_some() {
                    let dir = new_dir.as_ref().unwrap();
                    let last_jump = LAST_JUMP_TIME.get_or_init(|| Mutex::new(std::time::Instant::now() - std::time::Duration::from_secs(5)));
                    let mut can_jump = false;
                    if let Ok(lj) = last_jump.try_lock() {
                        if lj.elapsed() >= std::time::Duration::from_millis(cooldown_limit) {
                            can_jump = true;
                        }
                    }
                    
                    if can_jump {
                        if let Some(&(_, sx, sy)) = targets.iter().find(|&&(id, _, _)| id == last_id) {
                            let target_screen_x = hud_rect.left + sx;
                            let target_screen_y = hud_rect.top + sy;
                            let hud_screen_targets: Vec<(i32, i32)> = targets.iter()
                                .map(|&(_, tx, ty)| (hud_rect.left + tx, hud_rect.top + ty))
                                .collect();
                            
                            if let Some((jx, jy)) = find_adjacent_target(target_screen_x, target_screen_y, dir, &hud_screen_targets) {
                                if let Some(&(jid, _, _)) = targets.iter().find(|&&(_, tx, ty)| {
                                    hud_rect.left + tx == jx && hud_rect.top + ty == jy
                                }) {
                                    windows_sys::Win32::UI::WindowsAndMessaging::SetCursorPos(jx, jy);
                                    HUD_LAST_SNAPPED.store(jid, std::sync::atomic::Ordering::SeqCst);
                                    HUD_LANDED.store(true, std::sync::atomic::Ordering::SeqCst);
                                    
                                    if let Ok(mut lj) = last_jump.lock() {
                                        *lj = std::time::Instant::now();
                                    }
                                    
                                    let cooldown_lock = HUD_COOLDOWN.get_or_init(|| Mutex::new(None));
                                    *cooldown_lock.lock().unwrap() = Some((last_id, std::time::Instant::now()));
                                    *accum = 0.0;
                                    return;
                                }
                            }
                        }
                    }
                }
                
                if let Some(&(_, tx, ty)) = targets.iter().find(|&&(id, _, _)| id == last_id) {
                    let target_screen_x = hud_rect.left + tx;
                    let target_screen_y = hud_rect.top + ty;
                    
                    if should_release {
                        let mut cooldown_active = false;
                        let last_jump = LAST_JUMP_TIME.get_or_init(|| Mutex::new(std::time::Instant::now() - std::time::Duration::from_secs(5)));
                        if let Ok(lj) = last_jump.try_lock() {
                            if lj.elapsed() < std::time::Duration::from_millis(cooldown_limit) {
                                cooldown_active = true;
                            }
                        }
                        
                        if cooldown_active {
                            *accum = 0.0;
                        } else {
                            let step = (base_speed * 2.0).max(1.0);
                            *accum += step;
                        }
                        
                        let mut dynamic_release_threshold = release_threshold;
                        let hold_ms = hold_duration.as_millis();
                        if hold_ms > 150 {
                            let progress = ((hold_ms - 150) as f64 / 350.0).min(1.0);
                            dynamic_release_threshold = release_threshold - (progress * (release_threshold - 4.0));
                        }
                        
                        if *accum >= dynamic_release_threshold {
                            HUD_LAST_SNAPPED.store(0, std::sync::atomic::Ordering::SeqCst);
                            HUD_LANDED.store(false, std::sync::atomic::Ordering::SeqCst);
                            *accum = 0.0;
                            
                            let cooldown_lock = HUD_COOLDOWN.get_or_init(|| Mutex::new(None));
                            *cooldown_lock.lock().unwrap() = Some((last_id, std::time::Instant::now()));
                            
                            let mut push_dx = 0;
                            let mut push_dy = 0;
                            if let Some(ref dir) = new_dir {
                                match dir.as_str() {
                                    "Left" => push_dx = -15,
                                    "Right" => push_dx = 15,
                                    "Up" => push_dy = -15,
                                    "Down" => push_dy = 15,
                                    _ => {}
                                }
                            }
                            windows_sys::Win32::UI::WindowsAndMessaging::SetCursorPos(target_screen_x + push_dx, target_screen_y + push_dy);
                            
                            if let Some(state_arc) = crate::hook::APP_STATE.get() {
                                if let Ok(mut state) = state_arc.try_lock() {
                                    state.movement_start_time = Some(std::time::Instant::now());
                                }
                            }
                            return;
                        } else {
                            windows_sys::Win32::UI::WindowsAndMessaging::SetCursorPos(target_screen_x, target_screen_y);
                            HUD_LANDED.store(true, std::sync::atomic::Ordering::SeqCst);
                            return;
                        }
                    } else {
                        let elapsed = last_input_lock.lock().unwrap().elapsed();
                        if elapsed >= std::time::Duration::from_millis(300) {
                            *accum = 0.0;
                        }
                        if HUD_LANDED.load(std::sync::atomic::Ordering::SeqCst) {
                            let dx = cursor_pt.x - target_screen_x;
                            let dy = cursor_pt.y - target_screen_y;
                            let dist = ((dx * dx + dy * dy) as f64).sqrt();
                            if dist > 3.0 {
                                HUD_LAST_SNAPPED.store(0, std::sync::atomic::Ordering::SeqCst);
                                HUD_LANDED.store(false, std::sync::atomic::Ordering::SeqCst);
                                let cooldown_lock = HUD_COOLDOWN.get_or_init(|| Mutex::new(None));
                                *cooldown_lock.lock().unwrap() = Some((last_id, std::time::Instant::now()));
                                return;
                            }
                            windows_sys::Win32::UI::WindowsAndMessaging::SetCursorPos(target_screen_x, target_screen_y);
                            return;
                        }
                        
                        let dx = cursor_pt.x - target_screen_x;
                        let dy = cursor_pt.y - target_screen_y;
                        let dist = ((dx * dx + dy * dy) as f64).sqrt();
                        
                        if dist < 1.5 {
                            windows_sys::Win32::UI::WindowsAndMessaging::SetCursorPos(target_screen_x, target_screen_y);
                            HUD_LANDED.store(true, std::sync::atomic::Ordering::SeqCst);
                        } else {
                            let lerp_factor = 0.35;
                            let next_x = cursor_pt.x as f64 + (target_screen_x as f64 - cursor_pt.x as f64) * lerp_factor;
                            let next_y = cursor_pt.y as f64 + (target_screen_y as f64 - cursor_pt.y as f64) * lerp_factor;
                            windows_sys::Win32::UI::WindowsAndMessaging::SetCursorPos(next_x.round() as i32, next_y.round() as i32);
                        }
                        return;
                    }
                } else {
                    HUD_LAST_SNAPPED.store(0, std::sync::atomic::Ordering::SeqCst);
                    HUD_LANDED.store(false, std::sync::atomic::Ordering::SeqCst);
                }
            }
            
            let mut best_dist = f64::MAX;
            let mut best_target_id = 0;
            let mut best_target_pos = (0, 0);
            
            let cooldown_lock = HUD_COOLDOWN.get_or_init(|| Mutex::new(None));
            let cooldown = cooldown_lock.lock().unwrap();
            
            for &(id, tx, ty) in &targets {
                if let Some((cooldown_id, escaped_time)) = *cooldown {
                    if id == cooldown_id && escaped_time.elapsed() < std::time::Duration::from_millis(200) {
                        continue;
                    }
                }
                
                let target_screen_x = hud_rect.left + tx;
                let target_screen_y = hud_rect.top + ty;
                let dx = cursor_pt.x - target_screen_x;
                let dy = cursor_pt.y - target_screen_y;
                let dist = ((dx * dx + dy * dy) as f64).sqrt();
                
                if dist < snap_threshold && dist < best_dist {
                    best_dist = dist;
                    best_target_id = id;
                    best_target_pos = (target_screen_x, target_screen_y);
                }
            }
            
            if best_target_id != 0 {
                let lerp_factor = 0.35;
                let next_x = cursor_pt.x as f64 + (best_target_pos.0 as f64 - cursor_pt.x as f64) * lerp_factor;
                let next_y = cursor_pt.y as f64 + (best_target_pos.1 as f64 - cursor_pt.y as f64) * lerp_factor;
                windows_sys::Win32::UI::WindowsAndMessaging::SetCursorPos(next_x.round() as i32, next_y.round() as i32);
                HUD_LAST_SNAPPED.store(best_target_id, std::sync::atomic::Ordering::SeqCst);
                HUD_LANDED.store(false, std::sync::atomic::Ordering::SeqCst);
            }
        }
    }
}

fn scan_titlebar_targets(
    automation: &UIAutomation,
    element: &uiautomation::core::UIElement,
    win_rect: &RECT,
    true_cond: &uiautomation::core::UICondition,
    clickable_types: &[i32],
    depth: usize,
    results: &mut Vec<uiautomation::core::UIElement>,
) {
    if depth > 6 {
        return;
    }
    
    let mut should_traverse = true;
    
    if let Ok(rect) = element.get_bounding_rectangle() {
        let top = rect.get_top() as i32;
        
        // 만약 엘리먼트의 상단이 타이틀바 영역(win_rect.top + 100)보다 아래에 있다면 하위 탐색 중단 및 프루닝
        if top > win_rect.top + 100 {
            return;
        }
        
        if let Ok(ctrl_type) = element.get_control_type() {
            let ctrl_type_val = ctrl_type as i32;
            
            // Document 타입(웹 페이지 본문 등)은 하위 탐색하지 않음
            if ctrl_type_val == 50030 {
                return;
            }
            
            // 클릭 가능한 대상 타입인 경우
            if clickable_types.contains(&ctrl_type_val) {
                should_traverse = false;
                results.push(element.clone());
            }
        }
    }
    
    if should_traverse {
        if let Ok(children) = element.find_all(uiautomation::types::TreeScope::Children, true_cond) {
            for child in &children {
                scan_titlebar_targets(
                    automation,
                    child,
                    win_rect,
                    true_cond,
                    clickable_types,
                    depth + 1,
                    results,
                );
            }
        }
    }
}

pub fn start_global_targets_thread() {
    thread::spawn(|| {
        let automation = match UIAutomation::new() {
            Ok(auto) => auto,
            Err(e) => {
                eprintln!("[Error] Failed to initialize UIAutomation: {:?}", e);
                return;
            }
        };
        
        let clickable_types = [
            50000, // Button
            50002, // Calendar
            50003, // CheckBox
            50005, // ComboBox
            50007, // Hyperlink
            50011, // MenuItem
            50013, // RadioButton
            50031, // TabItem
        ];
        
        let condition = (|| -> Result<uiautomation::core::UICondition, uiautomation::errors::Error> {
            let mut cond = automation.create_property_condition(
                uiautomation::types::UIProperty::ControlType,
                uiautomation::variants::Variant::from(clickable_types[0]),
                None
            )?;
            for &ctrl_type in &clickable_types[1..] {
                let next_cond = automation.create_property_condition(
                    uiautomation::types::UIProperty::ControlType,
                    uiautomation::variants::Variant::from(ctrl_type),
                    None
                )?;
                cond = automation.create_or_condition(cond, next_cond)?;
            }
            Ok(cond)
        })();
        
        let condition = match condition {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[Error] Failed to create UIA condition: {:?}", e);
                return;
            }
        };
        
        let mut last_hwnd = 0;
        let mut query_counter = 0;
        let mut last_query_pt: Option<windows_sys::Win32::Foundation::POINT> = None;
        
        loop {
            thread::sleep(std::time::Duration::from_millis(300));
            
            let (enabled, is_mouse_mode) = {
                if let Some(state_arc) = crate::hook::APP_STATE.get() {
                    let state = state_arc.lock().unwrap();
                    (
                        state.config.settings.global_magnetic_mode.unwrap_or(false),
                        state.is_mouse_mode,
                    )
                } else {
                    (false, false)
                }
            };
            
            if !enabled || !is_mouse_mode {
                last_hwnd = 0;
                query_counter = 0;
                last_query_pt = None;
                if let Some(mutex) = GLOBAL_SNAP_TARGETS.get() {
                    let mut targets = mutex.lock().unwrap();
                    if !targets.is_empty() {
                        targets.clear();
                    }
                }
                continue;
            }
            
            let mut pt = unsafe { std::mem::zeroed() };
            unsafe {
                windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut pt);
            }
            let mut hwnd = unsafe { windows_sys::Win32::UI::WindowsAndMessaging::WindowFromPoint(pt) };
            
            let hud_hwnd = HUD_HWND.get().copied().unwrap_or(0);
            let indicator_hwnd = INDICATOR_HWND.get().copied().unwrap_or(0);
            let main_hwnd = MAIN_HWND.get().copied().unwrap_or(0);
            
            while hwnd != 0 && (hwnd == hud_hwnd || hwnd == indicator_hwnd || hwnd == main_hwnd) {
                hwnd = unsafe { windows_sys::Win32::UI::WindowsAndMessaging::GetWindow(hwnd, 2) };
            }
            
            if hwnd == 0 {
                continue;
            }

            // Get the root ancestor window so we don't lose targets when hovering over different child/sibling controls (e.g., taskbar sections)
            let ancestor = unsafe { windows_sys::Win32::UI::WindowsAndMessaging::GetAncestor(hwnd, 2) }; // GA_ROOT = 2
            if ancestor != 0 {
                hwnd = ancestor;
            }
            
            let mut force_query = false;
            if let Some(lpt) = last_query_pt {
                let dx = pt.x - lpt.x;
                let dy = pt.y - lpt.y;
                let dist = ((dx * dx + dy * dy) as f64).sqrt();
                if dist > 80.0 {
                    force_query = true;
                }
            } else {
                force_query = true;
            }
            
            let force_refresh = FORCE_UIA_REFRESH.swap(false, std::sync::atomic::Ordering::SeqCst);
            if force_refresh {
                force_query = true;
            }
            
            if hwnd == last_hwnd && !force_query {
                query_counter += 1;
                if query_counter < 10 {
                    continue;
                }
            }
            query_counter = 0;
            last_query_pt = Some(pt);
            
            let handle = Handle::from(hwnd as isize);
            let mut found_targets = false;
            
            let mut class_name = [0u16; 256];
            let len = unsafe { GetClassNameW(hwnd, class_name.as_mut_ptr(), 256) };
            let class_str = String::from_utf16_lossy(&class_name[..len as usize]);
            
            let mut win_rect = unsafe { std::mem::zeroed::<RECT>() };
            unsafe {
                GetWindowRect(hwnd, &mut win_rect);
            }
            
            if let Ok(element) = automation.element_from_handle(handle) {
                let is_heavy = class_str == "Chrome_WidgetWin_1"
                    || class_str == "MozillaWindowClass"
                    || class_str == "ApplicationFrameWindow"
                    || class_str == "CabinetWClass";

                let mut elements = Vec::new();
                if is_heavy {
                    if let Ok(true_cond) = automation.create_true_condition() {
                        scan_titlebar_targets(
                            &automation,
                            &element,
                            &win_rect,
                            &true_cond,
                            &clickable_types,
                            0,
                            &mut elements,
                        );
                    }
                } else {
                    if let Ok(descendants) = element.find_all(uiautomation::types::TreeScope::Descendants, &condition) {
                        elements = descendants;
                    }
                }

                let mut new_targets = Vec::new();
                for el in &elements {
                    if let Ok(rect) = el.get_bounding_rectangle() {
                        let left = rect.get_left() as i32;
                        let top = rect.get_top() as i32;
                        let right = rect.get_right() as i32;
                        let bottom = rect.get_bottom() as i32;
                        let w = right - left;
                        let h = bottom - top;
                        
                        if w >= 5 && h >= 5 && w <= 800 && h <= 800 {
                            let is_taskbar = class_str == "Shell_TrayWnd" || class_str == "SecondaryTrayWnd";
                            let mut is_valid = is_taskbar;
                            
                            if !is_valid {
                                // Check if it's in the top-left or top-right title bar area of the application window
                                let in_titlebar_y = top >= win_rect.top - 20 && bottom <= win_rect.top + 80;
                                let in_top_right = in_titlebar_y && right >= win_rect.right - 200 && left <= win_rect.right + 20;
                                let in_top_left = in_titlebar_y && left >= win_rect.left - 20 && right <= win_rect.left + 220;
                                
                                if in_top_right || in_top_left {
                                    is_valid = true;
                                } else {
                                    // Minimize/Maximize/Close ID/Name check as fallback
                                    if let Ok(auto_id) = el.get_automation_id() {
                                        let id_lower = auto_id.to_lowercase();
                                        if id_lower.contains("minimize") || id_lower.contains("maximize") || id_lower.contains("close") || id_lower.contains("restore") {
                                            is_valid = true;
                                        }
                                    }
                                    if !is_valid {
                                        if let Ok(name) = el.get_name() {
                                            let name_lower = name.to_lowercase();
                                            if name_lower.contains("최소화") || name_lower.contains("최대화") || name_lower.contains("닫기") || name_lower.contains("복원") ||
                                               name_lower.contains("minimize") || name_lower.contains("maximize") || name_lower.contains("close") || name_lower.contains("restore") {
                                                is_valid = true;
                                            }
                                        }
                                    }
                                }
                            }
                            
                            if is_valid {
                                new_targets.push((left + w / 2, top + h / 2));
                            }
                        }
                    }
                }
                
                found_targets = !new_targets.is_empty();
                log_debug(&format!("UIA thread found {} targets for HWND={}", new_targets.len(), hwnd));
                new_targets.truncate(200);
                let mut targets = GLOBAL_SNAP_TARGETS.get_or_init(|| Mutex::new(Vec::new())).lock().unwrap();
                *targets = new_targets;
            }
            
            if found_targets {
                last_hwnd = hwnd;
            } else {
                last_hwnd = 0;
            }
        }
    });
}

pub fn check_global_magnetic_snapping() {
    let state_arc = crate::hook::APP_STATE.get();
    let (enabled, is_mouse_mode, is_dragging, features_enabled) = state_arc.map_or((false, false, false, false), |arc| {
        let state = arc.lock().unwrap();
        (
            state.config.settings.global_magnetic_mode.unwrap_or(false),
            state.is_mouse_mode,
            state.is_dragging,
            state.is_pro || state.is_trial,
        )
    });
    
    if !enabled || !is_mouse_mode || is_dragging || !features_enabled {
        if let Some(lock) = LAST_GLOBAL_SNAPPED_POS.get() {
            if let Ok(mut pos) = lock.lock() {
                *pos = None;
            }
        }
        return;
    }
    
    let mut cursor_pt = unsafe { std::mem::zeroed() };
    unsafe {
        windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut cursor_pt);
    }
    
    let snap_threshold = 25.0;
    let release_threshold = 15.0;
    
    static ESCAPED_COOLDOWN: OnceLock<Mutex<Option<((i32, i32), std::time::Instant)>>> = OnceLock::new();
    
    let last_snapped_lock = LAST_GLOBAL_SNAPPED_POS.get_or_init(|| Mutex::new(None));
    let mut last_pos = last_snapped_lock.lock().unwrap();
    
    if let Some((sx, sy, landed)) = *last_pos {
        let (base_speed, should_release, new_dir, hold_duration) = get_movement_status();
        let cooldown_limit = calculate_cooldown_limit(hold_duration);
        
        let accum_lock = GLOBAL_ESCAPE_ACCUM.get_or_init(|| Mutex::new(0.0));
        let mut accum = accum_lock.lock().unwrap();
        
        static LAST_GLOBAL_INPUT: OnceLock<Mutex<std::time::Instant>> = OnceLock::new();
        let last_input_lock = LAST_GLOBAL_INPUT.get_or_init(|| Mutex::new(std::time::Instant::now() - std::time::Duration::from_secs(5)));
        if should_release {
            if let Ok(mut li) = last_input_lock.lock() {
                *li = std::time::Instant::now();
            }
        } else {
            let elapsed = last_input_lock.lock().unwrap().elapsed();
            if elapsed >= std::time::Duration::from_millis(300) {
                *accum = 0.0;
            }
        }
        
        if should_release && new_dir.is_some() {
            let dir = new_dir.as_ref().unwrap();
            let last_jump = LAST_JUMP_TIME.get_or_init(|| Mutex::new(std::time::Instant::now() - std::time::Duration::from_secs(5)));
            let mut can_jump = false;
            if let Ok(lj) = last_jump.try_lock() {
                if lj.elapsed() >= std::time::Duration::from_millis(cooldown_limit) {
                    can_jump = true;
                }
            }
            
            if can_jump {
                let targets_lock = GLOBAL_SNAP_TARGETS.get();
                if let Some(mutex) = targets_lock {
                    let targets = mutex.lock().unwrap();
                    let res = find_adjacent_target(sx, sy, dir, &targets);
                    log_debug(&format!("find_adjacent_target: sx={}, sy={}, dir={}, num_targets={}, result={:?}", sx, sy, dir, targets.len(), res));
                    if let Some((jx, jy)) = res {
                        unsafe {
                            windows_sys::Win32::UI::WindowsAndMessaging::SetCursorPos(jx, jy);
                        }
                        *last_pos = Some((jx, jy, true));
                        
                        if let Ok(mut lj) = last_jump.lock() {
                            *lj = std::time::Instant::now();
                        }
                        
                        let cooldown_lock = ESCAPED_COOLDOWN.get_or_init(|| Mutex::new(None));
                        *cooldown_lock.lock().unwrap() = Some(((sx, sy), std::time::Instant::now()));
                        *accum = 0.0;
                        return;
                    }
                }
            }
        }
        
        if should_release {
            let mut cooldown_active = false;
            let last_jump = LAST_JUMP_TIME.get_or_init(|| Mutex::new(std::time::Instant::now() - std::time::Duration::from_secs(5)));
            if let Ok(lj) = last_jump.try_lock() {
                if lj.elapsed() < std::time::Duration::from_millis(cooldown_limit) {
                    cooldown_active = true;
                }
            }
            
            if cooldown_active {
                *accum = 0.0;
            } else {
                let step = (base_speed * 2.0).max(1.0);
                *accum += step;
            }
            
            let mut dynamic_release_threshold = release_threshold;
            let hold_ms = hold_duration.as_millis();
            if hold_ms > 150 {
                let progress = ((hold_ms - 150) as f64 / 350.0).min(1.0);
                dynamic_release_threshold = release_threshold - (progress * (release_threshold - 4.0));
            }
            
            if *accum >= dynamic_release_threshold {
                log_debug(&format!("Snapping released! accum={}, release_threshold={}", *accum, dynamic_release_threshold));
                *last_pos = None;
                *accum = 0.0;
                
                let cooldown_lock = ESCAPED_COOLDOWN.get_or_init(|| Mutex::new(None));
                *cooldown_lock.lock().unwrap() = Some(((sx, sy), std::time::Instant::now()));
                
                let mut push_dx = 0;
                let mut push_dy = 0;
                if let Some(ref dir) = new_dir {
                    match dir.as_str() {
                        "Left" => push_dx = -15,
                        "Right" => push_dx = 15,
                        "Up" => push_dy = -15,
                        "Down" => push_dy = 15,
                        _ => {}
                    }
                }
                unsafe {
                    windows_sys::Win32::UI::WindowsAndMessaging::SetCursorPos(sx + push_dx, sy + push_dy);
                }
                
                if let Some(state_arc) = crate::hook::APP_STATE.get() {
                    if let Ok(mut state) = state_arc.try_lock() {
                        state.movement_start_time = Some(std::time::Instant::now());
                    }
                }
                return;
            } else {
                unsafe {
                    windows_sys::Win32::UI::WindowsAndMessaging::SetCursorPos(sx, sy);
                }
                *last_pos = Some((sx, sy, true));
                return;
            }
        } else {
            let elapsed = last_input_lock.lock().unwrap().elapsed();
            if elapsed >= std::time::Duration::from_millis(300) {
                *accum = 0.0;
            }
            if landed {
                let dx = cursor_pt.x - sx;
                let dy = cursor_pt.y - sy;
                let dist = ((dx * dx + dy * dy) as f64).sqrt();
                if dist > 3.0 {
                    *last_pos = None;
                    let cooldown_lock = ESCAPED_COOLDOWN.get_or_init(|| Mutex::new(None));
                    *cooldown_lock.lock().unwrap() = Some(((sx, sy), std::time::Instant::now()));
                    return;
                }
                unsafe {
                    windows_sys::Win32::UI::WindowsAndMessaging::SetCursorPos(sx, sy);
                }
                return;
            }
            
            let dx = cursor_pt.x - sx;
            let dy = cursor_pt.y - sy;
            let dist = ((dx * dx + dy * dy) as f64).sqrt();
            
            if dist < 1.5 {
                unsafe {
                    windows_sys::Win32::UI::WindowsAndMessaging::SetCursorPos(sx, sy);
                }
                *last_pos = Some((sx, sy, true));
            } else {
                let lerp_factor = 0.35;
                let next_x = cursor_pt.x as f64 + (sx as f64 - cursor_pt.x as f64) * lerp_factor;
                let next_y = cursor_pt.y as f64 + (sy as f64 - cursor_pt.y as f64) * lerp_factor;
                unsafe {
                    windows_sys::Win32::UI::WindowsAndMessaging::SetCursorPos(next_x.round() as i32, next_y.round() as i32);
                }
            }
            return;
        }
    }
    
    let targets_lock = GLOBAL_SNAP_TARGETS.get();
    if let Some(mutex) = targets_lock {
        let targets = mutex.lock().unwrap();
        let mut best_dist = f64::MAX;
        let mut best_target = (0, 0);
        
        let cooldown_lock = ESCAPED_COOLDOWN.get_or_init(|| Mutex::new(None));
        let cooldown = cooldown_lock.lock().unwrap();
        
        for &(tx, ty) in targets.iter() {
            if let Some(((ex, ey), escaped_time)) = *cooldown {
                let dx_cooldown = tx - ex;
                let dy_cooldown = ty - ey;
                let dist_cooldown = ((dx_cooldown * dx_cooldown + dy_cooldown * dy_cooldown) as f64).sqrt();
                if dist_cooldown < 35.0 && escaped_time.elapsed() < std::time::Duration::from_millis(350) {
                    continue;
                }
            }
            
            let dx = cursor_pt.x - tx;
            let dy = cursor_pt.y - ty;
            let dist = ((dx * dx + dy * dy) as f64).sqrt();
            
            if dist < snap_threshold && dist < best_dist {
                best_dist = dist;
                best_target = (tx, ty);
            }
        }
        
        if best_dist < snap_threshold {
            log_debug(&format!("Snapped to target: ({}, {}) dist={}", best_target.0, best_target.1, best_dist));
            let lerp_factor = 0.35;
            let next_x = cursor_pt.x as f64 + (best_target.0 as f64 - cursor_pt.x as f64) * lerp_factor;
            let next_y = cursor_pt.y as f64 + (best_target.1 as f64 - cursor_pt.y as f64) * lerp_factor;
            unsafe {
                windows_sys::Win32::UI::WindowsAndMessaging::SetCursorPos(next_x.round() as i32, next_y.round() as i32);
            }
            *last_pos = Some((best_target.0, best_target.1, false));
        }
    }
}

pub fn is_currently_snapped() -> bool {
    let hud_snapped = HUD_LAST_SNAPPED.load(std::sync::atomic::Ordering::SeqCst) != 0;
    let global_snapped = if let Some(lock) = LAST_GLOBAL_SNAPPED_POS.get() {
        if let Ok(pos) = lock.lock() {
            pos.is_some()
        } else {
            false
        }
    } else {
        false
    };
    hud_snapped || global_snapped
}

pub fn clear_magnetic_snapping() {
    HUD_LAST_SNAPPED.store(0, std::sync::atomic::Ordering::SeqCst);
    if let Some(lock) = LAST_GLOBAL_SNAPPED_POS.get() {
        if let Ok(mut pos) = lock.lock() {
            *pos = None;
        }
    }
}
