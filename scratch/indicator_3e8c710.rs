use std::sync::OnceLock;
use std::thread;
use std::sync::atomic::{AtomicU32, Ordering};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM, POINT, RECT};
use windows_sys::Win32::Graphics::Gdi::{
    BeginPaint, EndPaint, CreateSolidBrush, FillRect, DeleteObject, SelectObject, CreatePen, Ellipse,
    InvalidateRect, UpdateWindow, RoundRect, SetTextColor, SetBkMode, DrawTextW,
    CreateFontW
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    RegisterClassW, CreateWindowExW, DefWindowProcW, ShowWindow, SetWindowPos, MSG,
    WNDCLASSW, CS_HREDRAW, CS_VREDRAW, WS_POPUP, WS_EX_LAYERED, WS_EX_TRANSPARENT,
    WS_EX_TOPMOST, WS_EX_NOACTIVATE, SW_HIDE, SW_SHOWNA, LWA_COLORKEY, LWA_ALPHA, WM_PAINT,
    WM_DESTROY, GetMessageW, TranslateMessage, DispatchMessageW, HWND_TOPMOST,
    SWP_NOSIZE, SWP_NOACTIVATE, SetLayeredWindowAttributes, GetClientRect, GetSystemMetrics,
    SetTimer, WS_EX_APPWINDOW, WM_CLOSE, GetSystemMenu, AppendMenuW,
    MF_SEPARATOR, MF_STRING, WM_SYSCOMMAND
};

pub static INDICATOR_HWND: OnceLock<HWND> = OnceLock::new();
pub static HUD_HWND: OnceLock<HWND> = OnceLock::new();
pub static HUD_COUNTDOWN: AtomicU32 = AtomicU32::new(5);

