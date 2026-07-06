use std::sync::{OnceLock, Mutex};
use std::thread;
use std::sync::atomic::{AtomicU32, Ordering};
use uiautomation::core::UIAutomation;
use uiautomation::types::Handle;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM, POINT, RECT, SIZE};
use windows_sys::Win32::Graphics::Gdi::{
    BeginPaint, EndPaint, CreateSolidBrush, DeleteObject, SelectObject, CreatePen,
    InvalidateRect, UpdateWindow, RoundRect, SetTextColor, SetBkMode, DrawTextW,
    CreateFontW, GetDC, ReleaseDC, CreateCompatibleDC, CreateDIBSection, DeleteDC,
    BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, AC_SRC_OVER, AC_SRC_ALPHA
};
use windows_sys::Win32::Graphics::GdiPlus::{
    GdiplusStartup, GdiplusStartupInput,
    GdipCreateFromHDC, GdipDeleteGraphics, GdipSetSmoothingMode,
    GdipCreatePen1, GdipDeletePen, GdipDrawLineI, SmoothingModeAntiAlias,
    GdipSetPenStartCap, GdipSetPenEndCap, GdipSetPenLineJoin,
    GdipCreateLineBrush, GdipCreatePen2, GdipDeleteBrush, PointF
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    RegisterClassW, CreateWindowExW, DefWindowProcW, ShowWindow, SetWindowPos, MSG,
    WNDCLASSW, CS_HREDRAW, CS_VREDRAW, WS_POPUP, WS_EX_LAYERED, WS_EX_TRANSPARENT,
    WS_EX_TOPMOST, WS_EX_NOACTIVATE, SW_HIDE, SW_SHOWNA, LWA_ALPHA, WM_PAINT,
    WM_DESTROY, GetMessageW, TranslateMessage, DispatchMessageW, HWND_TOPMOST,
    SWP_NOSIZE, SWP_NOACTIVATE, SetLayeredWindowAttributes, GetClientRect, GetSystemMetrics,
    WS_EX_APPWINDOW, WM_CLOSE, GetSystemMenu, AppendMenuW,
    MF_SEPARATOR, MF_STRING, WM_SYSCOMMAND, SW_MINIMIZE, WS_MINIMIZEBOX, WS_SYSMENU,
    WM_ERASEBKGND, UpdateLayeredWindow, ULW_ALPHA, GetClassNameW, GetWindowRect,
    WS_EX_TOOLWINDOW, GetForegroundWindow
};

pub static INDICATOR_HWND: OnceLock<HWND> = OnceLock::new();
pub static HUD_HWND: OnceLock<HWND> = OnceLock::new();
pub static MAIN_HWND: OnceLock<HWND> = OnceLock::new();
static GDIPLUS_TOKEN: OnceLock<usize> = OnceLock::new();
pub static HUD_HOVER: AtomicU32 = AtomicU32::new(0); // 0: none, 1: minimize, 2: close
pub static HUD_LAST_SNAPPED: AtomicU32 = AtomicU32::new(0);
pub static SHOW_ALL_SENS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
pub static CLICK_SCALE: OnceLock<Mutex<f32>> = OnceLock::new();
pub static IS_INPUTTING_LICENSE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
pub static SUSPEND_CURSOR_HIDE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

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
        check_magnetic_snapping();
    }



    fn check_global_magnetic_snapping(&self) {
        check_global_magnetic_snapping();
    }

    fn is_currently_snapped(&self) -> bool {
        is_currently_snapped()
    }
}

pub static CLICK_TYPE: OnceLock<Mutex<ClickType>> = OnceLock::new();

const WM_MOUSELEAVE: u32 = 0x02A3;

fn encode_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

