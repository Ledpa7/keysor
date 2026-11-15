#[path = "hud.rs"]
mod hud;

use std::sync::{OnceLock, Mutex};
use std::thread;
use std::sync::atomic::{AtomicU32, Ordering};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM, POINT, SIZE};
use windows_sys::Win32::Graphics::Gdi::{
    BeginPaint, EndPaint, DeleteObject, SelectObject, InvalidateRect, UpdateWindow,
    GetDC, ReleaseDC, CreateCompatibleDC, CreateDIBSection, DeleteDC,
    BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, AC_SRC_OVER, AC_SRC_ALPHA,
};
use windows_sys::Win32::Graphics::GdiPlus::{
    GdiplusStartup, GdiplusStartupInput, GdipCreateFromHDC, GdipDeleteGraphics,
    GdipSetSmoothingMode, GdipCreatePen1, GdipDeletePen, GdipDrawLineI,
    SmoothingModeAntiAlias, GdipSetPenStartCap, GdipSetPenEndCap, GdipSetPenLineJoin,
    GdipCreateLineBrush, GdipCreatePen2, GdipDeleteBrush, PointF, GdipScaleWorldTransform,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    RegisterClassW, CreateWindowExW, DefWindowProcW, ShowWindow, MSG, WNDCLASSW,
    CS_HREDRAW, CS_VREDRAW, WS_POPUP, WS_EX_LAYERED, WS_EX_TRANSPARENT,
    WS_EX_TOPMOST, WS_EX_NOACTIVATE, SW_HIDE, SW_SHOWNA, LWA_ALPHA, WM_PAINT,
    WM_DESTROY, GetMessageW, TranslateMessage, DispatchMessageW,
    SetLayeredWindowAttributes, GetSystemMetrics, WS_EX_APPWINDOW, WM_CLOSE,
    GetSystemMenu, AppendMenuW, MF_SEPARATOR, MF_STRING, WM_SYSCOMMAND, SW_MINIMIZE,
    WS_MINIMIZEBOX, WS_SYSMENU, WM_ERASEBKGND, UpdateLayeredWindow, ULW_ALPHA,
    GetClassNameW, WS_EX_TOOLWINDOW, GetForegroundWindow,
};
use crate::ui::win_uia::{is_currently_snapped, clear_magnetic_snapping};

pub static INDICATOR_HWND: OnceLock<HWND> = OnceLock::new();
pub static HUD_HWND: OnceLock<HWND> = OnceLock::new();
pub static MAIN_HWND: OnceLock<HWND> = OnceLock::new();
static HUD_SUPPRESSED_BY_MOUSE_MODE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
const WM_KEYSOR_MINIMIZE_HUD: u32 = 0x8001;
const WM_KEYSOR_RESTORE_CURSOR: u32 = 0x8002;
const WM_KEYSOR_OPEN_HUD: u32 = 0x8003;
const WM_KEYSOR_CURSOR_STYLE: u32 = 0x8004;
#[cfg(test)]
static STYLE_UPDATE_COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
#[cfg(test)]
static DETAIL_QUERY_COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static CURSOR_STYLE_PENDING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static CURSOR_RESTORE_PENDING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn hud_can_open(inputting_license: bool, suppressed_by_mouse_mode: bool, explicit_open: bool) -> bool {
    !inputting_license && (explicit_open || !suppressed_by_mouse_mode)
}

fn should_show_hud(explicit_open: bool) -> bool {
    hud_can_open(
        IS_INPUTTING_LICENSE.load(Ordering::SeqCst),
        HUD_SUPPRESSED_BY_MOUSE_MODE.load(Ordering::SeqCst),
        explicit_open,
    )
}
static GDIPLUS_TOKEN: OnceLock<usize> = OnceLock::new();
pub static HUD_HOVER: AtomicU32 = AtomicU32::new(0); // 0: none, 1: minimize, 2: close
pub static HUD_LAST_SNAPPED: AtomicU32 = AtomicU32::new(0);
pub static SHOW_ALL_SENS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
pub static CLICK_SCALE: OnceLock<Mutex<f32>> = OnceLock::new();
pub static IS_INPUTTING_LICENSE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
pub static SUSPEND_CURSOR_HIDE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
pub static FORCE_UIA_REFRESH: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static CUR_INDICATOR_POS: OnceLock<Mutex<Option<(f64, f64)>>> = OnceLock::new();

use crate::ui::{KeysorUi, ClickType};

pub struct WindowsGdiUi;

impl WindowsGdiUi {
    pub fn new() -> Self {
        WindowsGdiUi
    }
}

impl KeysorUi for WindowsGdiUi {
    fn start(&self) -> Result<(), String> {
        start_indicator();
        Ok(())
    }

    fn show(&self, visible: bool) {
        if visible {
            show_indicator();
        } else {
            hide_indicator();
        }
    }

    fn update_position(&self) {
        update_indicator_position();
    }

    fn trigger_click_motion(&self, click_type: ClickType) {
        trigger_click_motion(click_type);
    }

    fn check_magnetic_snapping(&self) {
        crate::ui::win_uia::check_magnetic_snapping();
    }

    fn check_global_magnetic_snapping(&self, _is_moving: bool) {
        crate::ui::win_uia::check_global_magnetic_snapping();
    }

    fn is_currently_snapped(&self) -> bool {
        crate::ui::win_uia::is_currently_snapped()
    }
}

pub static CLICK_TYPE: OnceLock<Mutex<ClickType>> = OnceLock::new();

const WM_MOUSELEAVE: u32 = 0x02A3;

pub fn encode_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

static LICENSE_INPUT_RESULT: OnceLock<Mutex<Option<String>>> = OnceLock::new();
static INPUT_DIALOG_ACTIVE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static IS_LANG_EN_GLOBAL: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

static SENS_INPUT_RESULT: OnceLock<Mutex<Option<String>>> = OnceLock::new();
static SENS_INPUT_ACTIVE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static SENS_INPUT_LABEL: OnceLock<Mutex<String>> = OnceLock::new();
static SENS_INPUT_INITIAL: OnceLock<Mutex<String>> = OnceLock::new();