fn encode_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
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
                encode_wide("?쒖옉 ?꾨줈洹몃옩?먯꽌 ?ㅼ냼??Keysor)媛 ?뺤긽 ?댁젣?섏뿀?듬땲??").as_ptr(),
                encode_wide("Keysor ?뚮┝").as_ptr(),
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
                    encode_wide("?쒖옉 ?꾨줈洹몃옩???ㅼ냼??Keysor)媛 ?깅줉?섏뿀?듬땲??\n遺?????먮룞 ?ㅽ뻾?⑸땲??").as_ptr(),
                    encode_wide("Keysor ?뚮┝").as_ptr(),
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
    highlight: bool,
) {
    unsafe {
        let bg_color = 0x202424; 
        let border_color = if highlight { 0x81B910 } else { 0x3C4040 };
        
        let brush = CreateSolidBrush(bg_color);
        let pen = CreatePen(0, 1, border_color);

        let old_brush = SelectObject(hdc, brush);
        let old_pen = SelectObject(hdc, pen);
        let old_font = SelectObject(hdc, font);

        RoundRect(hdc, x, y, x + w, y + h, 8, 8);

        SetTextColor(hdc, if highlight { 0x81B910 } else { 0x888888 });
        let key_w = encode_wide(key_text);
        let mut r_key = RECT { left: x, top: y + 4, right: x + w, bottom: y + 21 };
        DrawTextW(hdc, key_w.as_ptr(), key_w.len() as i32 - 1, &mut r_key, 1 | 32);

        SetTextColor(hdc, if highlight { 0xFFFFFF } else { 0x555555 });
        let desc_w = encode_wide(desc_text);
        let mut r_desc = RECT { left: x + 2, top: y + 23, right: x + w - 2, bottom: y + h - 4 };
        DrawTextW(hdc, desc_w.as_ptr(), desc_w.len() as i32 - 1, &mut r_desc, 1 | 32);

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
            WM_CLOSE | WM_DESTROY => {
                crate::hook::cleanup_hook();
                std::process::exit(0);
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
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
                let hdc = BeginPaint(hwnd, &mut ps);

                let mut rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
                GetClientRect(hwnd, &mut rect);
                let bg_brush = CreateSolidBrush(0xFF00FF);
                FillRect(hdc, &rect, bg_brush);
                DeleteObject(bg_brush);

                let green_color = 0x81B910;
                let green_brush = CreateSolidBrush(green_color);
                let green_pen = CreatePen(0, 1, green_color);

                let old_brush = SelectObject(hdc, green_brush);
                let old_pen = SelectObject(hdc, green_pen);

                Ellipse(hdc, 4, 4, 28, 28);

                let magenta_brush = CreateSolidBrush(0xFF00FF);
                let magenta_pen = CreatePen(0, 1, 0xFF00FF);

                SelectObject(hdc, magenta_brush);
                SelectObject(hdc, magenta_pen);

                Ellipse(hdc, 10, 10, 22, 22);

                SelectObject(hdc, old_brush);
                DeleteObject(green_brush);
                DeleteObject(magenta_brush);

                SelectObject(hdc, old_pen);
                DeleteObject(green_pen);
                DeleteObject(magenta_pen);

                EndPaint(hwnd, &ps);
                0
            }
            WM_DESTROY => {
                0
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
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
                let border_pen = CreatePen(0, 2, 0x81B910);
                
                let old_brush = SelectObject(hdc, bg_brush);
                let old_pen = SelectObject(hdc, border_pen);
                
                RoundRect(hdc, 0, 0, rect.right, rect.bottom, 16, 16);
                
                SetBkMode(hdc, 1);
                
                let font_name = encode_wide("Segoe UI");
                let title_font = CreateFontW(22, 0, 0, 0, 700, 0, 0, 0, 1, 0, 0, 5, 0, font_name.as_ptr());
                let text_font = CreateFontW(13, 0, 0, 0, 400, 0, 0, 0, 1, 0, 0, 5, 0, font_name.as_ptr());
                let key_font = CreateFontW(12, 0, 0, 0, 600, 0, 0, 0, 1, 0, 0, 5, 0, font_name.as_ptr());
                let countdown_font = CreateFontW(18, 0, 0, 0, 700, 0, 0, 0, 1, 0, 0, 5, 0, font_name.as_ptr());
                let number_font = CreateFontW(36, 0, 0, 0, 700, 0, 0, 0, 1, 0, 0, 5, 0, font_name.as_ptr());

                let old_font = SelectObject(hdc, title_font);
                SetTextColor(hdc, 0x81B910);
                let title = encode_wide("??KEYSOR MOUSE EMULATION ACTIVE ??);
                let mut r_title = RECT { left: 20, top: 15, right: rect.right - 20, bottom: 45 };
                DrawTextW(hdc, title.as_ptr(), title.len() as i32 - 1, &mut r_title, 1 | 32);
                
                // Row 1: Numbers (Y = 60)
                draw_key_cap(hdc, key_font, 30, 60, 42, 42, "~", "", false);
                draw_key_cap(hdc, key_font, 78, 60, 42, 42, "1", "", false);
                draw_key_cap(hdc, key_font, 126, 60, 42, 42, "2", "", false);
                draw_key_cap(hdc, key_font, 174, 60, 42, 42, "3", "", false);
                draw_key_cap(hdc, key_font, 222, 60, 42, 42, "4", "", false);
                draw_key_cap(hdc, key_font, 270, 60, 42, 42, "5", "", false);
                draw_key_cap(hdc, key_font, 318, 60, 42, 42, "6", "", false);
                draw_key_cap(hdc, key_font, 366, 60, 42, 42, "7", "", false);
                draw_key_cap(hdc, key_font, 414, 60, 42, 42, "8", "", false);
                draw_key_cap(hdc, key_font, 462, 60, 42, 42, "9", "", false);
                draw_key_cap(hdc, key_font, 510, 60, 42, 42, "0", "", false);

                // Row 2: Q Row (Y = 108)
                draw_key_cap(hdc, key_font, 30, 108, 66, 42, "Tab", "", false);
                draw_key_cap(hdc, key_font, 102, 108, 42, 42, "Q", "", false);
                draw_key_cap(hdc, key_font, 150, 108, 42, 42, "W", "???대룞", true);
                draw_key_cap(hdc, key_font, 198, 108, 42, 42, "E", "", false);
                draw_key_cap(hdc, key_font, 246, 108, 42, 42, "R", "?졻뼯", true);
                draw_key_cap(hdc, key_font, 294, 108, 42, 42, "T", "", false);
                draw_key_cap(hdc, key_font, 342, 108, 42, 42, "Y", "", false);
                draw_key_cap(hdc, key_font, 390, 108, 42, 42, "U", "", false);
                draw_key_cap(hdc, key_font, 438, 108, 42, 42, "I", "", false);
                draw_key_cap(hdc, key_font, 486, 108, 42, 42, "O", "", false);
                draw_key_cap(hdc, key_font, 534, 108, 42, 42, "P", "", false);

                // Row 3: A Row (Y = 156)
                draw_key_cap(hdc, key_font, 30, 156, 78, 42, "Caps", "?댁젣", true);
                draw_key_cap(hdc, key_font, 114, 156, 42, 42, "A", "? ?대룞", true);
                draw_key_cap(hdc, key_font, 162, 156, 42, 42, "S", "???대룞", true);
                draw_key_cap(hdc, key_font, 210, 156, 42, 42, "D", "???대룞", true);
                draw_key_cap(hdc, key_font, 258, 156, 42, 42, "F", "?졻뼹", true);
                draw_key_cap(hdc, key_font, 306, 156, 42, 42, "G", "?고겢", true);
                draw_key_cap(hdc, key_font, 354, 156, 42, 42, "H", "", false);
                draw_key_cap(hdc, key_font, 402, 156, 42, 42, "J", "", false);
                draw_key_cap(hdc, key_font, 450, 156, 42, 42, "K", "", false);
                draw_key_cap(hdc, key_font, 498, 156, 42, 42, "L", "", false);

                // Row 4: Z Row & Enter (Y = 204)
                draw_key_cap(hdc, key_font, 30, 204, 96, 42, "Shift", "", false);
                draw_key_cap(hdc, key_font, 132, 204, 42, 42, "Z", "", false);
                draw_key_cap(hdc, key_font, 180, 204, 42, 42, "X", "", false);
                draw_key_cap(hdc, key_font, 228, 204, 42, 42, "C", "", false);
                draw_key_cap(hdc, key_font, 276, 204, 42, 42, "V", "", false);
                draw_key_cap(hdc, key_font, 324, 204, 42, 42, "B", "", false);
                draw_key_cap(hdc, key_font, 372, 204, 42, 42, "N", "", false);
                draw_key_cap(hdc, key_font, 420, 204, 42, 42, "M", "", false);
                draw_key_cap(hdc, key_font, 468, 204, 108, 42, "Enter", "", false);

                // Row 5: Modifier & Space (Y = 252)
                draw_key_cap(hdc, key_font, 30, 252, 54, 42, "Ctrl", "", false);
                draw_key_cap(hdc, key_font, 90, 252, 54, 42, "Win", "", false);
                draw_key_cap(hdc, key_font, 150, 252, 54, 42, "Alt", "", false);
                draw_key_cap(hdc, key_font, 210, 252, 216, 42, "Spacebar", "?대┃ (1:醫?/ 2:?붾툝 / ????쒕옒洹?", true);
                draw_key_cap(hdc, key_font, 432, 252, 54, 42, "Alt", "", false);
                draw_key_cap(hdc, key_font, 488, 252, 54, 42, "Win", "", false);
                draw_key_cap(hdc, key_font, 548, 252, 28, 42, "", "", false);

                // 3. Draw Arrow Keys (Right Side)
                draw_key_cap(hdc, key_font, 642, 156, 42, 42, "??, "?대룞", true);
                draw_key_cap(hdc, key_font, 594, 204, 42, 42, "?", "?대룞", true);
                draw_key_cap(hdc, key_font, 642, 204, 42, 42, "??, "?대룞", true);
                draw_key_cap(hdc, key_font, 690, 204, 42, 42, "??, "?대룞", true);

                // 4. Draw Info Footer & Countdown text (Y = 310, 330)
                SelectObject(hdc, text_font);
                SetTextColor(hdc, 0x888888);
                let info1 = encode_wide("??留덉슦??紐⑤뱶 以?紐⑤뱺 ?뚰뙆踰?????댄븨? 李⑤떒?⑸땲??(Ctrl, Alt, Win ?⑥텞???덉쇅 ?덉슜).");
                let mut r_info1 = RECT { left: 30, top: 310, right: rect.right - 30, bottom: 328 };
                DrawTextW(hdc, info1.as_ptr(), info1.len() as i32 - 1, &mut r_info1, 1 | 32);

                // [?섏젙] 移댁슫?몃떎??臾멸뎄瑜??곗륫 ?섎떒?쇰줈 ?대룞?섍퀬, ?レ옄留?36px ????고듃濡??ㅼ썙??蹂댁깋(?ㅼ삩 ?ㅻ젋吏-?덈뱶) ?쒕줈??                let current_count = HUD_COUNTDOWN.load(Ordering::SeqCst);
                
                let text_prefix = encode_wide("?깍툘 Auto-close: ");
                let text_number = encode_wide(&format!("{}", current_count));
                let text_suffix = encode_wide("s");

                let mut size_prefix = windows_sys::Win32::Foundation::SIZE { cx: 0, cy: 0 };
                let mut size_number = windows_sys::Win32::Foundation::SIZE { cx: 0, cy: 0 };
                let mut size_suffix = windows_sys::Win32::Foundation::SIZE { cx: 0, cy: 0 };

                SelectObject(hdc, countdown_font);
                windows_sys::Win32::Graphics::Gdi::GetTextExtentPoint32W(hdc, text_prefix.as_ptr(), text_prefix.len() as i32 - 1, &mut size_prefix);
                windows_sys::Win32::Graphics::Gdi::GetTextExtentPoint32W(hdc, text_suffix.as_ptr(), text_suffix.len() as i32 - 1, &mut size_suffix);

                SelectObject(hdc, number_font);
                windows_sys::Win32::Graphics::Gdi::GetTextExtentPoint32W(hdc, text_number.as_ptr(), text_number.len() as i32 - 1, &mut size_number);

                let total_w = size_prefix.cx + size_number.cx + size_suffix.cx;
                let start_x = rect.right - 30 - total_w; // ?곗륫 ?섎떒 (right - 30) ?뺣젹

                // 1. "?깍툘 Auto-close: " 洹몃━湲?(?ㅼ뺄?? ?먮찓?꾨뱶 洹몃┛)
                SelectObject(hdc, countdown_font);
                SetTextColor(hdc, 0x81B910);
                let mut r_prefix = RECT { left: start_x, top: 327, right: start_x + size_prefix.cx, bottom: 357 };
                DrawTextW(hdc, text_prefix.as_ptr(), text_prefix.len() as i32 - 1, &mut r_prefix, 0);

                // 2. ?レ옄 洹몃━湲?(蹂댁깋: ?좊챸???ㅼ삩 ?ㅻ젋吏-?덈뱶 0x0045FF, 36px ???
                SelectObject(hdc, number_font);
                SetTextColor(hdc, 0x0045FF); // BGR: Red=255, Green=69, Blue=0
                let mut r_number = RECT { left: start_x + size_prefix.cx, top: 315, right: start_x + size_prefix.cx + size_number.cx, bottom: 351 };
                DrawTextW(hdc, text_number.as_ptr(), text_number.len() as i32 - 1, &mut r_number, 0);

                // 3. "s" 洹몃━湲?(?ㅼ뺄?? ?먮찓?꾨뱶 洹몃┛)
                SelectObject(hdc, countdown_font);
                SetTextColor(hdc, 0x81B910);
                let mut r_suffix = RECT { left: start_x + size_prefix.cx + size_number.cx, top: 327, right: rect.right - 30, bottom: 357 };
                DrawTextW(hdc, text_suffix.as_ptr(), text_suffix.len() as i32 - 1, &mut r_suffix, 0);

                // 4. ?쇰컲 蹂듦? ?덈궡 ?띿뒪?몃뒗 醫뚯륫 ?섎떒?쇰줈 ?대룞
                SelectObject(hdc, text_font);
                let info2 = encode_wide("??Caps Lock????踰????꾨Ⅴ硫??쇰컲 ?ㅻ낫???곹깭濡?蹂듦??⑸땲??");
                SetTextColor(hdc, 0x888888); // ?쇰컲 ?띿뒪???뚯깋
                let mut r_info2 = RECT { left: 30, top: 331, right: rect.right - 300, bottom: 349 };
                DrawTextW(hdc, info2.as_ptr(), info2.len() as i32 - 1, &mut r_info2, 0);

                // Cleanup GDI
                SelectObject(hdc, old_font);
                DeleteObject(title_font);
                DeleteObject(text_font);
                DeleteObject(key_font);
                DeleteObject(countdown_font);
                DeleteObject(number_font);
                
                SelectObject(hdc, old_brush);
                SelectObject(hdc, old_pen);
                DeleteObject(bg_brush);
                DeleteObject(border_pen);

                EndPaint(hwnd, &ps);
                0
            }
            windows_sys::Win32::UI::WindowsAndMessaging::WM_TIMER => {
                if wparam == 1 {
                    let count = HUD_COUNTDOWN.load(Ordering::SeqCst);
                    if count > 1 {
                        HUD_COUNTDOWN.store(count - 1, Ordering::SeqCst);
                        InvalidateRect(hwnd, std::ptr::null(), 1);
                    } else {
                        windows_sys::Win32::UI::WindowsAndMessaging::KillTimer(hwnd, 1);
                        ShowWindow(hwnd, SW_HIDE);
                    }
                }
                0
            }
            WM_DESTROY => {
                0
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

pub fn start_indicator() {
    thread::spawn(|| unsafe {
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

        // 2. Register Indicator class
        let class_name_wide = encode_wide("KeysorIndicatorClass");
        let wnd_class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(indicator_wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: instance,
            hIcon: 0,
            hCursor: 0,
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
            hCursor: 0,
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
            WS_POPUP,
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
            // ?쒖뒪??硫붾돱瑜??살뼱??而ㅼ뒪? ??ぉ 異붽?
            let sys_menu = GetSystemMenu(main_hwnd, 0);
            if sys_menu != 0 {
                AppendMenuW(sys_menu, MF_SEPARATOR, 0, std::ptr::null());
                AppendMenuW(sys_menu, MF_STRING, 1001, encode_wide("?숋툘 Keysor ?ㅼ젙 ?닿린 (keysor.yaml)").as_ptr());
                AppendMenuW(sys_menu, MF_STRING, 1002, encode_wide("?뱛 ?ㅼ젙 ?대뜑 ?닿린 (.keysor)").as_ptr());
                AppendMenuW(sys_menu, MF_STRING, 1003, encode_wide("?? ?쒖옉 ?꾨줈洹몃옩 ?깅줉/?댁젣 ?좉?").as_ptr());
            }
            ShowWindow(main_hwnd, SW_SHOWNA);
        }

        // 5. Create Indicator Window
        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_NOACTIVATE,
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

        SetLayeredWindowAttributes(hwnd, 0xFF00FF, 0, LWA_COLORKEY);
        INDICATOR_HWND.set(hwnd).ok();

        // 6. Create HUD Window
        let screen_width = GetSystemMetrics(0); // SM_CXSCREEN
        let screen_height = GetSystemMetrics(1); // SM_CYSCREEN
        
        let hud_w = 760;
        let hud_h = 380;
        let hud_x = (screen_width - hud_w) / 2;
        let hud_y = (screen_height - hud_h) / 2;

        let hud_hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_NOACTIVATE,
            hud_class_name.as_ptr(),
            std::ptr::null(),
            WS_POPUP,
            hud_x,
            hud_y,
            hud_w,
            hud_h,
            0,
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

        // 理쒖큹 1?뚮쭔 ?붾㈃ ?뺤쨷?숈뿉 5珥??숈븞 移댁슫?몃떎????대㉧ ?몄텧
        HUD_COUNTDOWN.store(5, Ordering::SeqCst);
        ShowWindow(hud_hwnd, SW_SHOWNA);
        InvalidateRect(hud_hwnd, std::ptr::null(), 1);
        UpdateWindow(hud_hwnd);
        SetTimer(hud_hwnd, 1, 1000, None); // 1珥?媛꾧꺽 吏멸퉵吏멸퉵 ??대㉧ 援щ룞

        println!("[Info] Indicator & HUD windows created successfully.");

        // Run message loop
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, 0, 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    });
}

pub fn show_indicator() {
    println!("[Debug] show_indicator() called");
    if let Some(&hwnd) = INDICATOR_HWND.get() {
        unsafe {
            let mut pt = POINT { x: 0, y: 0 };
            windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut pt);
            SetWindowPos(
                hwnd,
                HWND_TOPMOST,
                pt.x - 16,
                pt.y - 16,
                0,
                0,
                SWP_NOSIZE | SWP_NOACTIVATE,
            );
            ShowWindow(hwnd, SW_SHOWNA);
            InvalidateRect(hwnd, std::ptr::null(), 1);
            UpdateWindow(hwnd);
        }
    }
}

pub fn hide_indicator() {
    println!("[Debug] hide_indicator() called");
    if let Some(&hwnd) = INDICATOR_HWND.get() {
        unsafe {
            ShowWindow(hwnd, SW_HIDE);
        }
    }
}

pub fn update_indicator_position() {
    if let Some(&hwnd) = INDICATOR_HWND.get() {
        unsafe {
            let mut pt = POINT { x: 0, y: 0 };
            windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut pt);
            SetWindowPos(
                hwnd,
                HWND_TOPMOST,
                pt.x - 16,
                pt.y - 16,
                0,
                0,
                SWP_NOSIZE | SWP_NOACTIVATE,
            );
            InvalidateRect(hwnd, std::ptr::null(), 1);
            UpdateWindow(hwnd);
        }
    }
}