static LICENSE_INPUT_RESULT: OnceLock<Mutex<Option<String>>> = OnceLock::new();
static INPUT_DIALOG_ACTIVE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static IS_LANG_EN_GLOBAL: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

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
            CreateWindowExW(
                0,
                encode_wide("STATIC").as_ptr(),
                encode_wide(static_text).as_ptr(),
                WS_CHILD | WS_VISIBLE,
                20, 20, 380, 25,
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
                20, 50, 380, 25,
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
                200, 90, 90, 30,
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
                310, 90, 90, 30,
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
            clear_magnetic_snapping();
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
        let screen_w = GetSystemMetrics(0);
        let screen_h = GetSystemMetrics(1);
        let dlg_w = 440;
        let dlg_h = 180;
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

        // Re-enable HUD and restore visibility
        if let Some(&hud) = HUD_HWND.get() {
            windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow(hud, 1);
            ShowWindow(hud, SW_SHOWNA);
            // Set focus back to HUD
            windows_sys::Win32::UI::Input::KeyboardAndMouse::SetFocus(hud);
        }
        IS_INPUTTING_LICENSE.store(false, Ordering::SeqCst);

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
                            if let Some(state_arc) = crate::hook::APP_STATE.get() {
                                if let Ok(mut state) = state_arc.lock() {
                                    state.is_pro = true;
                                }
                            }
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

fn draw_key_cap(
    hdc: windows_sys::Win32::Graphics::Gdi::HDC,
    font: windows_sys::Win32::Graphics::Gdi::HFONT,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    key_text: &str,
    desc_text: &str,
    category: u32, // 0: none, 1: Caps Lock, 2: WASD, 3: QERF, 4: Spacebar
) {
    unsafe {
        let bg_color = 0x202424; 
        let border_color = match category {
            1 => 0x3300FF, // Caps Lock: 네온 레드 (BGR: B=00, G=00, R=FF와 보정)
            2 => 0x40E000, // WASD: Grass Green
            3 => 0xFFFF00, // QERF: 네온 파랑 (Cyan)
            4 => 0x0045FF, // Spacebar: 네온 오렌지 (커서 드래그 색상과 동기화)
            5 => 0x00FFFF, // G Key: 네온 노랑 (Yellow)
            6 => 0xFFFFFF, // Shift: 흰색
            _ => 0x3C4040, // Non-highlighted (Grey)
        };
        
        let brush = CreateSolidBrush(bg_color);
        let pen = CreatePen(0, 1, border_color);

        let old_brush = SelectObject(hdc, brush);
        let old_pen = SelectObject(hdc, pen);
        let old_font = SelectObject(hdc, font);

        RoundRect(hdc, x, y, x + w, y + h, 8, 8);

        SetTextColor(hdc, if category > 0 { border_color } else { 0x888888 });
        let key_w = encode_wide(key_text);
        let mut r_key = RECT { left: x, top: y + 4, right: x + w, bottom: y + 21 };
        DrawTextW(hdc, key_w.as_ptr(), key_w.len() as i32 - 1, &mut r_key, 1 | 32);

        // 영문 텍스트가 좁은 키캡(w <= 45)에서 짤리는 것을 방지하기 위해 동적으로 작은 폰트 적용
        let desc_font = if w <= 45 && desc_text.len() > 4 {
            let font_name = encode_wide("Segoe UI");
            CreateFontW(11, 0, 0, 0, 500, 0, 0, 0, 1, 0, 0, 5, 0, font_name.as_ptr())
        } else {
            0
        };

        let old_desc_font = if desc_font != 0 {
            SelectObject(hdc, desc_font)
        } else {
            0
        };

        SetTextColor(hdc, if category > 0 { 0xFFFFFF } else { 0x555555 });
        let desc_w = encode_wide(desc_text);
        let top_offset = if category == 6 { 18 } else { 23 }; // Shift(6)는 2줄 설명이므로 5px 위로 올림
        let bottom_offset = if category == 6 { 2 } else { 4 };
        let mut r_desc = RECT { left: x + 2, top: y + top_offset, right: x + w - 2, bottom: y + h - bottom_offset };
        let align = if desc_text.contains('\n') { 1 } else { 1 | 32 }; // DT_CENTER vs DT_CENTER | DT_SINGLELINE
        DrawTextW(hdc, desc_w.as_ptr(), desc_w.len() as i32 - 1, &mut r_desc, align);

        if desc_font != 0 {
            SelectObject(hdc, old_desc_font);
            DeleteObject(desc_font);
        }

        SelectObject(hdc, old_font);
        SelectObject(hdc, old_brush);
        SelectObject(hdc, old_pen);
        
        DeleteObject(brush);
        DeleteObject(pen);
    }
}

unsafe extern "system" fn main_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        match msg {
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
                        clear_magnetic_snapping();
                    } else if state == 0 { // SIZE_RESTORED
                        if !IS_INPUTTING_LICENSE.load(Ordering::SeqCst) {
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
                        if !IS_INPUTTING_LICENSE.load(Ordering::SeqCst) {
                            ShowWindow(hud, SW_SHOWNA);
                        }
                    }
                }
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
            WM_CLOSE | WM_DESTROY => {
                crate::hook::cleanup_hook();
                std::process::exit(0);
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
        *scale = 0.5; // 작아짐
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

fn update_indicator_layered_image(hwnd: HWND) {
    unsafe {
        let width = 32;
        let height = 32;

        let scale_val = if let Some(scale_lock) = CLICK_SCALE.get() {
            if let Ok(scale) = scale_lock.lock() {
                *scale
            } else {
                1.0
            }
        } else {
            1.0
        };

        let scale_coord = |coord: i32, center: i32| -> i32 {
            center + ((coord - center) as f32 * scale_val).round() as i32
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

            // Draw cursor using GDI+
            let mut graphics = std::ptr::null_mut();
            if GdipCreateFromHDC(hdc_mem, &mut graphics) == 0 {
                GdipSetSmoothingMode(graphics, SmoothingModeAntiAlias);

                // 1. Draw high-contrast black border first (4.0px width)
                let mut black_pen = std::ptr::null_mut();
                if GdipCreatePen1(0xFF000000, 4.0, 2, &mut black_pen) == 0 {
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
                if GdipCreateLineBrush(&p1, &p2, start_color, end_color, 0, &mut brush) == 0 {
                    let mut gradient_pen = std::ptr::null_mut();
                    if GdipCreatePen2(brush, 2.5, 2, &mut gradient_pen) == 0 {
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
    if let Some(state_arc) = crate::hook::APP_STATE.get() {
        if let Ok(mut state) = state_arc.lock() {
            let cur = state.config.settings.magnetic_mode.unwrap_or(false);
            let next_mode = !cur;
            state.config.settings.magnetic_mode = Some(next_mode);
            state.config.settings.global_magnetic_mode = Some(next_mode); // Sync global magnet mode too!
            if !next_mode {
                HUD_LAST_SNAPPED.store(0, std::sync::atomic::Ordering::SeqCst);
                if let Some(lock) = LAST_GLOBAL_SNAPPED_POS.get() {
                    if let Ok(mut pos) = lock.lock() {
                        *pos = None;
                    }
                }
            }
            config_to_save = Some(state.config.clone());
        }
    }
    if let Some(cfg) = config_to_save {
        crate::config::save_config(&cfg);
    }
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
    BuyProTop = 10,
    LicenseTop = 11,
}

fn classify_hit_target(x: i16, y: i16, features_enabled: bool, is_pro: bool) -> HudHitTarget {
    if !features_enabled {
        if y >= 80 && y <= 344 && x >= 638 && x <= 778 {
            return HudHitTarget::ToggleMagnet;
        }
    }
    if y >= 10 && y <= 30 {
        if x >= 773 && x <= 798 {
            return HudHitTarget::Close;
        } else if x >= 742 && x <= 767 {
            return HudHitTarget::Minimize;
        }
    }
    if y >= 30 && y <= 50 {
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
    if y >= 170 && y <= 202 {
        if x >= 658 && x <= 698 {
            return HudHitTarget::DecSensitivity;
        } else if x >= 718 && x <= 758 {
            return HudHitTarget::IncSensitivity;
        }
    }
    if y >= 210 && y <= 242 {
        if x >= 658 && x <= 758 {
            return HudHitTarget::TogglePixelMode;
        }
    }
    if y >= 250 && y <= 282 {
        if x >= 658 && x <= 758 {
            return HudHitTarget::ToggleMagnet;
        }
    }
    if y >= 290 && y <= 322 {
        if x >= 658 && x <= 758 {
            return HudHitTarget::ToggleDetail;
        }
    }
    HudHitTarget::None
}

struct HudFonts {
    title: windows_sys::Win32::Graphics::Gdi::HFONT,
    number: windows_sys::Win32::Graphics::Gdi::HFONT,
    key: windows_sys::Win32::Graphics::Gdi::HFONT,
    text: windows_sys::Win32::Graphics::Gdi::HFONT,
}

impl HudFonts {
    unsafe fn create(font_name_ptr: *const u16) -> Self {
        use windows_sys::Win32::Graphics::Gdi::CreateFontW;
        unsafe {
            Self {
                title: CreateFontW(22, 0, 0, 0, 700, 0, 0, 0, 1, 0, 0, 5, 0, font_name_ptr),
                number: CreateFontW(36, 0, 0, 0, 700, 0, 0, 0, 1, 0, 0, 5, 0, font_name_ptr),
                key: CreateFontW(12, 0, 0, 0, 600, 0, 0, 0, 1, 0, 0, 5, 0, font_name_ptr),
                text: CreateFontW(13, 0, 0, 0, 400, 0, 0, 0, 1, 0, 0, 5, 0, font_name_ptr),
            }
        }
    }
}

impl Drop for HudFonts {
    fn drop(&mut self) {
        unsafe {
            DeleteObject(self.title);
            DeleteObject(self.number);
            DeleteObject(self.key);
            DeleteObject(self.text);
        }
    }
}

unsafe fn draw_hud_button(
    hdc: windows_sys::Win32::Graphics::Gdi::HDC,
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    radius: i32,
    bg_color: u32,
    border_color: u32,
    text_color: u32,
    text: &str,
    font: windows_sys::Win32::Graphics::Gdi::HFONT,
    align_flags: u32,
    text_top_offset: i32,
) {
    unsafe {
        let brush = CreateSolidBrush(bg_color);
        let old_brush = SelectObject(hdc, brush);

        // border_color가 Neon Lime Green(0xADFF2F)인 경우 입체 그라데이션/베벨 테두리 효과 적용
        if border_color == 0xADFF2F {
            // 1. 우하단 그림자 테두리 (Dark Green)
            let dark_pen = CreatePen(0, 2, 0x004D20);
            let old_pen = SelectObject(hdc, dark_pen);
            RoundRect(hdc, x1 + 1, y1 + 1, x2 + 1, y2 + 1, radius, radius);
            SelectObject(hdc, old_pen);
            DeleteObject(dark_pen);

            // 2. 좌상단 하이라이트 테두리 (Neon Lime Green)
            let neon_pen = CreatePen(0, 2, 0xADFF2F);
            let old_pen = SelectObject(hdc, neon_pen);
            RoundRect(hdc, x1, y1, x2, y2, radius, radius);
            SelectObject(hdc, old_pen);
            DeleteObject(neon_pen);
        } else {
            let pen = CreatePen(0, 1, border_color);
            let old_pen = SelectObject(hdc, pen);
            RoundRect(hdc, x1, y1, x2, y2, radius, radius);
            SelectObject(hdc, old_pen);
            DeleteObject(pen);
        }

        SelectObject(hdc, old_brush);
        DeleteObject(brush);

        let old_font = SelectObject(hdc, font);
        
        // text_color가 Neon Lime Green(0xADFF2F)인 경우 입체 그라데이션/그림자 텍스트 효과 적용
        if text_color == 0xADFF2F {
            let wide_text = encode_wide(text);
            
            // 1. 다크 그린 그림자 레이어
            SetTextColor(hdc, 0x004D20);
            let mut r_dark = RECT {
                left: x1,
                top: y1 + text_top_offset + 1,
                right: x2,
                bottom: y2,
            };
            DrawTextW(hdc, wide_text.as_ptr(), wide_text.len() as i32 - 1, &mut r_dark, align_flags);

            // 2. 네온 라임 그린 메인 레이어
            SetTextColor(hdc, 0xADFF2F);
            let mut r_neon = RECT {
                left: x1,
                top: y1 + text_top_offset,
                right: x2,
                bottom: y2,
            };
            DrawTextW(hdc, wide_text.as_ptr(), wide_text.len() as i32 - 1, &mut r_neon, align_flags);
        } else {
            SetTextColor(hdc, text_color);
            let wide_text = encode_wide(text);
            let mut r_text = RECT {
                left: x1,
                top: y1 + text_top_offset,
                right: x2,
                bottom: y2,
            };
            DrawTextW(hdc, wide_text.as_ptr(), wide_text.len() as i32 - 1, &mut r_text, align_flags);
        }
        
        SelectObject(hdc, old_font);
    }
}

unsafe extern "system" fn hud_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        match msg {
            WM_PAINT => {
                let mut ps = std::mem::zeroed();
                let hdc = BeginPaint(hwnd, &mut ps);

                let mut rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
                GetClientRect(hwnd, &mut rect);

                let bg_brush = CreateSolidBrush(0x121210);
                let border_pen = CreatePen(0, 2, 0xADFF2F);
                
                let old_brush = SelectObject(hdc, bg_brush);
                let old_pen = SelectObject(hdc, border_pen);
                
                RoundRect(hdc, 0, 0, rect.right, rect.bottom, 16, 16);
                
                SetBkMode(hdc, 1);
                
                let font_name = encode_wide("Segoe UI");
                let fonts = HudFonts::create(font_name.as_ptr());

                let pm_enabled = {
                    let state_arc = crate::hook::APP_STATE.get();
                    state_arc.map_or(false, |arc| arc.lock().unwrap().config.settings.pixel_mode.unwrap_or(false))
                };
                let lang_en = {
                    let state_arc = crate::hook::APP_STATE.get();
                    state_arc.map_or(false, |arc| arc.lock().unwrap().config.settings.lang_en.unwrap_or(false))
                };
                let (is_pro, is_trial) = {
                    let state_arc = crate::hook::APP_STATE.get();
                    state_arc.map_or((false, false), |arc| {
                        let state = arc.lock().unwrap();
                        (state.is_pro, state.is_trial)
                    })
                };

                let q_desc = if lang_en { "Scl ◀" } else { "◀스크롤" };
                let w_desc = if lang_en { "Up ▲" } else { "▲ 이동" };
                let e_desc = if lang_en { "Scl ▶" } else { "스크롤▶" };
                let r_desc = if lang_en { "Whl ▲" } else { "휠▲" };
                let caps_desc = if lang_en { "ON/OFF" } else { "온/오프" };
                let a_desc = if lang_en { "Left ◀" } else { "◀ 이동" };
                let s_desc = if lang_en { "Down ▼" } else { "▼ 이동" };
                let d_desc = if lang_en { "Right ▶" } else { "▶ 이동" };
                let f_desc = if lang_en { "Whl ▼" } else { "휠▼" };
                let g_desc = if lang_en { "R-Clk" } else { "우클릭" };
                let space_desc = if lang_en { "Left Click (1:L / 2:Db / Hold:Drag)" } else { "좌클릭 (1:일반 / 2:더블 / 홀드:드래그)" };
                
                let ui_status = if unsafe { is_process_uiaccess_active() } {
                    if lang_en { " [UIAccess: ON]" } else { " [UIAccess: 활성화]" }
                } else {
                    if lang_en { " [UIAccess: OFF (StartMenu Blocked)]" } else { " [UIAccess: 비활성화 (시작메뉴 가려짐)]" }
                };

                let title_text = if is_pro {
                    if lang_en {
                        format!("Keysor, Keyboard & Cursor as One!{}", ui_status)
                    } else {
                        format!("Keysor, 키보드와 커서를 하나로!{}", ui_status)
                    }
                } else if is_trial {
                    if let Some(days) = crate::license::get_remaining_trial_days() {
                        if lang_en {
                            format!("Keysor Trial ({} days left){}", days, ui_status)
                        } else {
                            format!("키서 평가판 ({}일 남음){}", days, ui_status)
                        }
                    } else if lang_en {
                        format!("Keysor Free (Trial Expired){}", ui_status)
                    } else {
                        format!("키서 무료 버전 (평가판 만료){}", ui_status)
                    }
                } else if lang_en {
                    format!("Keysor Free (Trial Expired){}", ui_status)
                } else {
                    format!("키서 무료 버전 (평가판 만료){}", ui_status)
                };
                let speed_sens_title = "SPEED SENS";
                
                let info1_text = if lang_en {
                    "※ All alphabet typing is blocked during mouse mode (except Ctrl/Alt/Win shortcuts)."
                } else {
                    "※ 마우스 모드 중 모든 알파벳 키 타이핑은 차단됩니다 (Ctrl, Alt, Win 단축키 예외 허용)."
                };
                let info2_text = if lang_en {
                    "• Press Caps Lock again to return to normal keyboard mode."
                } else {
                    "• Caps Lock을 한 번 더 누르면 일반 키보드 상태로 복귀합니다."
                };
                let info_close_text = if lang_en {
                    "• Automatically minimizes when mouse mode is active."
                } else {
                    "• 마우스 활성화 시 자동으로 최소화 됩니다."
                };


                let old_font = SelectObject(hdc, fonts.title);
                let title = encode_wide(&title_text);
                // 1. 타이틀 그림자 레이어 (좌측 정렬)
                SetTextColor(hdc, 0x004D20);
                let mut r_title_dark = RECT { left: 30, top: 31, right: 420, bottom: 61 };
                DrawTextW(hdc, title.as_ptr(), title.len() as i32 - 1, &mut r_title_dark, 0 | 32);

                // 2. 타이틀 메인 레이어 (네온 라임 그린)
                SetTextColor(hdc, 0xADFF2F); // Neon Lime Green title
                let mut r_title = RECT { left: 30, top: 30, right: 420, bottom: 60 };
                DrawTextW(hdc, title.as_ptr(), title.len() as i32 - 1, &mut r_title, 0 | 32);

                // Draw Top Right: Buy Pro and Register License Buttons (Y = 30 ~ 50)
                let hover_val = HUD_HOVER.load(Ordering::SeqCst);
                
                // 10. Buy Pro Button
                let buy_top_label = if is_pro {
                    if lang_en { "Pro Active" } else { "프로 활성화" }
                } else if lang_en {
                    "Buy Pro"
                } else {
                    "프로 결제하기"
                };
                let (buy_top_bg, buy_top_border, buy_top_text_color) = if is_pro {
                    (0x181818, 0x222222, 0xADFF2F) // Keycolor text, dark bg when active/non-clickable
                } else if hover_val == 10 {
                    (0x3C4040, 0xADFF2F, 0xADFF2F)
                } else {
                    (0x202424, 0x3C4040, 0x888888)
                };
                draw_hud_button(hdc, 430, 30, 540, 50, 6, buy_top_bg, buy_top_border, buy_top_text_color, buy_top_label, fonts.key, 37, 0);

                // 11. Register License Button
                let lic_top_label = if lang_en { "License" } else { "라이센스 등록" };
                let (lic_top_bg, lic_top_border, lic_top_text_color) = if hover_val == 11 {
                    (0x3C4040, 0xADFF2F, 0xADFF2F)
                } else {
                    (0x202424, 0x3C4040, 0x888888)
                };
                draw_hud_button(hdc, 546, 30, 656, 50, 6, lic_top_bg, lic_top_border, lic_top_text_color, lic_top_label, fonts.key, 37, 0);

                // Draw Minimize and Close Buttons in top right corner
                // Draw Language Toggle Button
                let (lang_bg, lang_border, lang_text_color) = if hover_val == 6 {
                    (0x3C4040, 0xADFF2F, 0xADFF2F)
                } else {
                    (0x202424, 0x3C4040, 0x888888)
                };
                let lang_btn_text = if lang_en { "KO" } else { "EN" };
                draw_hud_button(hdc, 662, 30, 712, 50, 6, lang_bg, lang_border, lang_text_color, lang_btn_text, fonts.key, 37, 0);
                
                // Draw Minimize Button
                let (min_bg, min_border, min_text_color) = if hover_val == 1 {
                    (0x3C4040, 0xADFF2F, 0xADFF2F)
                } else {
                    (0x202424, 0x3C4040, 0x888888)
                };
                draw_hud_button(hdc, 742, 10, 767, 30, 6, min_bg, min_border, min_text_color, "-", fonts.title, 1 | 32, -3);

                // Draw Close Button
                let (close_bg, close_border, close_text_color) = if hover_val == 2 {
                    (0x1A1A40, 0x0045FF, 0x0045FF)
                } else {
                    (0x202424, 0x3C4040, 0x888888)
                };
                draw_hud_button(hdc, 773, 10, 798, 30, 6, close_bg, close_border, close_text_color, "X", fonts.key, 1 | 32, 0);
                
                // Row 1: Numbers (Y = 80)
                draw_key_cap(hdc, fonts.key, 30, 80, 48, 48, "~", "", 0);
                draw_key_cap(hdc, fonts.key, 84, 80, 48, 48, "1", "", 0);
                draw_key_cap(hdc, fonts.key, 138, 80, 48, 48, "2", "", 0);
                draw_key_cap(hdc, fonts.key, 192, 80, 48, 48, "3", "", 0);
                draw_key_cap(hdc, fonts.key, 246, 80, 48, 48, "4", "", 0);
                draw_key_cap(hdc, fonts.key, 300, 80, 48, 48, "5", "", 0);
                draw_key_cap(hdc, fonts.key, 354, 80, 48, 48, "6", "", 0);
                draw_key_cap(hdc, fonts.key, 408, 80, 48, 48, "7", "", 0);
                draw_key_cap(hdc, fonts.key, 462, 80, 48, 48, "8", "", 0);
                draw_key_cap(hdc, fonts.key, 516, 80, 48, 48, "9", "", 0);
                draw_key_cap(hdc, fonts.key, 570, 80, 48, 48, "0", "", 0);

                // Row 2: Q Row (Y = 134)
                draw_key_cap(hdc, fonts.key, 30, 134, 72, 48, "Tab", "", 0);
                draw_key_cap(hdc, fonts.key, 108, 134, 48, 48, "Q", q_desc, 3);
                draw_key_cap(hdc, fonts.key, 162, 134, 48, 48, "W", w_desc, 2);
                draw_key_cap(hdc, fonts.key, 216, 134, 48, 48, "E", e_desc, 3);
                draw_key_cap(hdc, fonts.key, 270, 134, 48, 48, "R", r_desc, 3);
                draw_key_cap(hdc, fonts.key, 324, 134, 48, 48, "T", "", 0);
                draw_key_cap(hdc, fonts.key, 378, 134, 48, 48, "Y", "", 0);
                draw_key_cap(hdc, fonts.key, 432, 134, 48, 48, "U", "", 0);
                draw_key_cap(hdc, fonts.key, 486, 134, 48, 48, "I", "", 0);
                draw_key_cap(hdc, fonts.key, 540, 134, 48, 48, "O", "", 0);

                // Row 3: A Row (Y = 188)
                draw_key_cap(hdc, fonts.key, 30, 188, 84, 48, "Caps", caps_desc, 1);
                draw_key_cap(hdc, fonts.key, 120, 188, 48, 48, "A", a_desc, 2);
                draw_key_cap(hdc, fonts.key, 174, 188, 48, 48, "S", s_desc, 2);
                draw_key_cap(hdc, fonts.key, 228, 188, 48, 48, "D", d_desc, 2);
                draw_key_cap(hdc, fonts.key, 282, 188, 48, 48, "F", f_desc, 3);
                draw_key_cap(hdc, fonts.key, 336, 188, 48, 48, "G", g_desc, 5);
                draw_key_cap(hdc, fonts.key, 390, 188, 48, 48, "H", "", 0);
                let j_desc = if lang_en { "◀Tab" } else { "◀크롬탭" };
                let k_desc = if lang_en { "Tab▶" } else { "크롬탭▶" };
                draw_key_cap(hdc, fonts.key, 444, 188, 48, 48, "J", j_desc, 6);
                draw_key_cap(hdc, fonts.key, 498, 188, 48, 48, "K", k_desc, 6);
                draw_key_cap(hdc, fonts.key, 552, 188, 48, 48, "L", "", 0);

                // Row 4: Z Row & Enter (Y = 242)
                let shift_desc = if lang_en {
                    "+Q/E Back/Fwd\n+J/K Desktops"
                } else {
                    "+Q/E 뒤로/앞으로\n+J/K 데스크탑 이동"
                };
                draw_key_cap(hdc, fonts.key, 30, 242, 102, 48, "Shift", &shift_desc, 6);
                draw_key_cap(hdc, fonts.key, 138, 242, 48, 48, "Z", "", 0);
                draw_key_cap(hdc, fonts.key, 192, 242, 48, 48, "X", "", 0);
                draw_key_cap(hdc, fonts.key, 246, 242, 48, 48, "C", "", 0);
                draw_key_cap(hdc, fonts.key, 300, 242, 48, 48, "V", "", 0);
                draw_key_cap(hdc, fonts.key, 354, 242, 48, 48, "B", "", 0);
                draw_key_cap(hdc, fonts.key, 408, 242, 48, 48, "N", "", 0);
                draw_key_cap(hdc, fonts.key, 462, 242, 48, 48, "M", "", 0);
                draw_key_cap(hdc, fonts.key, 516, 242, 48, 48, "<", "", 0);

                // Row 5: Modifier & Space (Y = 296)
                draw_key_cap(hdc, fonts.key, 30, 296, 48, 48, "Ctrl", "", 0);
                draw_key_cap(hdc, fonts.key, 84, 296, 48, 48, "Win", "", 0);
                draw_key_cap(hdc, fonts.key, 138, 296, 48, 48, "Alt", "", 0);
                draw_key_cap(hdc, fonts.key, 192, 296, 264, 48, "Spacebar", space_desc, 4);
                draw_key_cap(hdc, fonts.key, 462, 296, 48, 48, "Alt", "", 0);
                draw_key_cap(hdc, fonts.key, 516, 296, 48, 48, "Win", "", 0);

                // Draw Speed Sensitivity Panel
                let features_enabled = is_pro || is_trial;
                let box_bg = if features_enabled { 0x161818 } else { 0x0D0E0E };
                let box_brush = CreateSolidBrush(box_bg);
                let old_box_brush = SelectObject(hdc, box_brush);

                // 1. 스피드센서 박스 그림자 테두리
                let shadow_color = if features_enabled { 0x004D20 } else { 0x1A1C1C };
                let dark_box_pen = CreatePen(0, 2, shadow_color);
                let old_box_pen = SelectObject(hdc, dark_box_pen);
                RoundRect(hdc, 638 + 1, 80 + 1, 778 + 1, 344 + 1, 12, 12);
                SelectObject(hdc, old_box_pen);
                DeleteObject(dark_box_pen);

                // 2. 스피드센서 박스 하이라이트 테두리
                let border_color = if features_enabled { 0xADFF2F } else { 0x3C4040 };
                let neon_box_pen = CreatePen(0, 2, border_color);
                let old_box_pen = SelectObject(hdc, neon_box_pen);
                RoundRect(hdc, 638, 80, 778, 344, 12, 12);
                SelectObject(hdc, old_box_pen);
                DeleteObject(neon_box_pen);

                SelectObject(hdc, old_box_brush);
                DeleteObject(box_brush);

                let sens_title = encode_wide(speed_sens_title);
                // 1. SPEED SENS 타이틀 그림자 레이어
                SetTextColor(hdc, if features_enabled { 0x004D20 } else { 0x151515 });
                let mut r_sens_title_dark = RECT { left: 638, top: 91, right: 778, bottom: 111 };
                DrawTextW(hdc, sens_title.as_ptr(), sens_title.len() as i32 - 1, &mut r_sens_title_dark, 1 | 32);

                // 2. SPEED SENS 타이틀 메인 레이어
                SetTextColor(hdc, if features_enabled { 0xADFF2F } else { 0x444444 });
                let mut r_sens_title = RECT { left: 638, top: 90, right: 778, bottom: 110 };
                DrawTextW(hdc, sens_title.as_ptr(), sens_title.len() as i32 - 1, &mut r_sens_title, 1 | 32);

                let show_all = SHOW_ALL_SENS.load(Ordering::SeqCst);
                if show_all {
                    let (base_speed, max_speed, accel) = {
                        let state_arc = crate::hook::APP_STATE.get();
                        state_arc.map_or((1.0, 30.0, 1.5), |arc| {
                            let state = arc.lock().unwrap();
                            if state.is_pro || state.is_trial {
                                (
                                    state.config.settings.base_speed,
                                    state.config.settings.max_speed,
                                    state.config.settings.acceleration,
                                )
                            } else {
                                (1.0, 30.0, 1.5)
                            }
                        })
                    };
                    
                    let old_font_txt = SelectObject(hdc, fonts.text);
                    SetTextColor(hdc, if is_pro || is_trial { 0xFFFFFF } else { 0x333333 });
                    
                    let line1 = encode_wide(&format!("Base : {:.1}", base_speed));
                    let mut r_l1 = RECT { left: 653, top: 114, right: 773, bottom: 132 };
                    DrawTextW(hdc, line1.as_ptr(), line1.len() as i32 - 1, &mut r_l1, 0);
                    
                    let line2 = encode_wide(&format!("Max  : {:.1}", max_speed));
                    let mut r_l2 = RECT { left: 653, top: 132, right: 773, bottom: 150 };
                    DrawTextW(hdc, line2.as_ptr(), line2.len() as i32 - 1, &mut r_l2, 0);
                    
                    let line3 = encode_wide(&format!("Accel: {:.1}", accel));
                    let mut r_l3 = RECT { left: 653, top: 150, right: 773, bottom: 168 };
                    DrawTextW(hdc, line3.as_ptr(), line3.len() as i32 - 1, &mut r_l3, 0);
                    
                    SelectObject(hdc, old_font_txt);
                } else {
                    let sens_val = {
                        let state_arc = crate::hook::APP_STATE.get();
                        state_arc.map_or(1.0, |arc| {
                            let state = arc.lock().unwrap();
                            if state.is_pro || state.is_trial {
                                state.config.settings.base_speed
                            } else {
                                1.0
                            }
                        })
                    };
                    let old_font_number = SelectObject(hdc, fonts.number);
                    SetTextColor(hdc, if is_pro || is_trial { 0xFFFFFF } else { 0x333333 });
                    let sens_val_text = encode_wide(&format!("{:.1}", sens_val));
                    let mut r_sens_val = RECT { left: 638, top: 114, right: 778, bottom: 159 };
                    DrawTextW(hdc, sens_val_text.as_ptr(), sens_val_text.len() as i32 - 1, &mut r_sens_val, 1 | 32);
                    SelectObject(hdc, old_font_number);
                }

                // Draw [-] Button
                let (dec_bg, dec_border, dec_text_color) = if !(is_pro || is_trial) {
                    (0x121414, 0x222222, 0x333333)
                } else if hover_val == 3 {
                    (0x3C4040, 0xADFF2F, 0xADFF2F)
                } else {
                    (0x202424, 0x3C4040, 0x888888)
                };
                draw_hud_button(hdc, 658, 170, 698, 202, 6, dec_bg, dec_border, dec_text_color, "-", fonts.number, 1 | 32, -8);

                // Draw [+] Button
                let (inc_bg, inc_border, inc_text_color) = if !(is_pro || is_trial) {
                    (0x121414, 0x222222, 0x333333)
                } else if hover_val == 4 {
                    (0x3C4040, 0xADFF2F, 0xADFF2F)
                } else {
                    (0x202424, 0x3C4040, 0x888888)
                };
                draw_hud_button(hdc, 718, 170, 758, 202, 6, inc_bg, inc_border, inc_text_color, "+", fonts.title, 1 | 32, -1);

                // Draw Pixel Mode Toggle Button
                let (pm_bg, pm_border, pm_text_color) = if !(is_pro || is_trial) {
                    (0x121414, 0x222222, 0x333333)
                } else if pm_enabled {
                    if hover_val == 5 { (0xBCFF7A, 0xADFF2F, 0xFFFFFF) } else { (0xADFF2F, 0x3C4040, 0xFFFFFF) }
                } else {
                    if hover_val == 5 { (0x3C4040, 0xADFF2F, 0xFFFFFF) } else { (0x202424, 0x3C4040, 0x888888) }
                };
                let pm_label_str = if lang_en {
                    if pm_enabled { "PIXEL: ON" } else { "PIXEL: OFF" }
                } else {
                    if pm_enabled { "픽셀 단위: ON" } else { "픽셀 단위: OFF" }
                };
                draw_hud_button(hdc, 658, 210, 758, 242, 6, pm_bg, pm_border, pm_text_color, pm_label_str, fonts.key, 37, 0);

                // Draw Magnet Mode Toggle Button
                let magnet_enabled = {
                    let state_arc = crate::hook::APP_STATE.get();
                    state_arc.map_or(false, |arc| arc.lock().unwrap().config.settings.magnetic_mode.unwrap_or(false))
                };
                let (mag_bg, mag_border, mag_text_color) = if !(is_pro || is_trial) {
                    (0x121414, 0x222222, 0x333333)
                } else if magnet_enabled {
                    if hover_val == 7 { (0xBCFF7A, 0xADFF2F, 0xFFFFFF) } else { (0xADFF2F, 0x3C4040, 0xFFFFFF) }
                } else {
                    if hover_val == 7 { (0x3C4040, 0xADFF2F, 0xFFFFFF) } else { (0x202424, 0x3C4040, 0x888888) }
                };
                let mag_label_str = if lang_en {
                    if magnet_enabled { "MAGNET: ON" } else { "MAGNET: OFF" }
                } else {
                    if magnet_enabled { "자석 모드: ON" } else { "자석 모드: OFF" }
                };
                draw_hud_button(hdc, 658, 250, 758, 282, 6, mag_bg, mag_border, mag_text_color, mag_label_str, fonts.key, 37, 0);

                // Draw SENS INFO (Detail) Toggle Button
                let sens_info_enabled = SHOW_ALL_SENS.load(Ordering::SeqCst);
                let (si_bg, si_border, si_text_color) = if !(is_pro || is_trial) {
                    (0x121414, 0x222222, 0x333333)
                } else if sens_info_enabled {
                    if hover_val == 8 { (0xBCFF7A, 0xADFF2F, 0xFFFFFF) } else { (0xADFF2F, 0x3C4040, 0xFFFFFF) }
                } else {
                    if hover_val == 8 { (0x3C4040, 0xADFF2F, 0xFFFFFF) } else { (0x202424, 0x3C4040, 0x888888) }
                };
                let si_label_str = if lang_en {
                    "VIEW DETAIL"
                } else {
                    "상세 감도 보기"
                };
                draw_hud_button(hdc, 658, 290, 758, 322, 6, si_bg, si_border, si_text_color, si_label_str, fonts.key, 37, 0);

                if !(is_pro || is_trial) {
                    let lock_icon = encode_wide("🔒");
                    let old_font_icon = SelectObject(hdc, fonts.title);
                    SetTextColor(hdc, 0x777777);
                    let mut r_lock_icon = RECT { left: 638, top: 160, right: 778, bottom: 200 };
                    DrawTextW(hdc, lock_icon.as_ptr(), lock_icon.len() as i32 - 1, &mut r_lock_icon, 1 | 32);
                    SelectObject(hdc, old_font_icon);
                    
                    let lock_msg = if lang_en {
                        encode_wide("PRO ONLY")
                    } else {
                        encode_wide("프로 전용")
                    };
                    let old_font_msg = SelectObject(hdc, fonts.key);
                    SetTextColor(hdc, 0x888888);
                    let mut r_lock_msg = RECT { left: 638, top: 210, right: 778, bottom: 240 };
                    DrawTextW(hdc, lock_msg.as_ptr(), lock_msg.len() as i32 - 1, &mut r_lock_msg, 1 | 32);
                    SelectObject(hdc, old_font_msg);
                }



                // 4. Draw Info Footer & Warnings
                let old_font_txt = SelectObject(hdc, fonts.text);
                SetTextColor(hdc, 0x888888);
                
                let info1 = encode_wide(info1_text);
                let mut r_info1 = RECT { left: 30, top: 366, right: 778, bottom: 384 };
                DrawTextW(hdc, info1.as_ptr(), info1.len() as i32 - 1, &mut r_info1, 0);

                let info2 = encode_wide(info2_text);
                let mut r_info2 = RECT { left: 30, top: 385, right: 778, bottom: 403 };
                DrawTextW(hdc, info2.as_ptr(), info2.len() as i32 - 1, &mut r_info2, 0);
                
                let info_close = encode_wide(info_close_text);
                let mut r_info_close = RECT { left: 30, top: 404, right: 778, bottom: 422 };
                DrawTextW(hdc, info_close.as_ptr(), info_close.len() as i32 - 1, &mut r_info_close, 0);
                
                // Draw version text at the bottom-right corner
                let version_str = format!("v{}", env!("CARGO_PKG_VERSION"));
                let version_w = encode_wide(&version_str);
                let mut r_version = RECT { left: 600, top: 404, right: 778, bottom: 422 };
                SetTextColor(hdc, 0x666666); // Darker gray for subtle version label
                DrawTextW(hdc, version_w.as_ptr(), version_w.len() as i32 - 1, &mut r_version, 2 | 32);

                SelectObject(hdc, old_font_txt);

                // Cleanup GDI
                SelectObject(hdc, old_font);
                
                SelectObject(hdc, old_brush);
                SelectObject(hdc, old_pen);
                DeleteObject(bg_brush);
                DeleteObject(border_pen);

                EndPaint(hwnd, &ps);
                0
            }

            windows_sys::Win32::UI::WindowsAndMessaging::WM_MOUSEMOVE => {
                let x = (lparam & 0xFFFF) as i16;
                let y = ((lparam >> 16) & 0xFFFF) as i16;
                
                let (is_pro, is_trial) = {
                    let state_arc = crate::hook::APP_STATE.get();
                    state_arc.map_or((false, false), |arc| {
                        let state = arc.lock().unwrap();
                        (state.is_pro, state.is_trial)
                    })
                };
                let prev_hover = HUD_HOVER.load(Ordering::SeqCst);
                let hit = classify_hit_target(x, y, is_pro || is_trial, is_pro);
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
                
                let hit = classify_hit_target(x, y, is_pro || is_trial, is_pro);
                match hit {
                    HudHitTarget::Minimize => {
                        if let Some(&main_hwnd) = MAIN_HWND.get() {
                            ShowWindow(main_hwnd, SW_MINIMIZE);
                        }
                    }
                    HudHitTarget::Close => {
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
                    HudHitTarget::DecSensitivity | HudHitTarget::IncSensitivity | HudHitTarget::TogglePixelMode | HudHitTarget::ToggleMagnet | HudHitTarget::ToggleDetail => {
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
                                HudHitTarget::ToggleDetail => {
                                    SHOW_ALL_SENS.store(true, Ordering::SeqCst);
                                    windows_sys::Win32::UI::Input::KeyboardAndMouse::SetCapture(hwnd);
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
                let cur = SHOW_ALL_SENS.load(Ordering::SeqCst);
                if cur {
                    SHOW_ALL_SENS.store(false, Ordering::SeqCst);
                    InvalidateRect(hwnd, std::ptr::null(), 0);
                }
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

pub unsafe fn set_window_band_safe(hwnd: HWND, band: u32) -> bool {
    let user32 = windows_sys::Win32::System::LibraryLoader::GetModuleHandleW(encode_wide("user32.dll").as_ptr());
    if user32 != 0 {
        let proc = windows_sys::Win32::System::LibraryLoader::GetProcAddress(
            user32,
            b"SetWindowBand\0".as_ptr() as *const _,
        );
        if let Some(set_window_band) = proc {
            let set_window_band: extern "system" fn(HWND, HWND, u32) -> i32 = std::mem::transmute(set_window_band);
            return set_window_band(hwnd, 0, band) != 0;
        }
    }
    false
}
pub unsafe fn create_keysor_cursor() -> windows_sys::Win32::UI::WindowsAndMessaging::HCURSOR {
    use windows_sys::Win32::Graphics::Gdi::*;
    use windows_sys::Win32::Graphics::GdiPlus::*;
    use windows_sys::Win32::UI::WindowsAndMessaging::*;

    let size = 32i32;
    let hdc_screen = GetDC(0);
    let hdc_color = CreateCompatibleDC(hdc_screen);
    let hdc_mask = CreateCompatibleDC(hdc_screen);

    // 32bpp 컬러 비트맵 (알파 블렌딩)
    let mut bmi: BITMAPINFO = std::mem::zeroed();
    bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
    bmi.bmiHeader.biWidth = size;
    bmi.bmiHeader.biHeight = -size;
    bmi.bmiHeader.biPlanes = 1;
    bmi.bmiHeader.biBitCount = 32;
    bmi.bmiHeader.biCompression = BI_RGB;

    let mut bits_color: *mut std::ffi::c_void = std::ptr::null_mut();
    let hbm_color = CreateDIBSection(hdc_color, &bmi, DIB_RGB_COLORS, &mut bits_color, 0, 0);

    // 1bpp 모노크롬 마스크 비트맵 (전부 흰색 = 컬러 비트맵의 알파로 결정)
    let hbm_mask = CreateBitmap(size, size, 1, 1, std::ptr::null());

    if hbm_color == 0 || hbm_mask == 0 || bits_color.is_null() {
        if hbm_color != 0 { DeleteObject(hbm_color); }
        if hbm_mask != 0 { DeleteObject(hbm_mask); }
        DeleteDC(hdc_color);
        DeleteDC(hdc_mask);
        ReleaseDC(0, hdc_screen);
        return 0;
    }

    let old_color = SelectObject(hdc_color, hbm_color);

    // 전체 투명으로 초기화 (alpha = 0)
    std::ptr::write_bytes(bits_color, 0, (size * size * 4) as usize);

    // GDI+로 에메랄드 K 심볼 그리기
    let mut graphics = std::ptr::null_mut();
    if GdipCreateFromHDC(hdc_color, &mut graphics) == 0 {
        GdipSetSmoothingMode(graphics, SmoothingModeAntiAlias);

        // 검은 외곽선 (4px)
        let mut black_pen = std::ptr::null_mut();
        if GdipCreatePen1(0xFF000000u32, 4.0, 2, &mut black_pen) == 0 {
            GdipSetPenStartCap(black_pen, 2);
            GdipSetPenEndCap(black_pen, 2);
            GdipSetPenLineJoin(black_pen, 2);
            GdipDrawLineI(graphics, black_pen, 8, 4, 8, 28);
            GdipDrawLineI(graphics, black_pen, 8, 16, 24, 4);
            GdipDrawLineI(graphics, black_pen, 8, 16, 24, 28);
            GdipDeletePen(black_pen);
        }

        // 에메랄드 그라디언트 채우기 (2.5px)
        let p1 = PointF { X: 8.0, Y: 4.0 };
        let p2 = PointF { X: 24.0, Y: 28.0 };
        let mut brush = std::ptr::null_mut();
        if GdipCreateLineBrush(&p1, &p2, 0xFF2FFFAD_u32, 0xFF00C853_u32, 0, &mut brush) == 0 {
            let mut grad_pen = std::ptr::null_mut();
            if GdipCreatePen2(brush, 2.5, 2, &mut grad_pen) == 0 {
                GdipSetPenStartCap(grad_pen, 2);
                GdipSetPenEndCap(grad_pen, 2);
                GdipSetPenLineJoin(grad_pen, 2);
                GdipDrawLineI(graphics, grad_pen, 8, 4, 8, 28);
                GdipDrawLineI(graphics, grad_pen, 8, 16, 24, 4);
                GdipDrawLineI(graphics, grad_pen, 8, 16, 24, 28);
                GdipDeletePen(grad_pen);
            }
            GdipDeleteBrush(brush);
        }

        GdipDeleteGraphics(graphics);
    }

    SelectObject(hdc_color, old_color);
    DeleteDC(hdc_color);
    DeleteDC(hdc_mask);
    ReleaseDC(0, hdc_screen);

    let info = ICONINFO {
        fIcon: 0,
        xHotspot: 8,
        yHotspot: 8,
        hbmMask: hbm_mask,
        hbmColor: hbm_color,
    };

    let hcursor = CreateIconIndirect(&info);
    DeleteObject(hbm_color);
    DeleteObject(hbm_mask);
    hcursor
}


pub fn start_indicator() {
    start_global_targets_thread();
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
            unsafe {
                set_window_band_safe(main_hwnd, 3);
            }
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
        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_NOACTIVATE,
            class_name_wide.as_ptr(),
            std::ptr::null(),
            WS_POPUP,
            0,
            0,
            32, // width
            32, // height
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
        unsafe {
            set_window_band_safe(hwnd, 3); // ZBID_UIAUTOMATION (공식 UIAccess용 최상위 윈도우 밴드 주입)
        }
        update_indicator_layered_image(hwnd);

        // 6. Create HUD Window
        let screen_width = GetSystemMetrics(0); // SM_CXSCREEN
        let screen_height = GetSystemMetrics(1); // SM_CYSCREEN
        
        let hud_w = 808;
        let hud_h = 452;
        let hud_x = (screen_width - hud_w) / 2;
        let hud_y = (screen_height - hud_h) / 2;

        let hud_hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_NOACTIVATE,
            hud_class_name.as_ptr(),
            std::ptr::null(),
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
        unsafe {
            set_window_band_safe(hud_hwnd, 3);
        }

        // 타이머 제거, 최초 팝업 상시 노출
        ShowWindow(hud_hwnd, SW_SHOWNA);
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

static HIDE_CURSOR_ACTIVE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);


fn hide_system_cursor_internal() {
    // SetSystemCursor는 Chrome Remote Desktop 환경에서 전혀 전송되지 않으며
    // 기본 커서를 오염시키므로 호출하지 않음. 오버레이 창으로만 커서를 표현함.
}

pub fn hide_system_cursor() {
    if HIDE_CURSOR_ACTIVE.swap(true, Ordering::SeqCst) {
        return;
    }
    hide_system_cursor_internal();
}

pub fn force_hide_system_cursor() {
    HIDE_CURSOR_ACTIVE.store(true, Ordering::SeqCst);
    hide_system_cursor_internal();
}

pub fn restore_system_cursor() {
    if !HIDE_CURSOR_ACTIVE.swap(false, Ordering::SeqCst) {
        return;
    }

    unsafe {
        windows_sys::Win32::UI::WindowsAndMessaging::SystemParametersInfoW(
            windows_sys::Win32::UI::WindowsAndMessaging::SPI_SETCURSORS,
            0,
            std::ptr::null_mut(),
            windows_sys::Win32::UI::WindowsAndMessaging::SPIF_SENDCHANGE,
        );

        // 마우스 강제 갱신용 1픽셀 이동 Hack (기본 커서 즉시 렌더링 강제)
        let mut pt = windows_sys::Win32::Foundation::POINT { x: 0, y: 0 };
        if windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut pt) != 0 {
            windows_sys::Win32::UI::WindowsAndMessaging::SetCursorPos(pt.x, pt.y + 1);
            windows_sys::Win32::UI::WindowsAndMessaging::SetCursorPos(pt.x, pt.y);
        }
    }
}

pub fn force_restore_system_cursor() {
    unsafe {
        windows_sys::Win32::UI::WindowsAndMessaging::SystemParametersInfoW(
            windows_sys::Win32::UI::WindowsAndMessaging::SPI_SETCURSORS,
            0,
            std::ptr::null_mut(),
            windows_sys::Win32::UI::WindowsAndMessaging::SPIF_SENDCHANGE | windows_sys::Win32::UI::WindowsAndMessaging::SPIF_UPDATEINIFILE,
        );
        HIDE_CURSOR_ACTIVE.store(false, Ordering::SeqCst);

        // 마우스 강제 갱신용 1픽셀 이동 Hack (기본 커서 즉시 렌더링 강제)
        let mut pt = windows_sys::Win32::Foundation::POINT { x: 0, y: 0 };
        if windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut pt) != 0 {
            windows_sys::Win32::UI::WindowsAndMessaging::SetCursorPos(pt.x, pt.y + 1);
            windows_sys::Win32::UI::WindowsAndMessaging::SetCursorPos(pt.x, pt.y);
        }
    }
}

pub fn show_indicator() {
    println!("[Debug] show_indicator() called");
    // SetSystemCursor로 OS 레벨 마우스 포인터 자체를 K 커서로 교체함.
    // INDICATOR_HWND 오버레이 창은 시작 메뉴에 가려지므로 사용하지 않음.
    hide_system_cursor();
    if let Some(&main_hwnd) = MAIN_HWND.get() {
        unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::ShowWindow(main_hwnd, SW_MINIMIZE);
        }
    }
    // INDICATOR_HWND는 숨겨둔 채로 유지 - 시스템 커서로만 표현
    if let Some(&hwnd) = INDICATOR_HWND.get() {
        unsafe {
            ShowWindow(hwnd, SW_HIDE);
        }
    }
}

pub fn hide_indicator() {
    println!("[Debug] hide_indicator() called");
    force_restore_system_cursor();
    if let Some(&hwnd) = INDICATOR_HWND.get() {
        unsafe {
            ShowWindow(hwnd, SW_HIDE);
        }
    }
    clear_magnetic_snapping();
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

unsafe fn is_system_shell_foreground_with_info(hwnd: HWND, proc_name: &str, class_name: &str) -> bool {
    unsafe {
        if hwnd == 0 {
            return true; // 포커스가 일시 유실되거나 UAC 창 등이 떴을 때는 시스템 커서를 안전하게 복원합니다.
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

        if proc_name.is_empty() {
            return false;
        }

        let mut is_system_shell = false;

        if proc_name == "explorer.exe" {
            if class_name == "Progman" 
                || class_name == "WorkerW" 
                || class_name == "Shell_TrayWnd" 
                || class_name == "SecondaryTrayWnd" 
            {
                is_system_shell = true;
            }
        } else if proc_name == "startmenuexperiencehost.exe" 
            || proc_name == "searchhost.exe" 
            || proc_name == "shellexperiencehost.exe" 
            || proc_name == "applicationframehost.exe"
        {
            is_system_shell = true;
        }

        if is_system_shell {
            // Keysor가 마우스 모드 중이라면 쉘 윈도우 포그라운드로 인해 커서가 숨겨지지 않도록 함
            if let Some(state_arc) = crate::hook::APP_STATE.get() {
                if let Ok(state) = state_arc.try_lock() {
                    if state.is_mouse_mode {
                        return false;
                    }
                }
            }
            return true;
        }

        false
    }
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

fn handle_cursor_visibility(hwnd: HWND, is_suspended: bool, is_visible: &mut bool) {
    unsafe {
        let current_mouse_mode = if let Some(state_arc) = crate::hook::APP_STATE.get() {
            state_arc.try_lock().map(|s| s.is_mouse_mode).unwrap_or(false)
        } else {
            false
        };

        static mut LAST_STATE: Option<(bool, bool)> = None;

        let state_changed = match LAST_STATE {
            Some((m, s)) => m != current_mouse_mode || s != is_suspended,
            None => true,
        };

        if state_changed {
            if current_mouse_mode && !is_suspended {
                force_hide_system_cursor();
            } else {
                force_restore_system_cursor();
            }
            LAST_STATE = Some((current_mouse_mode, is_suspended));
        }

        // 오버레이 창으로 커서 표시 (SetSystemCursor는 CRD에서 전송 안 됨)
        if current_mouse_mode && !is_suspended {
            if !*is_visible {
                windows_sys::Win32::UI::WindowsAndMessaging::ShowWindow(hwnd, windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNA);
                *is_visible = windows_sys::Win32::UI::WindowsAndMessaging::IsWindowVisible(hwnd) != 0;
            }
        } else {
            if *is_visible {
                windows_sys::Win32::UI::WindowsAndMessaging::ShowWindow(hwnd, windows_sys::Win32::UI::WindowsAndMessaging::SW_HIDE);
                *is_visible = false;
            }
        }
    }
}

fn update_click_scale() -> bool {
    let scale_lock = CLICK_SCALE.get_or_init(|| Mutex::new(1.0));
    let mut scale_changed = false;
    if let Ok(mut scale) = scale_lock.lock() {
        if *scale < 1.0 {
            *scale += 0.05;
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
    scale_changed
}

fn calculate_interpolated_pos() -> (f64, f64) {
    let mut pt = POINT { x: 0, y: 0 };
    unsafe {
        windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut pt);
    }
    
    let target_x = (pt.x - 16) as f64;
    let target_y = (pt.y - 16) as f64;
    
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

fn check_state_changed(is_dragging: bool, is_scrolling: bool, is_snapped: bool) -> bool {
    static LAST_INDICATOR_STATE: OnceLock<Mutex<(bool, bool, bool)>> = OnceLock::new();
    let last_state_lock = LAST_INDICATOR_STATE.get_or_init(|| Mutex::new((false, false, false)));
    if let Ok(mut last_state) = last_state_lock.lock() {
        if *last_state != (is_dragging, is_scrolling, is_snapped) {
            *last_state = (is_dragging, is_scrolling, is_snapped);
            true
        } else {
            false
        }
    } else {
        false
    }
}

pub fn update_indicator_position() {
    if let Some(&hwnd) = INDICATOR_HWND.get() {
        unsafe {
            // 1. 마우스 모드 안전성 검사 (비활성화 상태에서 타임 꼬임 방지)
            let is_mouse_mode = if let Some(state_arc) = crate::hook::APP_STATE.get() {
                state_arc.try_lock().map(|s| s.is_mouse_mode).unwrap_or(true)
            } else {
                true
            };

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

            // 3. Foreground window 정보 획득 및 서스펜드 상태 연산
            let fore_hwnd = GetForegroundWindow();


            
            static LAST_FORE_HWND: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
            static CACHED_PROC: OnceLock<Mutex<Option<String>>> = OnceLock::new();
            static CACHED_CLASS: OnceLock<Mutex<Option<String>>> = OnceLock::new();

            let last_hwnd = LAST_FORE_HWND.load(std::sync::atomic::Ordering::SeqCst) as HWND;
            let (proc_name, class_name) = if fore_hwnd == last_hwnd {
                let proc_lock = CACHED_PROC.get_or_init(|| Mutex::new(None));
                let class_lock = CACHED_CLASS.get_or_init(|| Mutex::new(None));
                let p = proc_lock.lock().unwrap().clone();
                let c = class_lock.lock().unwrap().clone();
                if p.is_some() && c.is_some() {
                    (p.unwrap(), c.unwrap())
                } else {
                    let (p_new, c_new) = get_foreground_window_info(fore_hwnd);
                    *proc_lock.lock().unwrap() = Some(p_new.clone());
                    *class_lock.lock().unwrap() = Some(c_new.clone());
                    LAST_FORE_HWND.store(fore_hwnd as usize, std::sync::atomic::Ordering::SeqCst);
                    (p_new, c_new)
                }
            } else {
                let (p_new, c_new) = get_foreground_window_info(fore_hwnd);
                let proc_lock = CACHED_PROC.get_or_init(|| Mutex::new(None));
                let class_lock = CACHED_CLASS.get_or_init(|| Mutex::new(None));
                *proc_lock.lock().unwrap() = Some(p_new.clone());
                *class_lock.lock().unwrap() = Some(c_new.clone());
                LAST_FORE_HWND.store(fore_hwnd as usize, std::sync::atomic::Ordering::SeqCst);
                (p_new, c_new)
            };

            let mut is_shortcut_active = crate::hook::APP_STATE.get()
                .and_then(|arc| arc.try_lock().ok())
                .map(|s| s.is_system_shortcut_active())
                .unwrap_or(false);

            // 훅 유실 등으로 인해 단축키 상태가 꼬였을 가능성을 대비하여 실제 물리 키 상태를 더블체크합니다.
            if is_shortcut_active {
                unsafe {
                    let ctrl_down = (windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(0x11) as u32 & 0x8000) != 0; // VK_CONTROL
                    let menu_down = (windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(0x12) as u32 & 0x8000) != 0; // VK_MENU (Alt)
                    let lwin_down = (windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(0x5B) as u32 & 0x8000) != 0; // VK_LWIN
                    let rwin_down = (windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(0x5C) as u32 & 0x8000) != 0; // VK_RWIN
                    if !ctrl_down && !menu_down && !lwin_down && !rwin_down {
                        is_shortcut_active = false;
                        // APP_STATE도 물리 키 상태에 맞게 동기화해 줍니다.
                        if let Some(state_arc) = crate::hook::APP_STATE.get() {
                            if let Ok(mut state) = state_arc.try_lock() {
                                state.ctrl_pressed = false;
                                state.alt_pressed = false;
                                state.win_pressed = false;
                            }
                        }
                    }
                }
            }

            // 단축키 활성화 상태가 해제되었다면, Alt+Tab 등으로 설정된 SUSPEND_CURSOR_HIDE 플래그를 자동으로 해제합니다.
            if !is_shortcut_active {
                SUSPEND_CURSOR_HIDE.store(false, Ordering::SeqCst);
            }

            let is_shell_active = is_system_shell_foreground_with_info(fore_hwnd, &proc_name, &class_name);
            let is_hide_suspended = SUSPEND_CURSOR_HIDE.load(Ordering::SeqCst);
            let is_suspended = is_shortcut_active || is_shell_active || is_hide_suspended;

            // 4. 디버그 로그 기록
            write_debug_log(fore_hwnd, &proc_name, &class_name, is_suspended, is_shortcut_active, is_shell_active, is_hide_suspended);

            // 5. 창 가시성 및 시스템 커서 토글 처리
            let mut is_visible = windows_sys::Win32::UI::WindowsAndMessaging::IsWindowVisible(hwnd) != 0;
            handle_cursor_visibility(hwnd, is_suspended, &mut is_visible);

            // 6. 클릭 스케일 복구 애니메이션 처리
            let scale_changed_in_loop = update_click_scale();

            // 7. 슬라이딩 보간 좌표 산출
            let (next_x, next_y) = calculate_interpolated_pos();

            // 8. 드래그, 스크롤, 스냅 상태 획득
            let (is_dragging, is_scrolling, is_snapped) = if let Some(state_arc) = crate::hook::APP_STATE.get() {
                state_arc.try_lock().map(|s| {
                    (s.is_dragging, !s.active_scroll_keys.is_empty(), is_currently_snapped())
                }).unwrap_or((false, false, false))
            } else {
                (false, false, false)
            };

            // 9. 상태/스케일 변경 감지 시 레이어드 이미지 즉시 갱신
            let state_changed = check_state_changed(is_dragging, is_scrolling, is_snapped);
            if state_changed || scale_changed_in_loop {
                update_indicator_layered_image(hwnd);
            }

            // 10. 최종 윈도우 위치 동기화 및 강제 드로우
            SetWindowPos(
                hwnd,
                HWND_TOPMOST,
                next_x.round() as i32,
                next_y.round() as i32,
                0,
                0,
                SWP_NOSIZE | SWP_NOACTIVATE,
            );
            InvalidateRect(hwnd, std::ptr::null(), 0);
            UpdateWindow(hwnd);
        }
    }
}

// =========================================================================
// 자석 모드 및 전역 UIA 점프/이탈 누적 연산 엔진 (시각적 잔상 피드백 적용)
// =========================================================================

static GLOBAL_SNAP_TARGETS: OnceLock<Mutex<Vec<(i32, i32)>>> = OnceLock::new();
static LAST_GLOBAL_SNAPPED_POS: OnceLock<Mutex<Option<(i32, i32, bool)>>> = OnceLock::new();
static LAST_JUMP_TIME: OnceLock<Mutex<std::time::Instant>> = OnceLock::new();
static HUD_ESCAPE_ACCUM: OnceLock<Mutex<f64>> = OnceLock::new();
static GLOBAL_ESCAPE_ACCUM: OnceLock<Mutex<f64>> = OnceLock::new();
pub static FORCE_UIA_REFRESH: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

// 인디케이터 부드러운 위치 보간용 static 좌표
static CUR_INDICATOR_POS: OnceLock<Mutex<Option<(f64, f64)>>> = OnceLock::new();

/// 마우스 조작 모드에서의 이동 상태(기본 속도, 이탈 여부, 이동 방향, 키 누름 경과 시간)를 획득합니다.
fn get_movement_status() -> (f64, bool, Option<String>, std::time::Duration) {
    let mut base_speed = 1.0;
    let mut current_speed = 1.0;
    let mut should_release = false;
    let mut new_dir = None;
    let mut hold_duration = std::time::Duration::from_secs(0);
    
    if let Some(state_arc) = crate::hook::APP_STATE.get() {
        let state = state_arc.lock().unwrap();
        base_speed = state.config.settings.base_speed;
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
                if let Ok(elements) = element.find_all(uiautomation::types::TreeScope::Descendants, &condition) {
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