unsafe extern "system" fn input_dialog_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    use windows_sys::Win32::UI::WindowsAndMessaging::*;
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    match msg {
        WM_CREATE => {
            // Static text
            let static_text = if IS_LANG_EN_GLOBAL.load(Ordering::SeqCst) {
                "Please enter your Keysor Pro license key:"
            } else {
                "발급받으신 Keysor Pro 라이선스 키를 입력해주세요:"
            };
            let scale = crate::platform::get_system_controller().get_dpi_scale();

            // Static text
            CreateWindowExW(
                0,
                encode_wide("STATIC").as_ptr(),
                encode_wide(static_text).as_ptr(),
                WS_CHILD | WS_VISIBLE,
                (20.0 * scale) as i32,
                (20.0 * scale) as i32,
                (380.0 * scale) as i32,
                (25.0 * scale) as i32,
                hwnd,
                0,
                GetModuleHandleW(std::ptr::null()),
                std::ptr::null(),
            );

            // Edit control (input text box)
            // ID = 201
            let edit_hwnd = CreateWindowExW(
                WS_EX_CLIENTEDGE,
                encode_wide("EDIT").as_ptr(),
                encode_wide("").as_ptr(),
                WS_CHILD | WS_VISIBLE | ES_AUTOHSCROLL as u32,
                (20.0 * scale) as i32,
                (50.0 * scale) as i32,
                (380.0 * scale) as i32,
                (25.0 * scale) as i32,
                hwnd,
                201,
                GetModuleHandleW(std::ptr::null()),
                std::ptr::null(),
            );
            windows_sys::Win32::UI::Input::KeyboardAndMouse::SetFocus(edit_hwnd);

            // OK Button (ID = 101)
            let ok_text = if IS_LANG_EN_GLOBAL.load(Ordering::SeqCst) { "OK" } else { "확인" };
            CreateWindowExW(
                0,
                encode_wide("BUTTON").as_ptr(),
                encode_wide(ok_text).as_ptr(),
                WS_CHILD | WS_VISIBLE | BS_DEFPUSHBUTTON as u32,
                (200.0 * scale) as i32,
                (90.0 * scale) as i32,
                (90.0 * scale) as i32,
                (30.0 * scale) as i32,
                hwnd,
                101,
                GetModuleHandleW(std::ptr::null()),
                std::ptr::null(),
            );

            // Cancel Button (ID = 102)
            let cancel_text = if IS_LANG_EN_GLOBAL.load(Ordering::SeqCst) { "Cancel" } else { "취소" };
            CreateWindowExW(
                0,
                encode_wide("BUTTON").as_ptr(),
                encode_wide(cancel_text).as_ptr(),
                WS_CHILD | WS_VISIBLE,
                (310.0 * scale) as i32,
                (90.0 * scale) as i32,
                (90.0 * scale) as i32,
                (30.0 * scale) as i32,
                hwnd,
                102,
                GetModuleHandleW(std::ptr::null()),
                std::ptr::null(),
            );
            0
        }
        WM_COMMAND => {
            let wm_id = wparam & 0xFFFF;
            if wm_id == 101 { // OK
                let edit_hwnd = GetDlgItem(hwnd, 201);
                let mut buffer = [0u16; 512];
                let len = GetWindowTextW(edit_hwnd, buffer.as_mut_ptr(), 512);
                let input_str = if len > 0 {
                    String::from_utf16_lossy(&buffer[..len as usize]).trim().to_string()
                } else {
                    "".to_string()
                };
                if let Some(lock) = LICENSE_INPUT_RESULT.get() {
                    if let Ok(mut res) = lock.lock() {
                        *res = Some(input_str);
                    }
                }
                INPUT_DIALOG_ACTIVE.store(false, Ordering::SeqCst);
                DestroyWindow(hwnd);
            } else if wm_id == 102 { // Cancel
                if let Some(lock) = LICENSE_INPUT_RESULT.get() {
                    if let Ok(mut res) = lock.lock() {
                        *res = None;
                    }
                }
                INPUT_DIALOG_ACTIVE.store(false, Ordering::SeqCst);
                DestroyWindow(hwnd);
            }
            0
        }
        WM_CLOSE => {
            if let Some(lock) = LICENSE_INPUT_RESULT.get() {
                if let Ok(mut res) = lock.lock() {
                    *res = None;
                }
            }
            INPUT_DIALOG_ACTIVE.store(false, Ordering::SeqCst);
            DestroyWindow(hwnd);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn prompt_license_input(parent_hwnd: HWND, lang_en: bool) {
    IS_INPUTTING_LICENSE.store(true, Ordering::SeqCst);
    IS_LANG_EN_GLOBAL.store(lang_en, Ordering::SeqCst);

    if let Some(&hud) = HUD_HWND.get() {
        unsafe {
            ShowWindow(hud, SW_HIDE);
            crate::ui::win_uia::clear_magnetic_snapping();
        }
    }

    thread::spawn(move || unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::*;
        use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
        
        let instance = GetModuleHandleW(std::ptr::null());
        let class_name = encode_wide("KeysorInputDlgClass");
        
        static REGISTER_ONCE: std::sync::Once = std::sync::Once::new();
        REGISTER_ONCE.call_once(|| {
            let dlg_class = WNDCLASSW {
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(input_dialog_proc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: instance,
                hIcon: 0,
                hCursor: LoadCursorW(0 as _, 32512 as *const u16),
                hbrBackground: 6 as _, // COLOR_WINDOW + 1
                lpszMenuName: std::ptr::null(),
                lpszClassName: class_name.as_ptr(),
            };
            RegisterClassW(&dlg_class);
        });

        // Initialize OnceLock for results
        let result_lock = LICENSE_INPUT_RESULT.get_or_init(|| Mutex::new(None));
        if let Ok(mut res) = result_lock.lock() {
            *res = None;
        }

        // Center on screen
        let scale = crate::platform::get_system_controller().get_dpi_scale();
        let screen_w = GetSystemMetrics(0);
        let screen_h = GetSystemMetrics(1);
        let dlg_w = (440.0 * scale) as i32;
        let dlg_h = (180.0 * scale) as i32;
        let dlg_x = (screen_w - dlg_w) / 2;
        let dlg_y = (screen_h - dlg_h) / 2;

        let title = if lang_en {
            "Register Keysor Pro License"
        } else {
            "Keysor Pro 라이선스 등록"
        };

        // Disable parent HUD to make it modal
        if let Some(&hud) = HUD_HWND.get() {
            windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow(hud, 0);
        }

        let dlg_hwnd = CreateWindowExW(
            WS_EX_TOPMOST,
            class_name.as_ptr(),
            encode_wide(title).as_ptr(),
            WS_POPUP | WS_CAPTION | WS_SYSMENU,
            dlg_x, dlg_y, dlg_w, dlg_h,
            parent_hwnd,
            0,
            instance,
            std::ptr::null(),
        );

        if dlg_hwnd != 0 {
            ShowWindow(dlg_hwnd, SW_SHOW);
            UpdateWindow(dlg_hwnd);
            INPUT_DIALOG_ACTIVE.store(true, Ordering::SeqCst);
            
            let mut msg: MSG = std::mem::zeroed();
            while INPUT_DIALOG_ACTIVE.load(Ordering::SeqCst) && GetMessageW(&mut msg, 0, 0, 0) > 0 {
                if IsDialogMessageW(dlg_hwnd, &msg) == 0 {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
        }

        IS_INPUTTING_LICENSE.store(false, Ordering::SeqCst);
        // Re-enable HUD and restore visibility
        if let Some(&hud) = HUD_HWND.get() {
            windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow(hud, 1);
            if should_show_hud(false) {
                ShowWindow(hud, SW_SHOWNA);
                windows_sys::Win32::UI::Input::KeyboardAndMouse::SetFocus(hud);
            }
        }

        // Process activation asynchronously
        let key_opt = {
            if let Ok(res) = result_lock.lock() {
                res.clone()
            } else {
                None
            }
        };

        if let Some(key) = key_opt {
            if !key.is_empty() {
                // Save key to config
                if let Some(state_arc) = crate::hook::APP_STATE.get() {
                    let mut config_to_save = None;
                    if let Ok(mut state) = state_arc.lock() {
                        state.config.settings.license_key = Some(key.clone());
                        config_to_save = Some(state.config.clone());
                    }
                    if let Some(cfg) = config_to_save {
                        crate::config::save_config(&cfg);
                    }
                }

                // Async activation
                thread::spawn(move || {
                    match crate::license::activate_license(&key) {
                        Ok(msg) => {
                            crate::license::refresh_entitlements();
                            if let Some(&hud) = HUD_HWND.get() {
                                InvalidateRect(hud, std::ptr::null(), 0);
                            }
                            
                            let success_title = if lang_en { "Success" } else { "성공" };
                            unsafe {
                                MessageBoxW(
                                    parent_hwnd,
                                    encode_wide(&msg).as_ptr(),
                                    encode_wide(success_title).as_ptr(),
                                    0x40, // MB_ICONINFORMATION
                                );
                            }
                        }
                        Err(e) => {
                            let err_title = if lang_en { "Error" } else { "오류" };
                            unsafe {
                                MessageBoxW(
                                    parent_hwnd,
                                    encode_wide(&e).as_ptr(),
                                    encode_wide(err_title).as_ptr(),
                                    0x10, // MB_ICONERROR
                                );
                            }
                        }
                    }
                });
            }
        }
    });
}

unsafe extern "system" fn sens_input_dialog_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    use windows_sys::Win32::UI::WindowsAndMessaging::*;
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    match msg {
        WM_CREATE => {
            let label = SENS_INPUT_LABEL.get().and_then(|m| m.lock().ok()).map(|s| s.clone()).unwrap_or_default();
            let initial = SENS_INPUT_INITIAL.get().and_then(|m| m.lock().ok()).map(|s| s.clone()).unwrap_or_default();
            let scale = crate::platform::get_system_controller().get_dpi_scale();

            // Label text
            CreateWindowExW(
                0,
                encode_wide("STATIC").as_ptr(),
                encode_wide(&label).as_ptr(),
                WS_CHILD | WS_VISIBLE,
                (20.0 * scale) as i32,
                (20.0 * scale) as i32,
                (380.0 * scale) as i32,
                (25.0 * scale) as i32,
                hwnd,
                0,
                GetModuleHandleW(std::ptr::null()),
                std::ptr::null(),
            );

            // Edit control (input text box)
            let edit_hwnd = CreateWindowExW(
                WS_EX_CLIENTEDGE,
                encode_wide("EDIT").as_ptr(),
                encode_wide(&initial).as_ptr(),
                WS_CHILD | WS_VISIBLE | ES_AUTOHSCROLL as u32,
                (20.0 * scale) as i32,
                (50.0 * scale) as i32,
                (380.0 * scale) as i32,
                (25.0 * scale) as i32,
                hwnd,
                201,
                GetModuleHandleW(std::ptr::null()),
                std::ptr::null(),
            );
            windows_sys::Win32::UI::Input::KeyboardAndMouse::SetFocus(edit_hwnd);
            windows_sys::Win32::UI::WindowsAndMessaging::SendMessageW(edit_hwnd, 0x00B1, 0, -1);

            // OK Button (ID = 101)
            let ok_text = if IS_LANG_EN_GLOBAL.load(Ordering::SeqCst) { "OK" } else { "확인" };
            CreateWindowExW(
                0,
                encode_wide("BUTTON").as_ptr(),
                encode_wide(ok_text).as_ptr(),
                WS_CHILD | WS_VISIBLE | BS_DEFPUSHBUTTON as u32,
                (200.0 * scale) as i32,
                (90.0 * scale) as i32,
                (90.0 * scale) as i32,
                (30.0 * scale) as i32,
                hwnd,
                101,
                GetModuleHandleW(std::ptr::null()),
                std::ptr::null(),
            );

            // Cancel Button (ID = 102)
            let cancel_text = if IS_LANG_EN_GLOBAL.load(Ordering::SeqCst) { "Cancel" } else { "취소" };
            CreateWindowExW(
                0,
                encode_wide("BUTTON").as_ptr(),
                encode_wide(cancel_text).as_ptr(),
                WS_CHILD | WS_VISIBLE,
                (310.0 * scale) as i32,
                (90.0 * scale) as i32,
                (90.0 * scale) as i32,
                (30.0 * scale) as i32,
                hwnd,
                102,
                GetModuleHandleW(std::ptr::null()),
                std::ptr::null(),
            );
            0
        }
        WM_COMMAND => {
            let wm_id = wparam & 0xFFFF;
            if wm_id == 101 { // OK
                let edit_hwnd = GetDlgItem(hwnd, 201);
                let mut buffer = [0u16; 512];
                let len = GetWindowTextW(edit_hwnd, buffer.as_mut_ptr(), 512);
                let input_str = if len > 0 {
                    String::from_utf16_lossy(&buffer[..len as usize]).trim().to_string()
                } else {
                    "".to_string()
                };
                if let Some(lock) = SENS_INPUT_RESULT.get() {
                    if let Ok(mut res) = lock.lock() {
                        *res = Some(input_str);
                    }
                }
                SENS_INPUT_ACTIVE.store(false, Ordering::SeqCst);
                DestroyWindow(hwnd);
            } else if wm_id == 102 { // Cancel
                if let Some(lock) = SENS_INPUT_RESULT.get() {
                    if let Ok(mut res) = lock.lock() {
                        *res = None;
                    }
                }
                SENS_INPUT_ACTIVE.store(false, Ordering::SeqCst);
                DestroyWindow(hwnd);
            }
            0
        }
        WM_CLOSE => {
            if let Some(lock) = SENS_INPUT_RESULT.get() {
                if let Ok(mut res) = lock.lock() {
                    *res = None;
                }
            }
            SENS_INPUT_ACTIVE.store(false, Ordering::SeqCst);
            DestroyWindow(hwnd);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn prompt_sens_value_input(parent_hwnd: HWND, target: HudHitTarget, lang_en: bool) {
    IS_LANG_EN_GLOBAL.store(lang_en, Ordering::SeqCst);

    if let Some(&hud) = HUD_HWND.get() {
        unsafe {
            ShowWindow(hud, SW_HIDE);
            crate::ui::win_uia::clear_magnetic_snapping();
        }
    }

    let (label, initial_val) = match target {
        HudHitTarget::EditBaseSpeed => {
            let val = crate::hook::APP_STATE.get().map_or(1.5, |arc| arc.lock().unwrap().config.settings.base_speed);
            let lbl = if lang_en {
                "Enter new Base Speed (0.1 ~ 100.0):"
            } else {
                "새로운 마우스 시작 속도를 입력하세요 (0.1 ~ 100.0):"
            };
            (lbl.to_string(), format!("{:.1}", val))
        }
        HudHitTarget::EditMaxSpeed => {
            let val = crate::hook::APP_STATE.get().map_or(30.0, |arc| arc.lock().unwrap().config.settings.max_speed);
            let lbl = if lang_en {
                "Enter new Max Speed (0.1 ~ 100.0):"
            } else {
                "새로운 마우스 최대 속도를 입력하세요 (0.1 ~ 100.0):"
            };
            (lbl.to_string(), format!("{:.1}", val))
        }
        HudHitTarget::EditAcceleration => {
            let val = crate::hook::APP_STATE.get().map_or(1.5, |arc| arc.lock().unwrap().config.settings.acceleration);
            let lbl = if lang_en {
                "Enter new Acceleration (0.1 ~ 10.0):"
            } else {
                "새로운 마우스 가속도를 입력하세요 (0.1 ~ 10.0):"
            };
            (lbl.to_string(), format!("{:.1}", val))
        }
        _ => return,
    };

    if let Some(lock) = SENS_INPUT_LABEL.get_or_init(|| Mutex::new(String::new())).lock().ok() {
        let mut s = lock;
        *s = label;
    }
    if let Some(lock) = SENS_INPUT_INITIAL.get_or_init(|| Mutex::new(String::new())).lock().ok() {
        let mut s = lock;
        *s = initial_val;
    }

    thread::spawn(move || unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::*;
        use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
        
        let instance = GetModuleHandleW(std::ptr::null());
        let class_name = encode_wide("KeysorSensInputDlgClass");
        
        static REGISTER_ONCE: std::sync::Once = std::sync::Once::new();
        REGISTER_ONCE.call_once(|| {
            let dlg_class = WNDCLASSW {
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(sens_input_dialog_proc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: instance,
                hIcon: 0,
                hCursor: LoadCursorW(0 as _, 32512 as *const u16),
                hbrBackground: 6 as _, // COLOR_WINDOW + 1
                lpszMenuName: std::ptr::null(),
                lpszClassName: class_name.as_ptr(),
            };
            RegisterClassW(&dlg_class);
        });

        let result_lock = SENS_INPUT_RESULT.get_or_init(|| Mutex::new(None));
        if let Ok(mut res) = result_lock.lock() {
            *res = None;
        }

        let scale = crate::platform::get_system_controller().get_dpi_scale();
        let screen_w = GetSystemMetrics(0);
        let screen_h = GetSystemMetrics(1);
        let dlg_w = (440.0 * scale) as i32;
        let dlg_h = (180.0 * scale) as i32;
        let dlg_x = (screen_w - dlg_w) / 2;
        let dlg_y = (screen_h - dlg_h) / 2;

        let title = if lang_en {
            "Edit Sensitivity Value"
        } else {
            "감도 설정 수정"
        };

        if let Some(&hud) = HUD_HWND.get() {
            windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow(hud, 0);
        }

        let dlg_hwnd = CreateWindowExW(
            WS_EX_TOPMOST,
            class_name.as_ptr(),
            encode_wide(title).as_ptr(),
            WS_POPUP | WS_CAPTION | WS_SYSMENU,
            dlg_x, dlg_y, dlg_w, dlg_h,
            parent_hwnd,
            0,
            instance,
            std::ptr::null(),
        );

        if dlg_hwnd != 0 {
            ShowWindow(dlg_hwnd, SW_SHOW);
            UpdateWindow(dlg_hwnd);
            SENS_INPUT_ACTIVE.store(true, Ordering::SeqCst);
            
            let mut msg: MSG = std::mem::zeroed();
            while SENS_INPUT_ACTIVE.load(Ordering::SeqCst) && GetMessageW(&mut msg, 0, 0, 0) > 0 {
                if IsDialogMessageW(dlg_hwnd, &msg) == 0 {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
        }

        if let Some(&hud) = HUD_HWND.get() {
            windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow(hud, 1);
            if should_show_hud(false) {
                ShowWindow(hud, SW_SHOWNA);
                windows_sys::Win32::UI::Input::KeyboardAndMouse::SetFocus(hud);
            }
        }

        let val_opt = {
            if let Ok(res) = result_lock.lock() {
                res.clone()
            } else {
                None
            }
        };

        if let Some(val_str) = val_opt {
            if let Ok(parsed_val) = val_str.parse::<f64>() {
                let mut config_to_save = None;
                let mut is_valid = false;
                
                match target {
                    HudHitTarget::EditBaseSpeed => {
                        if parsed_val >= 0.1 && parsed_val <= 100.0 {
                            if let Some(state_arc) = crate::hook::APP_STATE.get() {
                                if let Ok(mut state) = state_arc.lock() {
                                    state.config.settings.base_speed = parsed_val;
                                    config_to_save = Some(state.config.clone());
                                    is_valid = true;
                                }
                            }
                        }
                    }
                    HudHitTarget::EditMaxSpeed => {
                        if parsed_val >= 0.1 && parsed_val <= 100.0 {
                            if let Some(state_arc) = crate::hook::APP_STATE.get() {
                                if let Ok(mut state) = state_arc.lock() {
                                    state.config.settings.max_speed = parsed_val;
                                    config_to_save = Some(state.config.clone());
                                    is_valid = true;
                                }
                            }
                        }
                    }
                    HudHitTarget::EditAcceleration => {
                        if parsed_val >= 0.1 && parsed_val <= 10.0 {
                            if let Some(state_arc) = crate::hook::APP_STATE.get() {
                                if let Ok(mut state) = state_arc.lock() {
                                    state.config.settings.acceleration = parsed_val;
                                    config_to_save = Some(state.config.clone());
                                    is_valid = true;
                                }
                            }
                        }
                    }
                    _ => {}
                }

                if is_valid {
                    if let Some(cfg) = config_to_save {
                        crate::config::save_config(&cfg);
                    }
                    if let Some(&hud) = HUD_HWND.get() {
                        InvalidateRect(hud, std::ptr::null(), 0);
                    }
                } else {
                    let err_msg = if lang_en {
                        "Invalid value range."
                    } else {
                        "허용 범위를 벗어난 값입니다."
                    };
                    let err_title = if lang_en { "Input Error" } else { "입력 오류" };
                    MessageBoxW(parent_hwnd, encode_wide(err_msg).as_ptr(), encode_wide(err_title).as_ptr(), 0x10);
                }
            } else {
                let err_msg = if lang_en {
                    "Please enter a valid number."
                } else {
                    "올바른 숫자를 입력해 주세요."
                };
                let err_title = if lang_en { "Input Error" } else { "입력 오류" };
                MessageBoxW(parent_hwnd, encode_wide(err_msg).as_ptr(), encode_wide(err_title).as_ptr(), 0x10);
            }
        }
    });
}

fn open_notepad_config() {
    let yaml_path = crate::config::get_config_path();
    std::process::Command::new("notepad.exe")
        .arg(yaml_path)
        .spawn()
        .ok();
}

fn open_explorer_config_folder() {
    let mut folder_path = crate::config::get_config_path();
    folder_path.pop();
    std::process::Command::new("explorer.exe")
        .arg(folder_path)
        .spawn()
        .ok();
}

fn get_startup_shortcut_path() -> std::path::PathBuf {
    let mut path = std::env::var("APPDATA")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("."));
    path.push("Microsoft");
    path.push("Windows");
    path.push("Start Menu");
    path.push("Programs");
    path.push("Startup");
    path.push("keysor.lnk");
    path
}

fn toggle_startup_shortcut() {
    let shortcut_path = get_startup_shortcut_path();
    if shortcut_path.exists() {
        std::fs::remove_file(&shortcut_path).ok();
        unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::MessageBoxW(
                0,
                encode_wide("시작 프로그램에서 키소어(Keysor)가 정상 해제되었습니다.").as_ptr(),
                encode_wide("Keysor 알림").as_ptr(),
                0,
            );
        }
    } else {
        if let Ok(exe_path) = std::env::current_exe() {
            let exe_path_str = exe_path.to_string_lossy();
            let exe_dir = exe_path.parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
            let shortcut_path_str = shortcut_path.to_string_lossy();
            
            let script = format!(
                "$WshShell = New-Object -ComObject WScript.Shell; \
                 $Shortcut = $WshShell.CreateShortcut('{}'); \
                 $Shortcut.TargetPath = '{}'; \
                 $Shortcut.WorkingDirectory = '{}'; \
                 $Shortcut.Save()",
                shortcut_path_str, exe_path_str, exe_dir
            );
            
            #[cfg(target_os = "windows")]
            {
                use std::os::windows::process::CommandExt;
                std::process::Command::new("powershell")
                    .args(&["-Command", &script])
                    .creation_flags(0x08000000) // CREATE_NO_WINDOW
                    .status()
                    .ok();
            }
                
            unsafe {
                windows_sys::Win32::UI::WindowsAndMessaging::MessageBoxW(
                    0,
                    encode_wide("시작 프로그램에 키소어(Keysor)가 등록되었습니다.\n부팅 시 자동 실행됩니다.").as_ptr(),
                    encode_wide("Keysor 알림").as_ptr(),
                    0,
                );
            }
        }
    }
}

unsafe extern "system" fn main_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        if let Some(result) = crate::win_shortcuts::handle_message(hwnd, msg, wparam) {
            return result;
        }
        match msg {
            WM_KEYSOR_MINIMIZE_HUD => {
                ShowWindow(hwnd, SW_MINIMIZE);
                if let Some(&hud) = HUD_HWND.get() {
                    ShowWindow(hud, SW_HIDE);
                }
                restore_native_pointer_on_ui_thread();
                if let Some(&indicator) = INDICATOR_HWND.get() {
                    update_indicator_layered_image(indicator);
                }
                update_indicator_position();
                0
            }
            WM_KEYSOR_CURSOR_STYLE => {
                #[cfg(test)]
                STYLE_UPDATE_COUNT.fetch_add(1, Ordering::Relaxed);
                CURSOR_STYLE_PENDING.store(false, Ordering::SeqCst);
                crate::cursor_scheme::set_custom(custom_cursor_needed_now());
                0
            }
            WM_KEYSOR_OPEN_HUD => {
                if should_show_hud(true) {
                    ShowWindow(hwnd, windows_sys::Win32::UI::WindowsAndMessaging::SW_RESTORE);
                    if let Some(&hud) = HUD_HWND.get() {
                        ShowWindow(hud, windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOW);
                        windows_sys::Win32::UI::WindowsAndMessaging::SetForegroundWindow(hud);
                    }
                }
                0
            }
            WM_KEYSOR_RESTORE_CURSOR => {
                CURSOR_RESTORE_PENDING.store(false, Ordering::SeqCst);
                restore_native_pointer_on_ui_thread();
                0
            }
            WM_SYSCOMMAND => {
                let cmd_id = wparam & 0xFFFF;
                println!("[Debug] main_wnd_proc WM_SYSCOMMAND: cmd_id={}", cmd_id);
                match cmd_id {
                    1001 => {
                        open_notepad_config();
                        return 0;
                    }
                    1002 => {
                        open_explorer_config_folder();
                        return 0;
                    }
                    1003 => {
                        toggle_startup_shortcut();
                        return 0;
                    }
                    _ => {}
                }
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
            windows_sys::Win32::UI::WindowsAndMessaging::WM_SIZE => {
                let state = wparam;
                println!("[Debug] main_wnd_proc WM_SIZE: state={}", state);
                if let Some(&hud) = HUD_HWND.get() {
                    if state == 1 { // SIZE_MINIMIZED
                        ShowWindow(hud, SW_HIDE);
                        crate::ui::win_uia::clear_magnetic_snapping();
                    } else if state == 0 { // SIZE_RESTORED
                        if should_show_hud(true) {
                            ShowWindow(hud, SW_SHOWNA);
                        }
                    }
                }
                0
            }
            windows_sys::Win32::UI::WindowsAndMessaging::WM_ACTIVATE => {
                let state = wparam & 0xFFFF;
                println!("[Debug] main_wnd_proc WM_ACTIVATE: state={}", state);
                if state != 0 { // Activated
                    if let Some(&hud) = HUD_HWND.get() {
                        if should_show_hud(false) {
                            ShowWindow(hud, SW_SHOWNA);
                        }
                    }
                }
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
            WM_CLOSE | WM_DESTROY => {
                crate::hook::cleanup_hook();
                force_restore_system_cursor();
                std::process::exit(0);
            }
            0x0011 | 0x0016 => { // WM_QUERYENDSESSION | WM_ENDSESSION (Windows Shutdown / Restart)
                crate::hook::cleanup_hook();
                force_restore_system_cursor();
                1
            }
            0x02B1 => { // WM_WTSSESSION_CHANGE
                force_restore_system_cursor();
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
            0x007E | 0x0218 => { // WM_DISPLAYCHANGE | WM_POWERBROADCAST
                force_restore_system_cursor();
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
            _ => {
                if msg == windows_sys::Win32::UI::WindowsAndMessaging::WM_SHOWWINDOW {
                    println!("[Debug] main_wnd_proc WM_SHOWWINDOW: wparam={}", wparam);
                }
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
        }
    }
}

unsafe extern "system" fn indicator_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        match msg {
            WM_PAINT => {
                let mut ps = std::mem::zeroed();
                let _hdc = BeginPaint(hwnd, &mut ps);
                EndPaint(hwnd, &ps);
                0
            }
            WM_ERASEBKGND => {
                1
            }
            WM_DESTROY => {
                0
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

pub fn trigger_click_motion(click_type: ClickType) {
    let scale_lock = CLICK_SCALE.get_or_init(|| Mutex::new(1.0));
    if let Ok(mut scale) = scale_lock.lock() {
        *scale = 0.7; // 작아짐 (0.7)
    }
    let type_lock = CLICK_TYPE.get_or_init(|| Mutex::new(ClickType::None));
    if let Ok(mut t) = type_lock.lock() {
        *t = click_type;
    }
    // 즉시 인디케이터 갱신을 트리거
    if let Some(&hwnd) = INDICATOR_HWND.get() {
        unsafe {
            update_indicator_layered_image(hwnd);
            InvalidateRect(hwnd, std::ptr::null(), 0);
            UpdateWindow(hwnd);
        }
    }
}

pub fn get_dpi_scale_at_cursor() -> f64 {
    type GetDpiForMonitorFn = unsafe extern "system" fn(
        windows_sys::Win32::Graphics::Gdi::HMONITOR,
        u32,
        *mut u32,
        *mut u32,
    ) -> i32;

    static GET_DPI_FN: OnceLock<Option<GetDpiForMonitorFn>> = OnceLock::new();
    let get_dpi = GET_DPI_FN.get_or_init(|| unsafe {
        let shcore = windows_sys::Win32::System::LibraryLoader::LoadLibraryA(b"Shcore.dll\0".as_ptr());
        if shcore != 0 {
            let proc = windows_sys::Win32::System::LibraryLoader::GetProcAddress(shcore, b"GetDpiForMonitor\0".as_ptr());
            if let Some(p) = proc {
                return Some(std::mem::transmute(p));
            }
        }
        None
    });

    unsafe {
        let mut pt = std::mem::zeroed();
        windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut pt);
        let h_monitor = windows_sys::Win32::Graphics::Gdi::MonitorFromPoint(
            pt,
            windows_sys::Win32::Graphics::Gdi::MONITOR_DEFAULTTONEAREST,
        );

        if h_monitor != 0 {
            if let Some(func) = get_dpi {
                let mut dpi_x = 96;
                let mut dpi_y = 96;
                if func(h_monitor, 0, &mut dpi_x, &mut dpi_y) == 0 && dpi_x > 0 {
                    return dpi_x as f64 / 96.0;
                }
            }
        }
    }

    crate::platform::get_system_controller().get_dpi_scale()
}

fn get_indicator_dpi_scale() -> f64 {
    let sys_dpi = get_dpi_scale_at_cursor();
    if sys_dpi >= 1.8 {
        2.0    // 200% DPI 이상 -> 2.0배 (64px)
    } else if sys_dpi >= 1.4 {
        1.5    // 150% DPI -> 1.5배 (48px)
    } else if sys_dpi >= 1.15 {
        1.25   // 125% DPI -> 1.25배 (40px)
    } else {
        1.0    // 100% DPI -> 1.0배 (32px)
    }
}

fn draw_cursor_artwork(hdc_mem: isize, dpi_scale: f64, scale_val: f32, offset: (f32, f32)) {
    let scale_coord = |coord: i32, center: i32| -> i32 {
        center + ((coord - center) as f32 * scale_val).round() as i32
    };
    unsafe {
        // Draw cursor using GDI+
        let mut graphics = std::ptr::null_mut();
        if GdipCreateFromHDC(hdc_mem, &mut graphics) == 0 {
            GdipSetSmoothingMode(graphics, SmoothingModeAntiAlias);
            GdipScaleWorldTransform(graphics, dpi_scale as f32, dpi_scale as f32, 0);
            windows_sys::Win32::Graphics::GdiPlus::GdipTranslateWorldTransform(graphics, offset.0, offset.1, 0);
        
            // 1. Draw high-contrast black border first (4.0px width)
            let mut black_pen = std::ptr::null_mut();
            if GdipCreatePen1(0xFF000000, 4.0, 0, &mut black_pen) == 0 {
                GdipSetPenStartCap(black_pen, 2); // LineCapRound = 2
                GdipSetPenEndCap(black_pen, 2);   // LineCapRound = 2
                GdipSetPenLineJoin(black_pen, 2);  // LineJoinRound = 2
        
                GdipDrawLineI(graphics, black_pen, 16, 16, scale_coord(19, 16), scale_coord(29, 16));
                GdipDrawLineI(graphics, black_pen, 16, 16, scale_coord(30, 16), scale_coord(27, 16));
                GdipDrawLineI(graphics, black_pen, scale_coord(18, 16), scale_coord(25, 16), scale_coord(25, 16), scale_coord(14, 16));
        
                GdipDeletePen(black_pen);
            }
        
            // 2. Determine gradient colors based on state
            let (is_dragging, is_scrolling, is_snapped) = if let Some(state_arc) = crate::hook::APP_STATE.get() {
                if let Ok(state) = state_arc.try_lock() {
                    (state.is_dragging, !state.active_scroll_keys.is_empty(), is_currently_snapped())
                } else {
                    (false, false, false)
                }
            } else {
                (false, false, false)
            };
        
            let click_t = if let Some(type_lock) = CLICK_TYPE.get() {
                if let Ok(t) = type_lock.lock() {
                    *t
                } else {
                    ClickType::None
                }
            } else {
                ClickType::None
            };
        
            let (start_color, end_color) = if click_t == ClickType::Left {
                (0xFFFF4500, 0xFFFF0000) // Spacebar: Neon Orange to Red
            } else if click_t == ClickType::Right {
                (0xFFFFFF00, 0xFFFFAA00) // G Key: Yellow to Gold/Orange
            } else if click_t == ClickType::Scroll {
                (0xFF00E5FF, 0xFF0055FF) // QERF: Neon Cyan to Blue
            } else if is_dragging {
                (0xFFFF4500, 0xFFFF007F) // Dragging: Neon Orange to Pink-Red
            } else if is_scrolling {
                (0xFF00E5FF, 0xFF0055FF) // Scrolling: Neon Cyan to Blue
            } else if is_snapped {
                (0xFFFF00FF, 0xFFFF007F) // Snapped: Neon Magenta Gradient
            } else {
                (0xFF2FFFAD, 0xFF004D20) // Normal: Neon Green to Dark Green Gradient
            };
        
            // 3. Create linear gradient brush & pen
            let p1 = PointF { X: 16.0, Y: 16.0 };
            let p2 = PointF { X: 16.0 + 14.0 * scale_val, Y: 16.0 + 11.0 * scale_val };
            let mut brush = std::ptr::null_mut();
            if GdipCreateLineBrush(&p1, &p2, start_color, end_color, 3, &mut brush) == 0 {
                let mut gradient_pen = std::ptr::null_mut();
                if GdipCreatePen2(brush, 2.5, 0, &mut gradient_pen) == 0 {
                    GdipSetPenStartCap(gradient_pen, 2); // LineCapRound = 2
                    GdipSetPenEndCap(gradient_pen, 2);   // LineCapRound = 2
                    GdipSetPenLineJoin(gradient_pen, 2);  // LineJoinRound = 2
        
                    GdipDrawLineI(graphics, gradient_pen, 16, 16, scale_coord(19, 16), scale_coord(29, 16));
                    GdipDrawLineI(graphics, gradient_pen, 16, 16, scale_coord(30, 16), scale_coord(27, 16));
                    GdipDrawLineI(graphics, gradient_pen, scale_coord(18, 16), scale_coord(25, 16), scale_coord(25, 16), scale_coord(14, 16));
        
                    GdipDeletePen(gradient_pen);
                }
                GdipDeleteBrush(brush);
            }
        
            GdipDeleteGraphics(graphics);
        }
    }
}

/// Render at the final size, with the same vector strokes and anti-aliasing as the overlay.
pub(crate) fn render_native_cursor_32() -> isize {
    unsafe {
        let screen = GetDC(0);
        let dc = CreateCompatibleDC(screen);
        let mut bmi: BITMAPINFO = std::mem::zeroed();
        bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = 128;
        bmi.bmiHeader.biHeight = -128;
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = BI_RGB;
        let mut bits = std::ptr::null_mut();
        let high = CreateDIBSection(dc, &bmi, DIB_RGB_COLORS, &mut bits, 0, 0);
        let mut cursor = 0;
        if high != 0 && !bits.is_null() {
            let old = SelectObject(dc, high);
            std::ptr::write_bytes(bits, 0, 128 * 128 * 4);
            // Keep all caps inside the canvas and move the hotspot with the artwork.
            let scale = CLICK_SCALE.get().and_then(|s| s.lock().ok().map(|s| *s)).unwrap_or(1.0);
            draw_cursor_artwork(dc, 4.0, scale, (-2.0, -1.0));
            windows_sys::Win32::Graphics::Gdi::GdiFlush();
            SelectObject(dc, old);
            bmi.bmiHeader.biWidth = 32;
            bmi.bmiHeader.biHeight = -32;
            let mut output = std::ptr::null_mut();
            let bitmap = CreateDIBSection(dc, &bmi, DIB_RGB_COLORS, &mut output, 0, 0);
            if bitmap != 0 && !output.is_null() {
                let source = std::slice::from_raw_parts(bits as *const u32, 128 * 128);
                let target = std::slice::from_raw_parts_mut(output as *mut u32, 32 * 32);
                // Average premultiplied BGRA coverage, including alpha, rather than
                // stretching an already rasterized cursor with CopyImage.
                for y in 0..32 {
                    for x in 0..32 {
                        let mut channels = [0u32; 4];
                        for dy in 0..4 { for dx in 0..4 {
                            let pixel = source[(y * 4 + dy) * 128 + x * 4 + dx];
                            for c in 0..4 { channels[c] += (pixel >> (c * 8)) & 255; }
                        }}
                        target[y * 32 + x] = (0..4).fold(0, |pixel, c| pixel | (((channels[c] + 8) / 16) << (c * 8)));
                    }
                }
                let mask_bits = crate::cursor_scheme::transparency_mask(target, 32, 32);
                let mask = windows_sys::Win32::Graphics::Gdi::CreateBitmap(32, 32, 1, 1, mask_bits.as_ptr() as *const _);
                if mask != 0 {
                    let info = windows_sys::Win32::UI::WindowsAndMessaging::ICONINFO {
                        fIcon: 0, xHotspot: 14, yHotspot: 15, hbmMask: mask, hbmColor: bitmap,
                    };
                    cursor = windows_sys::Win32::UI::WindowsAndMessaging::CreateIconIndirect(&info);
                    DeleteObject(mask);
                }
            }
            if bitmap != 0 { DeleteObject(bitmap); }
        }
        if high != 0 { DeleteObject(high); }
        DeleteDC(dc);
        ReleaseDC(0, screen);
        cursor
    }
}

fn update_indicator_layered_image(hwnd: HWND) {
    unsafe {
        let dpi_scale = get_indicator_dpi_scale();
        let width = (32.0 * dpi_scale) as i32;
        let height = (32.0 * dpi_scale) as i32;

        let scale_val = if let Some(scale_lock) = CLICK_SCALE.get() {
            if let Ok(scale) = scale_lock.lock() {
                *scale
            } else {
                1.0
            }
        } else {
            1.0
        };

        let hdc_screen = GetDC(0);
        let hdc_mem = CreateCompatibleDC(hdc_screen);

        let mut bmi: BITMAPINFO = std::mem::zeroed();
        bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = width;
        bmi.bmiHeader.biHeight = -height; // Top-down
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = BI_RGB;

        let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
        let h_bitmap = CreateDIBSection(
            hdc_mem,
            &bmi,
            DIB_RGB_COLORS,
            &mut bits,
            0,
            0,
        );

        if h_bitmap != 0 {
            let old_bitmap = SelectObject(hdc_mem, h_bitmap);

            // Fill with completely transparent pixels (alpha = 0)
            std::ptr::write_bytes(bits, 0, (width * height * 4) as usize);

            draw_cursor_artwork(hdc_mem, dpi_scale, scale_val, (0.0, 0.0));

            let pt_src = POINT { x: 0, y: 0 };
            let size_wnd = SIZE { cx: width, cy: height };

            let blend = windows_sys::Win32::Graphics::Gdi::BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as u8,
            };

            UpdateLayeredWindow(
                hwnd,
                hdc_screen,
                std::ptr::null(),
                &size_wnd,
                hdc_mem,
                &pt_src,
                0,
                &blend,
                ULW_ALPHA,
            );

            SelectObject(hdc_mem, old_bitmap);
            // Reuse the exact visible Keysor artwork for the native remote cursor.
            windows_sys::Win32::Graphics::Gdi::GdiFlush();
            crate::cursor_scheme::cache_artwork(h_bitmap, width, height, bits);
            DeleteObject(h_bitmap);
        }

        DeleteDC(hdc_mem);
        ReleaseDC(0, hdc_screen);
    }
}

fn adjust_sensitivity(delta: f64) {
    let mut config_to_save = None;
    if let Some(state_arc) = crate::hook::APP_STATE.get() {
        if let Ok(mut state) = state_arc.lock() {
            let mut new_speed = state.config.settings.base_speed + delta;
            if new_speed < 0.1 {
                new_speed = 0.1;
            } else if new_speed > 10.0 {
                new_speed = 10.0;
            }
            new_speed = (new_speed * 10.0).round() / 10.0;
            
            state.config.settings.base_speed = new_speed;
            config_to_save = Some(state.config.clone());
        }
    }
    if let Some(cfg) = config_to_save {
        crate::config::save_config(&cfg);
    }
}

fn toggle_pixel_mode() {
    let mut config_to_save = None;
    if let Some(state_arc) = crate::hook::APP_STATE.get() {
        if let Ok(mut state) = state_arc.lock() {
            let cur = state.config.settings.pixel_mode.unwrap_or(false);
            state.config.settings.pixel_mode = Some(!cur);
            config_to_save = Some(state.config.clone());
        }
    }
    if let Some(cfg) = config_to_save {
        crate::config::save_config(&cfg);
    }
}

fn toggle_language() {
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

fn toggle_magnet() {
    let mut config_to_save = None;
    let mut clear_snap = false;
    if let Some(state_arc) = crate::hook::APP_STATE.get() {
        if let Ok(mut state) = state_arc.lock() {
            let cur = state.config.settings.magnetic_mode.unwrap_or(false);
            let next_mode = !cur;
            state.config.settings.magnetic_mode = Some(next_mode);
            state.config.settings.global_magnetic_mode = Some(next_mode); // Sync global magnet mode too!
            if !next_mode {
                clear_snap = true;
            }
            config_to_save = Some(state.config.clone());
        }
    }
    if clear_snap {
        clear_magnetic_snapping();
    }
    if let Some(cfg) = config_to_save {
        crate::config::save_config(&cfg);
    }
}

fn toggle_autostart() -> bool {
    let controller = crate::platform::get_system_controller();
    let is_autostart = check_autostart_status_win();
    let next_status = !is_autostart;
    let _ = controller.register_startup(next_status);
    next_status
}

fn check_autostart_status_win() -> bool {
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::System::Registry::*;
        unsafe {
            let mut key: HKEY = 0;
            let subkey = encode_wide("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
            let res = RegOpenKeyExW(HKEY_CURRENT_USER, subkey.as_ptr(), 0, KEY_READ, &mut key);
            if res == 0 && key != 0 {
                let val_name = encode_wide("Keysor");
                let mut type_reg: u32 = 0;
                let mut cb_data: u32 = 0;
                let query_res = RegQueryValueExW(key, val_name.as_ptr(), std::ptr::null_mut(), &mut type_reg, std::ptr::null_mut(), &mut cb_data);
                RegCloseKey(key);
                if query_res == 0 && cb_data > 0 {
                    return true;
                }
            }
        }
    }
    let shortcut_path = get_startup_shortcut_path();
    shortcut_path.exists()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
enum HudHitTarget {
    None = 0,
    Minimize = 1,
    Close = 2,
    DecSensitivity = 3,
    IncSensitivity = 4,
    TogglePixelMode = 5,
    ToggleLanguage = 6,
    ToggleMagnet = 7,
    ToggleDetail = 8,
    ToggleAutoStart = 9,
    BuyProTop = 10,
    LicenseTop = 11,
    EditBaseSpeed = 12,
    EditMaxSpeed = 13,
    EditAcceleration = 14,
    Support = 15,
}

fn classify_hit_target(x: i16, y: i16, features_enabled: bool, is_pro: bool) -> HudHitTarget {
    if (600..=778).contains(&x) && (404..=426).contains(&y) {
        return HudHitTarget::Support;
    }
    if !features_enabled {
        if y >= 80 && y <= 344 && x >= 638 && x <= 778 {
            return HudHitTarget::ToggleMagnet;
        }
    }

    if SHOW_ALL_SENS.load(Ordering::SeqCst) && features_enabled {
        if y >= 114 && y <= 150 {
            if y >= 118 && y <= 133 {
                if x >= 646 && x <= 708 {
                    return HudHitTarget::EditBaseSpeed;
                } else if x >= 708 && x <= 770 {
                    return HudHitTarget::EditMaxSpeed;
                }
            } else if y >= 134 && y <= 150 {
                return HudHitTarget::EditAcceleration;
            }
        }
    }

    if y >= 10 && y <= 30 {
        if x >= 773 && x <= 798 {
            return HudHitTarget::Close;
        } else if x >= 742 && x <= 767 {
            return HudHitTarget::Minimize;
        }
    }
    if y >= 30 && y <= 54 {
        if x >= 662 && x <= 712 {
            return HudHitTarget::ToggleLanguage;
        } else if x >= 430 && x <= 540 {
            if !is_pro {
                return HudHitTarget::BuyProTop;
            }
        } else if x >= 546 && x <= 656 {
            return HudHitTarget::LicenseTop;
        }
    }
    if y >= 154 && y <= 186 {
        if x >= 648 && x <= 703 {
            return HudHitTarget::DecSensitivity;
        } else if x >= 713 && x <= 768 {
            return HudHitTarget::IncSensitivity;
        }
    }
    if y >= 191 && y <= 223 {
        if x >= 648 && x <= 768 {
            return HudHitTarget::TogglePixelMode;
        }
    }
    if y >= 228 && y <= 260 {
        if x >= 648 && x <= 768 {
            return HudHitTarget::ToggleMagnet;
        }
    }
    if y >= 265 && y <= 297 {
        if x >= 648 && x <= 768 {
            return HudHitTarget::ToggleAutoStart;
        }
    }
    if y >= 302 && y <= 334 {
        if x >= 648 && x <= 768 {
            return HudHitTarget::ToggleDetail;
        }
    }
    HudHitTarget::None
}

unsafe extern "system" fn hud_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        match msg {
            WM_PAINT => hud::paint(hwnd),

            windows_sys::Win32::UI::WindowsAndMessaging::WM_MOUSEMOVE => {
                let x = (lparam & 0xFFFF) as i16;
                let y = ((lparam >> 16) & 0xFFFF) as i16;
                
                let scale = crate::platform::get_system_controller().get_dpi_scale();
                let lx = (x as f64 / scale) as i16;
                let ly = (y as f64 / scale) as i16;
                
                let (is_pro, is_trial) = {
                    let state_arc = crate::hook::APP_STATE.get();
                    state_arc.map_or((false, false), |arc| {
                        let state = arc.lock().unwrap();
                        (state.is_pro, state.is_trial)
                    })
                };
                let prev_hover = HUD_HOVER.load(Ordering::SeqCst);
                let hit = classify_hit_target(lx, ly, is_pro || is_trial, is_pro);
                let new_hover = hit as u32;
                
                if new_hover != prev_hover {
                    HUD_HOVER.store(new_hover, Ordering::SeqCst);
                    InvalidateRect(hwnd, std::ptr::null(), 0);
                    
                    let mut tme = windows_sys::Win32::UI::Input::KeyboardAndMouse::TRACKMOUSEEVENT {
                        cbSize: std::mem::size_of::<windows_sys::Win32::UI::Input::KeyboardAndMouse::TRACKMOUSEEVENT>() as u32,
                        dwFlags: windows_sys::Win32::UI::Input::KeyboardAndMouse::TME_LEAVE,
                        hwndTrack: hwnd,
                        dwHoverTime: 0,
                    };
                    windows_sys::Win32::UI::Input::KeyboardAndMouse::TrackMouseEvent(&mut tme);
                }
                0
            }
            WM_MOUSELEAVE => {
                let prev_hover = HUD_HOVER.load(Ordering::SeqCst);
                if prev_hover != 0 {
                    HUD_HOVER.store(0, Ordering::SeqCst);
                    InvalidateRect(hwnd, std::ptr::null(), 0);
                }
                0
            }
            windows_sys::Win32::UI::WindowsAndMessaging::WM_LBUTTONDOWN => {
                let x = (lparam & 0xFFFF) as i16;
                let y = ((lparam >> 16) & 0xFFFF) as i16;
                
                let scale = crate::platform::get_system_controller().get_dpi_scale();
                let lx = (x as f64 / scale) as i16;
                let ly = (y as f64 / scale) as i16;

                let (is_pro, is_trial) = {
                    let state_arc = crate::hook::APP_STATE.get();
                    state_arc.map_or((false, false), |arc| {
                        let state = arc.lock().unwrap();
                        (state.is_pro, state.is_trial)
                    })
                };
                let lang_en = {
                    let state_arc = crate::hook::APP_STATE.get();
                    state_arc.map_or(false, |arc| arc.lock().unwrap().config.settings.lang_en.unwrap_or(false))
                };
                
                let hit = classify_hit_target(lx, ly, is_pro || is_trial, is_pro);
                match hit {
                    HudHitTarget::Minimize => {
                        if let Some(&main_hwnd) = MAIN_HWND.get() {
                            ShowWindow(main_hwnd, SW_MINIMIZE);
                        }
                    }
                    HudHitTarget::Close => {
                        crate::restore_windows_system_cursor();
                        if let Some(&main_hwnd) = MAIN_HWND.get() {
                            windows_sys::Win32::UI::WindowsAndMessaging::SendMessageW(
                                main_hwnd,
                                windows_sys::Win32::UI::WindowsAndMessaging::WM_CLOSE,
                                0,
                                0,
                             );
                        }
                    }
                    HudHitTarget::ToggleLanguage => {
                        toggle_language();
                        InvalidateRect(hwnd, std::ptr::null(), 0);
                    }
                    HudHitTarget::DecSensitivity | HudHitTarget::IncSensitivity | HudHitTarget::TogglePixelMode | HudHitTarget::ToggleMagnet | HudHitTarget::ToggleAutoStart | HudHitTarget::ToggleDetail
                    | HudHitTarget::EditBaseSpeed | HudHitTarget::EditMaxSpeed | HudHitTarget::EditAcceleration => {
                        if !(is_pro || is_trial) {
                            let msg_text = if lang_en {
                                "This feature is only available in Keysor Pro.\n\nWould you like to purchase a Pro license?"
                            } else {
                                "이 기능은 키서 프로(Keysor Pro) 전용 기능입니다.\n\n지금 프로 라이선스를 구매하시겠습니까?"
                            };
                            let title_text = if lang_en { "Pro Feature Locked" } else { "프로 기능 잠김" };
                            let ret = windows_sys::Win32::UI::WindowsAndMessaging::MessageBoxW(
                                hwnd,
                                encode_wide(msg_text).as_ptr(),
                                encode_wide(title_text).as_ptr(),
                                1, // MB_OKCANCEL
                            );
                            if ret == 1 { // IDOK
                                std::process::Command::new("cmd")
                                    .args(&["/C", "start", "https://keysor.vercel.app/#pricing"])
                                    .spawn()
                                    .ok();
                            }
                        } else {
                            match hit {
                                HudHitTarget::DecSensitivity => {
                                    adjust_sensitivity(-0.1);
                                }
                                HudHitTarget::IncSensitivity => {
                                    adjust_sensitivity(0.1);
                                }
                                HudHitTarget::TogglePixelMode => {
                                    toggle_pixel_mode();
                                }
                                HudHitTarget::ToggleMagnet => {
                                    toggle_magnet();
                                }
                                HudHitTarget::ToggleAutoStart => {
                                    toggle_autostart();
                                }
                                HudHitTarget::ToggleDetail => {
                                    let prev = SHOW_ALL_SENS.load(Ordering::SeqCst);
                                    SHOW_ALL_SENS.store(!prev, Ordering::SeqCst);
                                }
                                HudHitTarget::EditBaseSpeed | HudHitTarget::EditMaxSpeed | HudHitTarget::EditAcceleration => {
                                    prompt_sens_value_input(hwnd, hit, lang_en);
                                }
                                _ => {}
                            }
                        }
                        InvalidateRect(hwnd, std::ptr::null(), 0);
                    }

                    HudHitTarget::BuyProTop => {
                        std::process::Command::new("cmd")
                            .args(&["/C", "start", "https://keysor.vercel.app/#pricing"])
                            .spawn()
                            .ok();
                        InvalidateRect(hwnd, std::ptr::null(), 0);
                    }
                    HudHitTarget::Support => {
                        crate::support::show(lang_en);
                    }
                    HudHitTarget::LicenseTop => {
                        prompt_license_input(hwnd, lang_en);
                        InvalidateRect(hwnd, std::ptr::null(), 0);
                    }
                    HudHitTarget::None => {
                        // Make borderless window draggable!
                        windows_sys::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture();
                        windows_sys::Win32::UI::WindowsAndMessaging::SendMessageW(
                            hwnd,
                            windows_sys::Win32::UI::WindowsAndMessaging::WM_NCLBUTTONDOWN,
                            2, // HTCAPTION
                            0,
                        );
                    }
                }
                0
            }
            windows_sys::Win32::UI::WindowsAndMessaging::WM_LBUTTONUP => {
                windows_sys::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture();
                0
            }
            windows_sys::Win32::UI::WindowsAndMessaging::WM_ERASEBKGND => {
                1
            }
            WM_DESTROY => {
                0
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}





pub fn start_indicator() {
    restore_system_cursor(); // 시작 시 이전 꼬인 시스템 커서 상태 방지 복원
    crate::ui::win_uia::start_global_targets_thread();
    thread::spawn(|| unsafe {
        // Initialize GDI+
        let mut token: usize = 0;
        let input = GdiplusStartupInput {
            GdiplusVersion: 1,
            DebugEventCallback: 0,
            SuppressBackgroundThread: 0,
            SuppressExternalCodecs: 0,
        };
        if GdiplusStartup(&mut token, &input, std::ptr::null_mut()) == 0 {
            GDIPLUS_TOKEN.set(token).ok();
        } else {
            eprintln!("[Error] Failed to initialize GDI+");
        }

        let instance = windows_sys::Win32::System::LibraryLoader::GetModuleHandleW(std::ptr::null());
        
        // 1. Register Main Dummy class for Taskbar Icon
        let main_class_name = encode_wide("KeysorMainClass");
        let main_wnd_class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(main_wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: instance,
            hIcon: 0,
            hCursor: 0,
            hbrBackground: 0,
            lpszMenuName: std::ptr::null(),
            lpszClassName: main_class_name.as_ptr(),
        };

        if RegisterClassW(&main_wnd_class) == 0 {
            eprintln!("[Error] Failed to register Main Dummy window class.");
            return;
        }

        let arrow_cursor = windows_sys::Win32::UI::WindowsAndMessaging::LoadCursorW(0 as _, 32512 as *const u16);

        // 2. Register Indicator class
        let class_name_wide = encode_wide("KeysorIndicatorClass");
        let wnd_class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(indicator_wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: instance,
            hIcon: 0,
            hCursor: arrow_cursor,
            hbrBackground: 0,
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name_wide.as_ptr(),
        };

        if RegisterClassW(&wnd_class) == 0 {
            eprintln!("[Error] Failed to register indicator window class.");
            return;
        }

        // 3. Register HUD class
        let hud_class_name = encode_wide("KeysorHUDClass");
        let hud_wnd_class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(hud_wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: instance,
            hIcon: 0,
            hCursor: arrow_cursor,
            hbrBackground: 0,
            lpszMenuName: std::ptr::null(),
            lpszClassName: hud_class_name.as_ptr(),
        };

        if RegisterClassW(&hud_wnd_class) == 0 {
            eprintln!("[Error] Failed to register HUD window class.");
            return;
        }

        // 4. Create Main Dummy Window for Taskbar Icon
        let main_title = encode_wide("Keysor (Keyboard Mouse)");
        let main_hwnd = CreateWindowExW(
            WS_EX_APPWINDOW,
            main_class_name.as_ptr(),
            main_title.as_ptr(),
            WS_POPUP | WS_MINIMIZEBOX | WS_SYSMENU,
            -200,
            -200,
            0,
            0,
            0,
            0,
            instance,
            std::ptr::null(),
        );

        if main_hwnd != 0 {
            MAIN_HWND.set(main_hwnd).ok();

            // 시스템 메뉴를 얻어서 커스텀 항목 추가
            let sys_menu = GetSystemMenu(main_hwnd, 0);
            if sys_menu != 0 {
                AppendMenuW(sys_menu, MF_SEPARATOR, 0, std::ptr::null());
                AppendMenuW(sys_menu, MF_STRING, 1001, encode_wide("⚙️ Keysor 설정 열기 (keysor.yaml)").as_ptr());
                AppendMenuW(sys_menu, MF_STRING, 1002, encode_wide("📂 설정 폴더 열기 (.keysor)").as_ptr());
                AppendMenuW(sys_menu, MF_STRING, 1003, encode_wide("🚀 시작 프로그램 등록/해제 토글").as_ptr());
            }
            ShowWindow(main_hwnd, SW_SHOWNA);
        }

        // 5. Create Indicator Window
        let indicator_size = (32.0 * get_indicator_dpi_scale()) as i32;
        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_NOACTIVATE | WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
            class_name_wide.as_ptr(),
            std::ptr::null(),
            WS_POPUP,
            0,
            0,
            indicator_size, // width
            indicator_size, // height
            0,
            0,
            instance,
            std::ptr::null(),
        );

        if hwnd == 0 {
            eprintln!("[Error] Failed to create indicator window.");
            return;
        }

        INDICATOR_HWND.set(hwnd).ok();

        update_indicator_layered_image(hwnd);

        // UIAccess 상태 확인 로그 (UIAccess가 실제로 활성화되어 있는지 검증)
        let uia_active = process_has_uiaccess();
        println!("[UIAccess] Process UIAccess Active: {}", uia_active);
        if !uia_active {
            println!("[UIAccess] WARNING: UIAccess is NOT active. Start Menu overlay may be hidden behind DWM bands.");
        }

        // 6. Create HUD Window
        let screen_width = GetSystemMetrics(0); // SM_CXSCREEN
        let screen_height = GetSystemMetrics(1); // SM_CYSCREEN
        
        let scale = crate::platform::get_system_controller().get_dpi_scale();
        let hud_w = (808.0 * scale) as i32;
        let hud_h = (452.0 * scale) as i32;
        let hud_x = (screen_width - hud_w) / 2;
        let hud_y = (screen_height - hud_h) / 2;

        let hud_hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_NOACTIVATE | WS_EX_TOPMOST,
            hud_class_name.as_ptr(),
            encode_wide("Keysor HUD").as_ptr(),
            WS_POPUP,
            hud_x,
            hud_y,
            hud_w,
            hud_h,
            main_hwnd, // Owner window
            0,
            instance,
            std::ptr::null(),
        );

        if hud_hwnd == 0 {
            eprintln!("[Error] Failed to create HUD window.");
            return;
        }

        SetLayeredWindowAttributes(hud_hwnd, 0, 230, LWA_ALPHA); 
        HUD_HWND.set(hud_hwnd).ok();


        // 타이머 제거, 최초 팝업 상시 노출
        if should_show_hud(true) {
            ShowWindow(hud_hwnd, SW_SHOWNA);
        }
        InvalidateRect(hud_hwnd, std::ptr::null(), 0);
        UpdateWindow(hud_hwnd);

        println!("[Info] Indicator & HUD windows created successfully.");

        // Run message loop
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, 0, 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    });
}

fn hide_system_cursor_internal() {
    // Preserve the native pointer as the fallback.
    force_restore_system_cursor();
}

pub fn hide_system_cursor() {
    hide_system_cursor_internal();
}

pub fn force_hide_system_cursor() {
    hide_system_cursor_internal();
}

pub fn restore_system_cursor() {
    force_restore_system_cursor();
}

pub fn force_restore_system_cursor() {
    // Never run desktop-wide recovery synchronously from the keyboard hook.
    if let Some(&hwnd) = MAIN_HWND.get() {
        if !CURSOR_RESTORE_PENDING.swap(true, Ordering::SeqCst) {
            unsafe {
                if windows_sys::Win32::UI::WindowsAndMessaging::PostMessageW(hwnd, WM_KEYSOR_RESTORE_CURSOR, 0, 0) == 0 {
                    CURSOR_RESTORE_PENDING.store(false, Ordering::SeqCst);
                }
            }
        }
    }
}

fn custom_cursor_needed_now() -> bool {
    let mouse_mode = crate::hook::APP_STATE.get()
        .and_then(|state| state.lock().ok().map(|state| state.is_mouse_mode))
        .unwrap_or(false);
    if !mouse_mode { return false; }
    let foreground = unsafe { GetForegroundWindow() };
    let (process, class) = cached_foreground_window_info(foreground);
    // A shortcut alone is not a system popup. Decide from current state, not the
    // state when an asynchronous style/recovery message was queued.
    !unsafe { is_system_shell_foreground_with_info(foreground, &process, &class) }
}

fn restore_native_pointer_on_ui_thread() {
    let custom = custom_cursor_needed_now();
    crate::cursor_scheme::set_custom(custom);
    if custom { return; }
    static LAST_RESTORE: OnceLock<Mutex<Option<std::time::Instant>>> = OnceLock::new();
    let mut last = LAST_RESTORE.get_or_init(|| Mutex::new(None)).lock().unwrap();
    if last.is_some_and(|time| time.elapsed() < std::time::Duration::from_millis(250)) {
        return;
    }
    *last = Some(std::time::Instant::now());
    drop(last);
        unsafe {
            crate::cursor_scheme::restore();
            windows_sys::Win32::UI::Input::KeyboardAndMouse::mouse_event(
                windows_sys::Win32::UI::Input::KeyboardAndMouse::MOUSEEVENTF_MOVE,
                0,
                0,
                0,
                0,
            );
        }
}

/// Explicitly reopen the HUD without changing mouse mode or CapsLock state.
pub fn request_open_hud() {
    if let Some(&main) = MAIN_HWND.get() {
        unsafe { windows_sys::Win32::UI::WindowsAndMessaging::PostMessageW(main, WM_KEYSOR_OPEN_HUD, 0, 0); }
    }
}

pub fn show_indicator() {
    println!("[Debug] show_indicator() called");
    HUD_SUPPRESSED_BY_MOUSE_MODE.store(true, Ordering::SeqCst);
    if let Some(&main_hwnd) = MAIN_HWND.get() {
        unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::PostMessageW(main_hwnd, WM_KEYSOR_MINIMIZE_HUD, 0, 0);
        }
    }
    // The UI thread and movement loop apply the same foreground visibility policy.
}

pub fn hide_indicator() {
    println!("[Debug] hide_indicator() called");
    HUD_SUPPRESSED_BY_MOUSE_MODE.store(false, Ordering::SeqCst);
    force_restore_system_cursor();
    if let Some(&hwnd) = INDICATOR_HWND.get() {
        unsafe {
            ShowWindow(hwnd, SW_HIDE);
        }
    }
    crate::ui::win_uia::clear_magnetic_snapping();
}

unsafe fn get_process_name_from_window(hwnd: HWND) -> String {
    use windows_sys::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;
    use windows_sys::Win32::System::Threading::{OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION};
    use windows_sys::Win32::Foundation::CloseHandle;

    unsafe {
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == 0 {
            return "".to_string();
        }

        let process_handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if process_handle == 0 {
            return "".to_string();
        }

        let mut path_buf = [0u16; 1024];
        let mut size = path_buf.len() as u32;
        let success = QueryFullProcessImageNameW(process_handle, 0, path_buf.as_mut_ptr(), &mut size);
        CloseHandle(process_handle);

        if success != 0 && size > 0 {
            let full_path = String::from_utf16_lossy(&path_buf[..size as usize]);
            if let Some(pos) = full_path.rfind('\\') {
                full_path[pos + 1..].to_lowercase()
            } else {
                full_path.to_lowercase()
            }
        } else {
            "".to_string()
        }
    }
}



#[cfg(test)]
mod hud_visibility_tests {
    use super::hud_can_open;

    #[test]
    fn startup_and_taskbar_restore_open_hud_even_in_mouse_mode() {
        assert!(hud_can_open(false, true, true));
        assert!(hud_can_open(false, false, true));
    }

    #[test]
    fn automatic_activation_does_not_reopen_suppressed_hud() {
        assert!(!hud_can_open(false, true, false));
        assert!(hud_can_open(false, false, false));
    }

    #[test]
    fn license_input_keeps_hud_hidden() {
        assert!(!hud_can_open(true, true, true));
        assert!(!hud_can_open(true, false, true));
    }
}

#[cfg(test)]
mod shell_cursor_tests {
    use super::shell_surface_needs_native_cursor;

    #[test]
    fn shell_panels_use_native_cursor() {
        for process in ["StartMenuExperienceHost.exe", "SearchHost.exe", "ShellExperienceHost.exe", "ControlCenter.exe", "Taskmgr.exe"] {
            assert!(shell_surface_needs_native_cursor(process, ""));
        }
        assert!(shell_surface_needs_native_cursor("", "XamlExplorerHostIslandWindow"));
    }

    #[test]
    fn secure_and_switching_surfaces_keep_native_pointer() {
        assert!(shell_surface_needs_native_cursor("consent.exe", ""));
        for class in ["ForegroundStaging", "MultitaskingViewFrame"] {
            assert!(shell_surface_needs_native_cursor("explorer.exe", class));
        }
    }

    #[test]
    fn ordinary_windows_keep_overlay() {
        assert!(!shell_surface_needs_native_cursor("notepad.exe", "Notepad"));
        assert!(!shell_surface_needs_native_cursor("explorer.exe", "Shell_TrayWnd"));
    }

    #[test]
    fn cursor_policy_keeps_remote_overlay_and_system_fallback() {
        for native in [Some(true), Some(false), None] {
            assert_eq!(super::should_display_overlay(true, false, native, true), native == Some(false));
            for remote in [false, true] {
                assert!(!super::should_display_overlay(false, false, native, remote));
                assert!(!super::should_display_overlay(true, true, native, remote));
            }
        }
        assert!(!super::should_display_overlay(true, false, Some(true), false));
        assert!(!super::should_display_overlay(true, false, None, false));
        assert!(super::should_display_overlay(true, false, Some(false), false));
        assert!(!super::native_cursor_visible(3, true));
        assert!(!super::native_cursor_visible(1, false));
        assert!(super::native_cursor_visible(1, true));
    }

    #[test]
    fn native_cursor_recovery_uses_os_state() {
        assert!(super::native_pointer_needs_refresh(0, true));
        assert!(super::native_pointer_needs_refresh(2, true));
        assert!(super::native_pointer_needs_refresh(1, false));
        assert!(!super::native_pointer_needs_refresh(1, true));
    }
}

fn native_cursor_visible(flags: u32, has_cursor: bool) -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{CURSOR_SHOWING, CURSOR_SUPPRESSED};
    flags & CURSOR_SHOWING != 0 && flags & CURSOR_SUPPRESSED == 0 && has_cursor
}

fn native_pointer_needs_refresh(flags: u32, has_cursor: bool) -> bool {
    !native_cursor_visible(flags, has_cursor)
}

fn query_native_cursor_visible() -> Option<bool> {
    unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::{CURSORINFO, GetCursorInfo};
        let mut info: CURSORINFO = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<CURSORINFO>() as u32;
        (GetCursorInfo(&mut info) != 0).then(|| native_cursor_visible(info.flags, info.hCursor != 0))
    }
}

fn should_display_overlay(mouse_mode: bool, suspended: bool, native_visible: Option<bool>, remote_mode: bool) -> bool {
    let _ = remote_mode;
    // Native cursor carries Keysor artwork; never add an overlay over a visible pointer.
    mouse_mode && !suspended && native_visible == Some(false)
}

fn ensure_native_pointer_visible() {
    if query_native_cursor_visible() != Some(true) { force_restore_system_cursor(); }
}

fn process_has_uiaccess() -> bool {
    static ACTIVE: OnceLock<bool> = OnceLock::new();
    *ACTIVE.get_or_init(|| unsafe { is_process_uiaccess_active() })
}

fn shell_surface_needs_native_cursor(proc_name: &str, class_name: &str) -> bool {
    let p = proc_name.to_ascii_lowercase();
    let c = class_name.to_ascii_lowercase();
    matches!(p.as_str(), "consent.exe" | "startmenuexperiencehost.exe" | "searchhost.exe"
        | "shellexperiencehost.exe" | "controlcenter.exe" | "taskmgr.exe")
        || matches!(c.as_str(), "foregroundstaging" | "multitaskingviewframe"
            | "xamlexplorerhostislandwindow")
}

unsafe fn is_system_shell_foreground_with_info(hwnd: HWND, proc_name: &str, class_name: &str) -> bool {
    unsafe {
        if hwnd == 0 {
            // UAC 창이나 보안 시스템 포커스 팝업일 때는 기본 커서 복원
            return true;
        }
        
        if let Some(&main_hwnd) = MAIN_HWND.get() {
            if hwnd == main_hwnd { return false; }
        }
        if let Some(&hud_hwnd) = HUD_HWND.get() {
            if hwnd == hud_hwnd { return false; }
        }
        if let Some(&indicator_hwnd) = INDICATOR_HWND.get() {
            if hwnd == indicator_hwnd { return false; }
        }

        shell_surface_needs_native_cursor(proc_name, class_name)
    }
}

fn cached_foreground_window_info(hwnd: HWND) -> (String, String) {
    struct Cache { hwnd: HWND, at: std::time::Instant, details: (String, String) }
    static CACHE: OnceLock<Mutex<Option<Cache>>> = OnceLock::new();
    let mut cache = CACHE.get_or_init(|| Mutex::new(None)).lock().unwrap();
    let now = std::time::Instant::now();
    if let Some(cached) = cache.as_ref() {
        if cached.hwnd == hwnd && now.duration_since(cached.at) < std::time::Duration::from_secs(1) {
            return cached.details.clone();
        }
    }
    #[cfg(test)]
    DETAIL_QUERY_COUNT.fetch_add(1, Ordering::Relaxed);
    let details = get_foreground_window_info(hwnd);
    *cache = Some(Cache { hwnd, at: now, details: details.clone() });
    details
}

fn get_foreground_window_info(fore_hwnd: HWND) -> (String, String) {
    let mut proc_name = String::new();
    let mut class_name = String::new();
    if fore_hwnd != 0 {
        unsafe {
            proc_name = get_process_name_from_window(fore_hwnd);
            let mut class_name_buf = [0u16; 256];
            let len = GetClassNameW(fore_hwnd, class_name_buf.as_mut_ptr(), 256);
            if len > 0 {
                class_name = String::from_utf16_lossy(&class_name_buf[..len as usize]);
            }
        }
    }
    (proc_name, class_name)
}

pub fn shell_cursor_diagnostic(fore_hwnd: HWND) -> (String, String, bool) {
    let (process, class) = get_foreground_window_info(fore_hwnd);
    let native_fallback = fore_hwnd == 0
        || shell_surface_needs_native_cursor(&process, &class);
    (process, class, native_fallback)
}

fn write_debug_log(
    fore_hwnd: HWND,
    proc_name: &str,
    class_name: &str,
    is_suspended: bool,
    is_shortcut_active: bool,
    is_shell_active: bool,
    is_hide_suspended: bool,
) {
    #[cfg(debug_assertions)]
    unsafe {
        static mut LOG_TICK: u32 = 0;
        LOG_TICK = (LOG_TICK + 1) % 10;
        if LOG_TICK == 0 {
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .append(true)
                .open("C:\\Users\\wjdwl\\.gemini\\antigravity\\scratch\\14-Keysor\\debug_cursor.txt")
            {
                use std::io::Write;
                let _ = writeln!(
                    file,
                    "[Debug] ForeHwnd: 0x{:X}, Proc: {}, Class: {}, is_suspended: {}, is_shortcut: {}, is_shell: {}, is_hide_sus: {}", 
                    fore_hwnd as usize, proc_name, class_name, is_suspended, is_shortcut_active, is_shell_active, is_hide_suspended
                );
            }
        }
    }
}

fn apply_overlay_visibility(hwnd: HWND, show: bool, is_visible: &mut bool) -> bool {
    unsafe {
        if show {
            if !*is_visible {
                update_indicator_layered_image(hwnd);
                ShowWindow(hwnd, SW_SHOWNA);
                *is_visible = windows_sys::Win32::UI::WindowsAndMessaging::IsWindowVisible(hwnd) != 0;
            }
            // The positioning path also uses SWP_SHOWWINDOW if an initial show was ignored.
            true
        } else {
            if *is_visible {
                ShowWindow(hwnd, SW_HIDE);
                *is_visible = false;
            }
            false
        }
    }
}

#[derive(Default)]
struct CursorStyleSchedule {
    last: Option<((bool, HWND, (bool, bool)), std::time::Instant)>,
}
impl CursorStyleSchedule {
    fn due(&self, key: (bool, HWND, (bool, bool)), now: std::time::Instant) -> bool {
        self.last.as_ref().is_none_or(|(old, at)| *old != key
            || now.saturating_duration_since(*at) >= std::time::Duration::from_secs(1))
    }
    fn sent(&mut self, key: (bool, HWND, (bool, bool)), now: std::time::Instant) { self.last = Some((key, now)); }
}

fn request_cursor_style_update(mouse_mode: bool) {
    static SCHEDULE: OnceLock<Mutex<CursorStyleSchedule>> = OnceLock::new();
    let key = (mouse_mode, unsafe { GetForegroundWindow() }, crate::cursor_scheme::status());
    let now = std::time::Instant::now();
    let mut schedule = SCHEDULE.get_or_init(|| Mutex::new(CursorStyleSchedule::default())).lock().unwrap();
    if !schedule.due(key, now) { return; }
    if let Some(&main) = MAIN_HWND.get() {
        if !CURSOR_STYLE_PENDING.swap(true, Ordering::SeqCst) {
            let posted = unsafe { windows_sys::Win32::UI::WindowsAndMessaging::PostMessageW(main, WM_KEYSOR_CURSOR_STYLE, 0, 0) != 0 };
            if posted { schedule.sent(key, now); }
            else { CURSOR_STYLE_PENDING.store(false, Ordering::SeqCst); }
        }
    }
}

fn handle_cursor_visibility(hwnd: HWND, current_mouse_mode: bool, is_suspended: bool, is_visible: &mut bool) -> bool {
    static LAST_FALLBACK_WINDOW: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);
    request_cursor_style_update(current_mouse_mode);
    let native_visible = query_native_cursor_visible();
    let show_overlay = should_display_overlay(current_mouse_mode, is_suspended, native_visible, crate::platform::windows::chrome_remote_mode());
    // Hide the overlay before requesting any native recovery, never blank the OS cursor.
    let show_overlay = apply_overlay_visibility(hwnd, show_overlay, is_visible);
    if !show_overlay {
        let fallback_window = if is_suspended { unsafe { GetForegroundWindow() } } else { 0 };
        let entering_fallback = LAST_FALLBACK_WINDOW.swap(fallback_window, Ordering::SeqCst) != fallback_window && is_suspended;
        if entering_fallback { force_restore_system_cursor(); }
        else if !current_mouse_mode || is_suspended || native_visible.is_none() { ensure_native_pointer_visible(); }
    } else {
        LAST_FALLBACK_WINDOW.store(0, Ordering::SeqCst);
    }
    show_overlay
}

#[cfg(test)]
pub(crate) fn assert_cursor_style_update_budget() {
    let updates = STYLE_UPDATE_COUNT.load(Ordering::Relaxed);
    let queries = DETAIL_QUERY_COUNT.load(Ordering::Relaxed);
    for _ in 0..100 {
        request_cursor_style_update(true);
        cached_foreground_window_info(unsafe { GetForegroundWindow() });
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    std::thread::sleep(std::time::Duration::from_millis(20));
    assert!(STYLE_UPDATE_COUNT.load(Ordering::Relaxed) - updates <= 3,
        "100 idle requests must not produce 100 UI style messages");
    assert!(DETAIL_QUERY_COUNT.load(Ordering::Relaxed) - queries <= 2,
        "The movement and UI paths must share cached foreground details");
}

#[cfg(test)]
pub(crate) fn assert_no_normal_mode_cursor_restore() {
    assert!(custom_cursor_needed_now());
    crate::cursor_scheme::set_custom(true);
    assert!(crate::cursor_scheme::status().1);
    let before = crate::cursor_scheme::restore_count();
    for _ in 0..20 {
        force_restore_system_cursor();
        std::thread::sleep(std::time::Duration::from_millis(20));
        assert!(crate::cursor_scheme::status().1);
    }
    assert_eq!(crate::cursor_scheme::restore_count(), before,
        "Delayed recovery must not restore the default cursor on ordinary surfaces");
}

#[cfg(test)]
pub(crate) fn assert_single_cursor_windows() {
    let hwnd = *INDICATOR_HWND.get().expect("Indicator window");
    let previous_remote = crate::platform::windows::chrome_remote_mode();
    for remote in [false, true] {
      crate::platform::windows::set_chrome_remote_mode(remote);
      for (mouse_mode, suspended) in [(true, false), (true, true), (true, false), (false, false)] {
        for _ in 0..5 {
            let mut visible = unsafe { windows_sys::Win32::UI::WindowsAndMessaging::IsWindowVisible(hwnd) != 0 };
            if handle_cursor_visibility(hwnd, mouse_mode, suspended, &mut visible) {
                update_overlay_window_position(hwnd, false);
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let visible = unsafe { windows_sys::Win32::UI::WindowsAndMessaging::IsWindowVisible(hwnd) != 0 };
        if !should_display_overlay(mouse_mode, suspended, query_native_cursor_visible(), remote) {
            assert!(!visible, "Overlay must be hidden when the native cursor is available or required");
        } else {
            assert!(visible, "Overlay must remain available in CRD or when Windows suppresses its cursor");
        }
      }
    }
    crate::platform::windows::set_chrome_remote_mode(previous_remote);
}

fn update_click_scale() -> bool {
    let (is_holding_action, hold_type) = if let Some(state_arc) = crate::hook::APP_STATE.get() {
        if let Ok(state) = state_arc.try_lock() {
            let space_held = state.is_space_down || state.is_dragging;
            let scroll_held = !state.active_scroll_keys.is_empty();
            let right_held = state.is_right_down;
            if space_held {
                (true, ClickType::Left)
            } else if scroll_held {
                (true, ClickType::Scroll)
            } else if right_held {
                (true, ClickType::Right)
            } else {
                (false, ClickType::None)
            }
        } else {
            (false, ClickType::None)
        }
    } else {
        (false, ClickType::None)
    };

    let scale_lock = CLICK_SCALE.get_or_init(|| Mutex::new(1.0));
    let mut scale_changed = false;
    if let Ok(mut scale) = scale_lock.lock() {
        if is_holding_action {
            // 스페이스바(드래그) 또는 스크롤 키를 누르고 있는 동안에는 작은 상태(0.7)로 고정 유지
            if *scale != 0.7 {
                *scale = 0.7;
                scale_changed = true;
            }
            let type_lock = CLICK_TYPE.get_or_init(|| Mutex::new(ClickType::None));
            if let Ok(mut t) = type_lock.lock() {
                if *t != hold_type {
                    *t = hold_type;
                    scale_changed = true;
                }
            }
        } else {
            // 손을 떼었을 때 기본 크기(1.0)로 부드럽게 복귀
            if *scale < 1.0 {
                *scale += 0.08;
                if *scale > 1.0 {
                    *scale = 1.0;
                    let type_lock = CLICK_TYPE.get_or_init(|| Mutex::new(ClickType::None));
                    if let Ok(mut t) = type_lock.lock() {
                        *t = ClickType::None;
                    }
                }
                scale_changed = true;
            }
        }
    }
    scale_changed
}

fn calculate_interpolated_pos() -> (f64, f64) {
    let mut pt = POINT { x: 0, y: 0 };
    unsafe {
        windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut pt);
    }
    
    let dpi_scale = get_indicator_dpi_scale();
    let target_x = pt.x as f64 - 16.0 * dpi_scale;
    let target_y = pt.y as f64 - 16.0 * dpi_scale;
    
    let pos_lock = CUR_INDICATOR_POS.get_or_init(|| Mutex::new(None));
    let mut pos = pos_lock.lock().unwrap();
    
    let (next_x, next_y) = match *pos {
        Some((cx, cy)) => {
            let lerp_factor = 0.35;
            let nx = cx + (target_x - cx) * lerp_factor;
            let ny = cy + (target_y - cy) * lerp_factor;
            (nx, ny)
        }
        None => {
            (target_x, target_y)
        }
    };
    
    *pos = Some((next_x, next_y));
    (next_x, next_y)
}

fn check_state_changed(is_space_down: bool, is_right_down: bool, is_dragging: bool, is_scrolling: bool, is_snapped: bool) -> bool {
    static LAST_INDICATOR_STATE: OnceLock<Mutex<(bool, bool, bool, bool, bool)>> = OnceLock::new();
    let last_state_lock = LAST_INDICATOR_STATE.get_or_init(|| Mutex::new((false, false, false, false, false)));
    if let Ok(mut last_state) = last_state_lock.lock() {
        if *last_state != (is_space_down, is_right_down, is_dragging, is_scrolling, is_snapped) {
            *last_state = (is_space_down, is_right_down, is_dragging, is_scrolling, is_snapped);
            true
        } else {
            false
        }
    } else {
        false
    }
}

/// 실제 물리 키 상태를 스캔하여 락(Lock)이 걸린 모디파이어 키 상태 동기화
fn check_and_sync_physical_modifiers(is_shortcut_active: &mut bool) {
    if !*is_shortcut_active { return; }
    let Some(state_arc) = crate::hook::APP_STATE.get() else { return; };
    let Ok(mut state) = state_arc.try_lock() else { return; };
    // Read and repair under one lock so a new modifier keydown cannot be erased
    // between the physical-state observation and the recovery operation.
    unsafe {
        let down = |vk| (windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(vk) as u16 & 0x8000) != 0;
        if !down(0x11) && !down(0x12) && !down(0x5B) && !down(0x5C) {
            state.clear_released_shortcut_modifiers();
            *is_shortcut_active = false;
        }
    }
}

/// 포그라운드 윈도우 정보를 기준으로 서스펜드(일시정지) 판단
fn resolve_foreground_suspend_state(fore_hwnd: HWND, is_shortcut_active: bool) -> (String, String, bool) {
    let (proc_name, class_name) = cached_foreground_window_info(fore_hwnd);

    let is_shell_active = unsafe { is_system_shell_foreground_with_info(fore_hwnd, &proc_name, &class_name) };
    let is_hide_suspended = SUSPEND_CURSOR_HIDE.load(Ordering::SeqCst);
    let is_suspended = is_shortcut_active || is_shell_active || is_hide_suspended;

    unsafe {
        write_debug_log(fore_hwnd, &proc_name, &class_name, is_suspended, is_shortcut_active, is_shell_active, is_hide_suspended);
    }

    (proc_name, class_name, is_suspended)
}

/// 인디케이터 오버레이 창의 위치 및 렌더링 상태 동기화
fn update_overlay_window_position(hwnd: HWND, scale_changed_in_loop: bool) {
    let (next_x, next_y) = calculate_interpolated_pos();

    let (is_space_down, is_right_down, is_dragging, is_scrolling, is_snapped) = if let Some(state_arc) = crate::hook::APP_STATE.get() {
        state_arc.try_lock().map(|s| {
            (s.is_space_down, s.is_right_down, s.is_dragging, !s.active_scroll_keys.is_empty(), is_currently_snapped())
        }).unwrap_or((false, false, false, false, false))
    } else {
        (false, false, false, false, false)
    };

    static LAST_DPI_SCALE: OnceLock<Mutex<f64>> = OnceLock::new();
    let current_dpi_scale = get_indicator_dpi_scale();
    let dpi_changed = {
        let lock = LAST_DPI_SCALE.get_or_init(|| Mutex::new(0.0));
        if let Ok(mut last) = lock.lock() {
            if (*last - current_dpi_scale).abs() > 0.01 {
                *last = current_dpi_scale;
                true
            } else {
                false
            }
        } else {
            false
        }
    };

    let state_changed = check_state_changed(is_space_down, is_right_down, is_dragging, is_scrolling, is_snapped);
    if state_changed || scale_changed_in_loop || dpi_changed {
        unsafe { update_indicator_layered_image(hwnd) };
    }

    // Native cursors use the same animation frames even while the overlay is hidden.
    if query_native_cursor_visible() != Some(false) {
        let mut visible = unsafe { windows_sys::Win32::UI::WindowsAndMessaging::IsWindowVisible(hwnd) != 0 };
        apply_overlay_visibility(hwnd, false, &mut visible);
        return;
    }

    let current_size = (32.0 * current_dpi_scale) as i32;
    let next_position = (next_x.round() as i32, next_y.round() as i32, current_size);
    static LAST_OVERLAY_PLACEMENT: OnceLock<Mutex<Option<((i32, i32, i32), HWND, std::time::Instant)>>> = OnceLock::new();
    let placement = LAST_OVERLAY_PLACEMENT.get_or_init(|| Mutex::new(None));
    let foreground = unsafe { GetForegroundWindow() };
    let should_position = if let Ok(mut last) = placement.lock() {
        let now = std::time::Instant::now();
        let changed = last.as_ref().is_none_or(|(position, window, raised_at)| {
            *position != next_position || *window != foreground || raised_at.elapsed() >= std::time::Duration::from_millis(250)
        });
        if changed {
            *last = Some((next_position, foreground, now));
        }
        changed
    } else {
        true
    };

    unsafe {
        if should_position || windows_sys::Win32::UI::WindowsAndMessaging::IsWindowVisible(hwnd) == 0 {
            windows_sys::Win32::UI::WindowsAndMessaging::SetWindowPos(
                hwnd,
                windows_sys::Win32::UI::WindowsAndMessaging::HWND_TOPMOST,
                next_position.0,
                next_position.1,
                current_size,
                current_size,
                windows_sys::Win32::UI::WindowsAndMessaging::SWP_NOACTIVATE
                    | windows_sys::Win32::UI::WindowsAndMessaging::SWP_SHOWWINDOW,
            );
        }
    }
}

pub fn update_indicator_position() {
    if let Some(&hwnd) = INDICATOR_HWND.get() {
        unsafe {
            // 1. 마우스 모드 안전성 검사 (비활성화 상태에서 타임 꼬임 방지)
            let Some(is_mouse_mode) = crate::hook::APP_STATE.get()
                .and_then(|state| state.try_lock().ok().map(|state| state.is_mouse_mode))
            else { return; };

            if !is_mouse_mode {
                            force_restore_system_cursor();
                if windows_sys::Win32::UI::WindowsAndMessaging::IsWindowVisible(hwnd) != 0 {
                    windows_sys::Win32::UI::WindowsAndMessaging::ShowWindow(hwnd, windows_sys::Win32::UI::WindowsAndMessaging::SW_HIDE);
                }
                return;
            }

            // 2. 최소화 상태 복구
            let is_minimized = windows_sys::Win32::UI::WindowsAndMessaging::IsIconic(hwnd) != 0;
            if is_minimized {
                windows_sys::Win32::UI::WindowsAndMessaging::ShowWindow(hwnd, windows_sys::Win32::UI::WindowsAndMessaging::SW_RESTORE);
            }

            // 3. 물리 단축키 상태 불일치 점검 및 동기화
            let mut is_shortcut_active = crate::hook::APP_STATE.get()
                .and_then(|arc| arc.try_lock().ok())
                .map(|s| s.is_system_shortcut_active())
                .unwrap_or(false);
            check_and_sync_physical_modifiers(&mut is_shortcut_active);

            if !is_shortcut_active {
                SUSPEND_CURSOR_HIDE.store(false, Ordering::SeqCst);
            }

            // 4. 포그라운드 윈도우 스캔 및 서스펜드 상태 분석
            let fore_hwnd = GetForegroundWindow();
            let (_, _, is_suspended) = resolve_foreground_suspend_state(fore_hwnd, is_shortcut_active);

            // 5. 창 가시성 및 시스템 커서 토글 처리
            let mut is_visible = windows_sys::Win32::UI::WindowsAndMessaging::IsWindowVisible(hwnd) != 0;
            handle_cursor_visibility(hwnd, is_mouse_mode, is_suspended, &mut is_visible);
            if is_suspended {
                return;
            }

            // 6. 클릭 스케일 애니메이션 처리
            let scale_changed_in_loop = update_click_scale();

            // 7. 인디케이터 좌표/위치 동기화 및 렌더링 갱신
            update_overlay_window_position(hwnd, scale_changed_in_loop);
        }
    }
}














#[allow(dead_code)]
pub unsafe fn is_process_uiaccess_active() -> bool {
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
    use windows_sys::Win32::Security::{GetTokenInformation, TOKEN_QUERY, TokenUIAccess};
    use windows_sys::Win32::Foundation::{HANDLE, CloseHandle};

    let mut token: HANDLE = 0;
    if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) != 0 {
        let mut uia_state: u32 = 0;
        let mut return_len: u32 = 0;
        let success = GetTokenInformation(
            token,
            TokenUIAccess, // TokenUIAccess
            &mut uia_state as *mut _ as *mut _,
            std::mem::size_of::<u32>() as u32,
            &mut return_len,
        );
        CloseHandle(token);
        if success != 0 {
            return uia_state != 0;
        }
    }
    false
}

#[allow(dead_code)]
pub fn is_snap_flyout_active() -> bool {
    unsafe {
        let mut pt = windows_sys::Win32::Foundation::POINT { x: 0, y: 0 };
        if windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut pt) != 0 {
            let hwnd_at_pt = windows_sys::Win32::UI::WindowsAndMessaging::WindowFromPoint(pt);
            if hwnd_at_pt != 0 {
                let mut class_buf = [0u16; 256];
                let len = windows_sys::Win32::UI::WindowsAndMessaging::GetClassNameW(
                    hwnd_at_pt,
                    class_buf.as_mut_ptr(),
                    class_buf.len() as i32,
                );
                if len > 0 {
                    let class_str = String::from_utf16_lossy(&class_buf[..len as usize]);
                    if class_str.contains("XamlExplorerHostIsland")
                        || class_str.contains("DesktopWindowXamlSource")
                        || class_str.contains("SnapFlyout")
                    {
                        return true;
                    }
                }

                // WM_NCHITTEST: 최대화 버튼(HTMAXBUTTON = 9 또는 21) 위에 커서가 위치했는지 체크
                let lparam = ((pt.y as u32) << 16) | (pt.x as u32 & 0xFFFF);
                let hit = windows_sys::Win32::UI::WindowsAndMessaging::SendMessageW(
                    hwnd_at_pt,
                    windows_sys::Win32::UI::WindowsAndMessaging::WM_NCHITTEST,
                    0,
                    lparam as isize,
                );
                if hit == 9 || hit == 21 {
                    return true;
                }
            }
        }

        let class_name = windows_sys::w!("XamlExplorerHostIslandWindow");
        let hwnd = windows_sys::Win32::UI::WindowsAndMessaging::FindWindowExW(
            0, 0, class_name, std::ptr::null(),
        );
        if hwnd != 0 && windows_sys::Win32::UI::WindowsAndMessaging::IsWindowVisible(hwnd) != 0 {
            return true;
        }
        false
    }
}

#[cfg(test)]
mod support_entry_tests {
    use super::*;
    #[test]
    fn support_is_accessible_for_free_trial_and_pro() {
        for (features, pro) in [(false, false), (true, false), (true, true)] {
            assert_eq!(classify_hit_target(700, 414, features, pro), HudHitTarget::Support);
            assert_ne!(classify_hit_target(700, 390, features, pro), HudHitTarget::Support);
        }
    }
}

#[cfg(test)]
mod cursor_schedule_tests {
    use super::*;
    #[test]
    fn idle_hundred_hz_updates_are_bounded_but_transitions_are_immediate() {
        let now = std::time::Instant::now();
        let key = (true, 100, (true, true));
        let mut schedule = CursorStyleSchedule::default();
        let mut sends = 0;
        for tick in 0..1000 {
            let time = now + std::time::Duration::from_millis(tick * 10);
            if schedule.due(key, time) { sends += 1; schedule.sent(key, time); }
        }
        assert_eq!(sends, 10, "Idle requests should be once per second rather than every frame");
        let time = now + std::time::Duration::from_millis(9991);
        for changed in [(false, 100, (true, true)), (true, 200, (true, true)), (true, 100, (true, false))] {
            assert!(schedule.due(changed, time));
        }
    }
}

#[cfg(test)]
mod hud_render_tests {
    use super::*;
    #[test]
    #[ignore = "Renders a standalone HUD for pixel regression comparison; no hooks, clipboard or settings changes"]
    fn native_hud_rendering_preview() {
        unsafe {
            let instance = windows_sys::Win32::System::LibraryLoader::GetModuleHandleW(std::ptr::null());
            let name = encode_wide("KeysorHudRenderTest");
            let class = WNDCLASSW { lpfnWndProc: Some(hud_wnd_proc), hInstance: instance,
                lpszClassName: name.as_ptr(), ..std::mem::zeroed() };
            RegisterClassW(&class);
            let hwnd = CreateWindowExW(WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE, name.as_ptr(), name.as_ptr(),
                WS_POPUP, 20, 20, 808, 452, 0, 0, instance, std::ptr::null());
            assert_ne!(hwnd, 0);
            ShowWindow(hwnd, SW_SHOWNA); UpdateWindow(hwnd);
            crate::ui::test_capture::save_window_bitmap(hwnd, "target/hud-render-preview.bmp");
            windows_sys::Win32::UI::WindowsAndMessaging::DestroyWindow(hwnd);
        }
    }
}
