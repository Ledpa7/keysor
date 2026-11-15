//! Main HUD painting. Input dispatch and window lifecycle stay in win_gdi.
#[path = "hud/drawing.rs"]
mod drawing;
use super::{HUD_HOVER, SHOW_ALL_SENS, check_autostart_status_win, encode_wide};
use drawing::{HudFonts, draw_hud_button, draw_hud_toggle_row, draw_key_cap};
use std::sync::atomic::Ordering;
use windows_sys::Win32::Foundation::{HWND, LRESULT, RECT};
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::UI::WindowsAndMessaging::GetClientRect;

struct HudLabels {
    q_desc: &'static str,
    w_desc: &'static str,
    e_desc: &'static str,
    r_desc: &'static str,
    caps_desc: &'static str,
    win_desc: &'static str,
    a_desc: &'static str,
    s_desc: &'static str,
    d_desc: &'static str,
    f_desc: &'static str,
    g_desc: &'static str,
    space_desc: &'static str,
    title_text: String,
    speed_sens_title: &'static str,
    info1_text: &'static str,
    info2_text: &'static str,
    info_close_text: &'static str,
    shortcut_info_text: &'static str,
}
impl HudLabels {
    fn new(lang_en: bool, is_pro: bool, is_trial: bool) -> Self {
        let q_desc = if lang_en { "Scl ◀" } else { "◀스크롤" };
        let w_desc = if lang_en { "Up ▲" } else { "▲ 이동" };
        let e_desc = if lang_en { "Scl ▶" } else { "스크롤▶" };
        let r_desc = if lang_en { "Whl ▲" } else { "휠▲" };
        let caps_desc = if lang_en { "ON/OFF" } else { "온/오프" };
        let win_desc = "+ 1~9";
        let a_desc = if lang_en { "Left ◀" } else { "◀ 이동" };
        let s_desc = if lang_en { "Down ▼" } else { "▼ 이동" };
        let d_desc = if lang_en { "Right ▶" } else { "▶ 이동" };
        let f_desc = if lang_en { "Whl ▼" } else { "휠▼" };
        let g_desc = if lang_en { "R-Clk" } else { "우클릭" };
        let space_desc = if lang_en {
            "Left Click (1:L / 2:Db / +Shift:Enter)"
        } else {
            "좌클릭 (1:일반 / 2:더블 / +Shift:엔터)"
        };

        let title_text = if is_pro {
            if lang_en {
                format!("Keysor, Keyboard & Cursor as One!")
            } else {
                format!("Keysor, 키보드와 커서를 하나로!")
            }
        } else if is_trial {
            if let Some(days) = crate::license::get_remaining_trial_days() {
                if lang_en {
                    format!("Keysor Trial ({} days left)", days)
                } else {
                    format!("키서 평가판 ({}일 남음)", days)
                }
            } else if lang_en {
                format!("Keysor Free (Trial Expired)")
            } else {
                format!("키서 무료 버전 (평가판 만료)")
            }
        } else if lang_en {
            format!("Keysor Free (Trial Expired)")
        } else {
            format!("키서 무료 버전 (평가판 만료)")
        };
        let speed_sens_title = "SPEED SENS";

        let info1_text = if lang_en {
            "※ All alphabet typing is blocked during mouse mode (except Ctrl/Alt/Win shortcuts)."
        } else {
            "※ 마우스 모드 중 모든 알파벳 키 타이핑은 차단됩니다 (Ctrl, Alt, Win 단축키 예외 허용)."
        };
        let info2_text = if lang_en {
            "• Caps Lock: mouse mode ON/OFF · Hold 0.5s to reopen this popup."
        } else {
            "• Caps Lock: 마우스 모드 켜기/끄기 · 0.5초 길게 누르면 팝업 다시 열기"
        };
        let info_close_text = if lang_en {
            "• Automatically minimizes when mouse mode is active."
        } else {
            "• 마우스 활성화 시 자동으로 최소화 됩니다."
        };
        let shortcut_info_text = if lang_en {
            "• Hold Win 1s: shortcut guide · Win + 1–9: open/switch taskbar apps."
        } else {
            "• Win 1초 길게: 단축키 안내 · Win + 1~9: 작업 표시줄 앱 실행/전환"
        };
        Self {
            q_desc,
            w_desc,
            e_desc,
            r_desc,
            caps_desc,
            win_desc,
            a_desc,
            s_desc,
            d_desc,
            f_desc,
            g_desc,
            space_desc,
            title_text,
            speed_sens_title,
            info1_text,
            info2_text,
            info_close_text,
            shortcut_info_text,
        }
    }
}

pub(super) unsafe fn paint(hwnd: HWND) -> LRESULT {
    unsafe {
        let mut ps = std::mem::zeroed();
        let hdc_paint = BeginPaint(hwnd, &mut ps);

        let mut rect = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        GetClientRect(hwnd, &mut rect);
        let width = rect.right;
        let height = rect.bottom;

        // Double Buffering: Create memory DC & Bitmap compatible with screen DC
        let hdc = CreateCompatibleDC(hdc_paint);
        let h_bm = CreateCompatibleBitmap(hdc_paint, width, height);
        let old_bm = SelectObject(hdc, h_bm);

        // Set GDI mapping mode to automatically scale the 808x452 design coordinates
        SetMapMode(hdc, MM_ANISOTROPIC);
        SetWindowExtEx(hdc, 808, 452, std::ptr::null_mut());
        SetViewportExtEx(hdc, width, height, std::ptr::null_mut());

        let bg_brush = CreateSolidBrush(0x121210);
        let border_pen = CreatePen(0, 2, 0xADFF2F);

        let old_brush = SelectObject(hdc, bg_brush);
        let old_pen = SelectObject(hdc, border_pen);

        RoundRect(hdc, 0, 0, 808, 452, 16, 16);

        SetBkMode(hdc, 1);

        let font_name = encode_wide("Segoe UI");
        let fonts = HudFonts::create(font_name.as_ptr());

        let lang_en = {
            let state_arc = crate::hook::APP_STATE.get();
            state_arc.map_or(false, |arc| {
                arc.lock().unwrap().config.settings.lang_en.unwrap_or(false)
            })
        };
        let (is_pro, is_trial) = {
            let state_arc = crate::hook::APP_STATE.get();
            state_arc.map_or((false, false), |arc| {
                let state = arc.lock().unwrap();
                (state.is_pro, state.is_trial)
            })
        };

        let labels = HudLabels::new(lang_en, is_pro, is_trial);
        let old_font = SelectObject(hdc, fonts.title);
        draw_header(hdc, &fonts, lang_en, is_pro, &labels);
        draw_keyboard(hdc, &fonts, lang_en, &labels);
        draw_sensitivity(hdc, &fonts, lang_en, is_pro, is_trial, &labels);
        draw_footer(hdc, &fonts, lang_en, &labels);
        // Cleanup GDI
        SelectObject(hdc, old_font);

        SelectObject(hdc, old_brush);
        SelectObject(hdc, old_pen);
        DeleteObject(bg_brush);
        DeleteObject(border_pen);

        // Restore memory DC mapping mode to MM_TEXT to copy pixels 1:1 without void stretching
        SetMapMode(hdc, MM_TEXT);

        // Double Buffering: Copy offscreen bitmap to screen DC, and delete compatibility objects
        BitBlt(hdc_paint, 0, 0, width, height, hdc, 0, 0, SRCCOPY);
        SelectObject(hdc, old_bm);
        DeleteObject(h_bm);
        DeleteDC(hdc);

        EndPaint(hwnd, &ps);
        0
    }
}

unsafe fn draw_header(
    hdc: isize,
    fonts: &HudFonts,
    lang_en: bool,
    is_pro: bool,
    labels: &HudLabels,
) {
    unsafe {
        let title = encode_wide(&labels.title_text);
        // 1. 타이틀 그림자 레이어 (좌측 정렬)
        SetTextColor(hdc, 0x004D20);
        let mut r_title_dark = RECT {
            left: 30,
            top: 31,
            right: 420,
            bottom: 61,
        };
        DrawTextW(
            hdc,
            title.as_ptr(),
            title.len() as i32 - 1,
            &mut r_title_dark,
            0 | 32,
        );

        // 2. 타이틀 메인 레이어 (네온 라임 그린)
        SetTextColor(hdc, 0xADFF2F); // Neon Lime Green title
        let mut r_title = RECT {
            left: 30,
            top: 30,
            right: 420,
            bottom: 60,
        };
        DrawTextW(
            hdc,
            title.as_ptr(),
            title.len() as i32 - 1,
            &mut r_title,
            0 | 32,
        );

        // Draw Top Right: Buy Pro and Register License Buttons (Y = 30 ~ 50)
        let hover_val = HUD_HOVER.load(Ordering::SeqCst);

        // 10. Buy Pro Button
        let buy_top_label = if is_pro {
            if lang_en {
                "Pro Active"
            } else {
                "프로 활성화"
            }
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
        draw_hud_button(
            hdc,
            430,
            30,
            540,
            50,
            6,
            buy_top_bg,
            buy_top_border,
            buy_top_text_color,
            buy_top_label,
            fonts.key,
            37,
            0,
        );

        // 11. Register License Button
        let lic_top_label = if lang_en {
            "License"
        } else {
            "라이센스 등록"
        };
        let (lic_top_bg, lic_top_border, lic_top_text_color) = if hover_val == 11 {
            (0x3C4040, 0xADFF2F, 0xADFF2F)
        } else {
            (0x202424, 0x3C4040, 0x888888)
        };
        draw_hud_button(
            hdc,
            546,
            30,
            656,
            50,
            6,
            lic_top_bg,
            lic_top_border,
            lic_top_text_color,
            lic_top_label,
            fonts.key,
            37,
            0,
        );

        // Draw Minimize and Close Buttons in top right corner
        // Draw Language Toggle Button
        let (lang_bg, lang_border, lang_text_color) = if hover_val == 6 {
            (0x3C4040, 0xADFF2F, 0xADFF2F)
        } else {
            (0x202424, 0x3C4040, 0x888888)
        };
        let lang_btn_text = if lang_en { "KO" } else { "EN" };
        draw_hud_button(
            hdc,
            662,
            30,
            712,
            50,
            6,
            lang_bg,
            lang_border,
            lang_text_color,
            lang_btn_text,
            fonts.key,
            37,
            0,
        );

        // Draw Minimize Button
        let (min_bg, min_border, min_text_color) = if hover_val == 1 {
            (0x3C4040, 0xADFF2F, 0xADFF2F)
        } else {
            (0x202424, 0x3C4040, 0x888888)
        };
        draw_hud_button(
            hdc,
            742,
            10,
            767,
            30,
            6,
            min_bg,
            min_border,
            min_text_color,
            "-",
            fonts.title,
            37,
            0,
        );

        // Draw Close Button
        let (close_bg, close_border, close_text_color) = if hover_val == 2 {
            (0x1A1A40, 0x0045FF, 0x0045FF)
        } else {
            (0x202424, 0x3C4040, 0x888888)
        };
        draw_hud_button(
            hdc,
            773,
            10,
            798,
            30,
            6,
            close_bg,
            close_border,
            close_text_color,
            "X",
            fonts.key,
            37,
            0,
        );
    }
}

fn draw_keyboard(hdc: isize, fonts: &HudFonts, lang_en: bool, labels: &HudLabels) {
    let j_desc = if lang_en { "◀Tab" } else { "◀크롬탭" };
    let k_desc = if lang_en { "Tab▶" } else { "크롬탭▶" };
    let shift_desc = if lang_en {
        "+Q/E Back/Fwd\n+J/K Desktops"
    } else {
        "+Q/E 뒤로/앞으로\n+J/K 데스크탑 이동"
    };
    let keys: &[(i32, i32, i32, i32, &str, &str, u32)] = &[
        // Row 1: Numbers (Y = 80)
        (30, 80, 48, 48, "~", "", 0),
        (84, 80, 48, 48, "1", "", 0),
        (138, 80, 48, 48, "2", "", 0),
        (192, 80, 48, 48, "3", "", 0),
        (246, 80, 48, 48, "4", "", 0),
        (300, 80, 48, 48, "5", "", 0),
        (354, 80, 48, 48, "6", "", 0),
        (408, 80, 48, 48, "7", "", 0),
        (462, 80, 48, 48, "8", "", 0),
        (516, 80, 48, 48, "9", "", 0),
        (570, 80, 48, 48, "0", "", 0),
        // Row 2: Q Row (Y = 134)
        (30, 134, 72, 48, "Tab", "", 0),
        (108, 134, 48, 48, "Q", labels.q_desc, 3),
        (162, 134, 48, 48, "W", labels.w_desc, 2),
        (216, 134, 48, 48, "E", labels.e_desc, 3),
        (270, 134, 48, 48, "R", labels.r_desc, 3),
        (324, 134, 48, 48, "T", "", 0),
        (378, 134, 48, 48, "Y", "", 0),
        (432, 134, 48, 48, "U", "", 0),
        (486, 134, 48, 48, "I", "", 0),
        (540, 134, 48, 48, "O", "", 0),
        // Row 3: A Row (Y = 188)
        (30, 188, 84, 48, "Caps", labels.caps_desc, 1),
        (120, 188, 48, 48, "A", labels.a_desc, 2),
        (174, 188, 48, 48, "S", labels.s_desc, 2),
        (228, 188, 48, 48, "D", labels.d_desc, 2),
        (282, 188, 48, 48, "F", labels.f_desc, 3),
        (336, 188, 48, 48, "G", labels.g_desc, 5),
        (390, 188, 48, 48, "H", "", 0),
        (444, 188, 48, 48, "J", j_desc, 6),
        (498, 188, 48, 48, "K", k_desc, 6),
        (552, 188, 48, 48, "L", "", 0),
        // Row 4: Z Row & Enter (Y = 242)
        (30, 242, 102, 48, "Shift", &shift_desc, 6),
        (138, 242, 48, 48, "Z", "", 0),
        (192, 242, 48, 48, "X", "", 0),
        (246, 242, 48, 48, "C", "", 0),
        (300, 242, 48, 48, "V", "", 0),
        (354, 242, 48, 48, "B", "", 0),
        (408, 242, 48, 48, "N", "", 0),
        (462, 242, 48, 48, "M", "", 0),
        (516, 242, 48, 48, "<", "", 0),
        // Row 5: Modifier & Space (Y = 296)
        (30, 296, 48, 48, "Ctrl", "", 0),
        (84, 296, 48, 48, "Win", labels.win_desc, 7),
        (138, 296, 48, 48, "Alt", "", 0),
        (192, 296, 264, 48, "Spacebar", labels.space_desc, 4),
        (462, 296, 48, 48, "Alt", "", 0),
    ];
    for &(x, y, w, h, key, description, category) in keys {
        draw_key_cap(hdc, fonts.key, x, y, w, h, key, description, category);
    }
}

unsafe fn draw_sensitivity(
    hdc: isize,
    fonts: &HudFonts,
    lang_en: bool,
    is_pro: bool,
    is_trial: bool,
    labels: &HudLabels,
) {
    unsafe {
        draw_settings_frame(hdc, fonts, is_pro || is_trial, labels);
        draw_speed_value(hdc, fonts, lang_en, is_pro || is_trial);
        draw_settings_controls(hdc, fonts, lang_en, is_pro, is_trial);
        if !(is_pro || is_trial) {
            draw_pro_lock(hdc, fonts, lang_en);
        }
    }
}

unsafe fn draw_settings_frame(
    hdc: isize,
    fonts: &HudFonts,
    features_enabled: bool,
    labels: &HudLabels,
) {
    unsafe {
        // Draw Speed Sensitivity Panel

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

        let sens_title = encode_wide(labels.speed_sens_title);
        let old_font_title = SelectObject(hdc, fonts.title);
        // 1. SPEED SENS 타이틀 그림자 레이어
        SetTextColor(hdc, if features_enabled { 0x004D20 } else { 0x151515 });
        let mut r_sens_title_dark = RECT {
            left: 638,
            top: 88,
            right: 778,
            bottom: 106,
        };
        DrawTextW(
            hdc,
            sens_title.as_ptr(),
            sens_title.len() as i32 - 1,
            &mut r_sens_title_dark,
            1 | 4,
        );

        // 2. SPEED SENS 타이틀 메인 레이어
        SetTextColor(hdc, if features_enabled { 0xADFF2F } else { 0x444444 });
        let mut r_sens_title = RECT {
            left: 638,
            top: 87,
            right: 778,
            bottom: 105,
        };
        DrawTextW(
            hdc,
            sens_title.as_ptr(),
            sens_title.len() as i32 - 1,
            &mut r_sens_title,
            1 | 4,
        );
        SelectObject(hdc, old_font_title);
    }
}

unsafe fn draw_speed_value(hdc: isize, fonts: &HudFonts, lang_en: bool, features_enabled: bool) {
    unsafe {
        // 3. 디스플레이 화면 전용 내부 박스 (LCD Screen: Y: 114 ~ 150, 하단 버튼과 4px 타이트 여백)
        let disp_bg = if features_enabled { 0x090B0B } else { 0x070808 };
        let disp_brush = CreateSolidBrush(disp_bg);
        let old_disp_brush = SelectObject(hdc, disp_brush);

        let disp_border = if features_enabled { 0x2A3030 } else { 0x1A1C1C };
        let disp_pen = CreatePen(0, 1, disp_border);
        let old_disp_pen = SelectObject(hdc, disp_pen);

        RoundRect(hdc, 646, 114, 770, 150, 5, 5);

        SelectObject(hdc, old_disp_pen);
        DeleteObject(disp_pen);
        SelectObject(hdc, old_disp_brush);
        DeleteObject(disp_brush);

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
            SetTextColor(hdc, if features_enabled { 0xFFFFFF } else { 0x444444 });

            let line1 = encode_wide(&format!("Base: {:.1}", base_speed));
            let mut r_l1 = RECT {
                left: 652,
                top: 118,
                right: 708,
                bottom: 133,
            };
            DrawTextW(hdc, line1.as_ptr(), line1.len() as i32 - 1, &mut r_l1, 0);

            let line2 = encode_wide(&format!("Max: {:.1}", max_speed));
            let mut r_l2 = RECT {
                left: 708,
                top: 118,
                right: 764,
                bottom: 133,
            };
            DrawTextW(hdc, line2.as_ptr(), line2.len() as i32 - 1, &mut r_l2, 0);

            let line3 = encode_wide(&format!("Accel: {:.1}", accel));
            let mut r_l3 = RECT {
                left: 652,
                top: 134,
                right: 764,
                bottom: 149,
            };
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
            SetTextColor(hdc, if features_enabled { 0xFFFFFF } else { 0x444444 });
            let sens_val_text = encode_wide(&format!("{:.1}", sens_val));
            let mut r_sens_val = RECT {
                left: 646,
                top: 113,
                right: 770,
                bottom: 149,
            };
            DrawTextW(
                hdc,
                sens_val_text.as_ptr(),
                sens_val_text.len() as i32 - 1,
                &mut r_sens_val,
                1 | 32 | 4,
            );
            SelectObject(hdc, old_font_number);
        }
    }
}

unsafe fn draw_settings_controls(
    hdc: isize,
    fonts: &HudFonts,
    lang_en: bool,
    is_pro: bool,
    is_trial: bool,
) {
    unsafe {
        let hover_val = HUD_HOVER.load(Ordering::SeqCst);

        // Draw [-] Button
        let (dec_bg, dec_border, dec_text_color) = if !(is_pro || is_trial) {
            (0x121414, 0x222222, 0x333333)
        } else if hover_val == 3 {
            (0x3C4040, 0xADFF2F, 0xADFF2F)
        } else {
            (0x202424, 0x3C4040, 0x888888)
        };
        draw_hud_button(
            hdc,
            648,
            154,
            703,
            186,
            5,
            dec_bg,
            dec_border,
            dec_text_color,
            "-",
            fonts.title,
            37,
            -3,
        );

        // Draw [+] Button
        let (inc_bg, inc_border, inc_text_color) = if !(is_pro || is_trial) {
            (0x121414, 0x222222, 0x333333)
        } else if hover_val == 4 {
            (0x3C4040, 0xADFF2F, 0xADFF2F)
        } else {
            (0x202424, 0x3C4040, 0x888888)
        };
        draw_hud_button(
            hdc,
            713,
            154,
            768,
            186,
            5,
            inc_bg,
            inc_border,
            inc_text_color,
            "+",
            fonts.title,
            37,
            -4,
        );

        let features_enabled = is_pro || is_trial;

        // 1. Draw Grid Mode Toggle Button
        let pm_enabled = {
            let state_arc = crate::hook::APP_STATE.get();
            state_arc.map_or(false, |arc| {
                arc.lock()
                    .unwrap()
                    .config
                    .settings
                    .pixel_mode
                    .unwrap_or(false)
            })
        };
        let pm_label_str = if lang_en {
            "Grid Mode"
        } else {
            "그리드 모드"
        };
        draw_hud_toggle_row(
            hdc,
            648,
            191,
            768,
            223,
            5,
            pm_enabled,
            hover_val == 5,
            features_enabled,
            pm_label_str,
            fonts.key,
        );

        // 2. Draw Magnet Mode Toggle Button
        let magnet_enabled = {
            let state_arc = crate::hook::APP_STATE.get();
            state_arc.map_or(false, |arc| {
                arc.lock()
                    .unwrap()
                    .config
                    .settings
                    .magnetic_mode
                    .unwrap_or(false)
            })
        };
        let mag_label_str = if lang_en {
            "Magnet Mode"
        } else {
            "자석 모드"
        };
        draw_hud_toggle_row(
            hdc,
            648,
            228,
            768,
            260,
            5,
            magnet_enabled,
            hover_val == 7,
            features_enabled,
            mag_label_str,
            fonts.key,
        );

        // 3. Draw Auto-Start Toggle Button
        let autostart_enabled = check_autostart_status_win();
        let auto_label_str = if lang_en {
            "Auto-Start"
        } else {
            "자동 실행"
        };
        draw_hud_toggle_row(
            hdc,
            648,
            265,
            768,
            297,
            5,
            autostart_enabled,
            hover_val == 9,
            features_enabled,
            auto_label_str,
            fonts.key,
        );

        // 4. Draw Details Toggle Button
        let sens_info_enabled = SHOW_ALL_SENS.load(Ordering::SeqCst);
        let (si_bg, si_border, si_text_color) = if !(is_pro || is_trial) {
            (0x121414, 0x222222, 0x333333)
        } else if sens_info_enabled {
            if hover_val == 8 {
                (0xBCFF7A, 0xADFF2F, 0x000000)
            } else {
                (0xADFF2F, 0x3C4040, 0x000000)
            }
        } else {
            if hover_val == 8 {
                (0x3C4040, 0xADFF2F, 0xFFFFFF)
            } else {
                (0x202424, 0x3C4040, 0x888888)
            }
        };
        let si_label_str = if !(is_pro || is_trial) {
            ""
        } else if sens_info_enabled {
            if lang_en { "Standard" } else { "기본보기" }
        } else {
            if lang_en { "Details" } else { "상세보기" }
        };
        draw_hud_button(
            hdc,
            648,
            302,
            768,
            334,
            5,
            si_bg,
            si_border,
            si_text_color,
            si_label_str,
            fonts.key,
            37,
            0,
        );
    }
}

unsafe fn draw_pro_lock(hdc: isize, fonts: &HudFonts, lang_en: bool) {
    unsafe {
        let lock_icon = encode_wide("🔒");
        let old_font_icon = SelectObject(hdc, fonts.title);
        SetTextColor(hdc, 0x777777);
        let mut r_lock_icon = RECT {
            left: 638,
            top: 200,
            right: 778,
            bottom: 235,
        };
        DrawTextW(
            hdc,
            lock_icon.as_ptr(),
            lock_icon.len() as i32 - 1,
            &mut r_lock_icon,
            1 | 32,
        );
        SelectObject(hdc, old_font_icon);

        let lock_msg = if lang_en {
            encode_wide("PRO ONLY")
        } else {
            encode_wide("프로 전용")
        };
        let old_font_msg = SelectObject(hdc, fonts.key);
        SetTextColor(hdc, 0x888888);
        let mut r_lock_msg = RECT {
            left: 638,
            top: 240,
            right: 778,
            bottom: 270,
        };
        DrawTextW(
            hdc,
            lock_msg.as_ptr(),
            lock_msg.len() as i32 - 1,
            &mut r_lock_msg,
            1 | 32,
        );
        SelectObject(hdc, old_font_msg);
    }
}

unsafe fn draw_footer(hdc: isize, fonts: &HudFonts, lang_en: bool, labels: &HudLabels) {
    unsafe {
        // 4. Draw Info Footer & Warnings
        let old_font_txt = SelectObject(hdc, fonts.text);
        SetTextColor(hdc, 0x888888);

        let info1 = encode_wide(labels.info1_text);
        let mut r_info1 = RECT {
            left: 30,
            top: 366,
            right: 778,
            bottom: 384,
        };
        DrawTextW(hdc, info1.as_ptr(), info1.len() as i32 - 1, &mut r_info1, 0);

        let info2 = encode_wide(labels.info2_text);
        let mut r_info2 = RECT {
            left: 30,
            top: 385,
            right: 778,
            bottom: 403,
        };
        DrawTextW(hdc, info2.as_ptr(), info2.len() as i32 - 1, &mut r_info2, 0);

        let info_close = encode_wide(labels.info_close_text);
        let mut r_info_close = RECT {
            left: 30,
            top: 404,
            right: 778,
            bottom: 422,
        };
        DrawTextW(
            hdc,
            info_close.as_ptr(),
            info_close.len() as i32 - 1,
            &mut r_info_close,
            0,
        );
        let shortcut_info = encode_wide(labels.shortcut_info_text);
        let mut r_shortcut_info = RECT {
            left: 30,
            top: 425,
            right: 778,
            bottom: 443,
        };
        SetTextColor(hdc, 0x888888);
        DrawTextW(
            hdc,
            shortcut_info.as_ptr(),
            shortcut_info.len() as i32 - 1,
            &mut r_shortcut_info,
            0 | 32,
        );

        // Draw version text at the bottom-right corner
        let version_str = format!(
            "v{} · {}",
            env!("CARGO_PKG_VERSION"),
            if lang_en {
                "Info / Support"
            } else {
                "정보 / 지원"
            }
        );
        let version_w = encode_wide(&version_str);
        let mut r_version = RECT {
            left: 600,
            top: 404,
            right: 778,
            bottom: 422,
        };
        SetTextColor(hdc, 0xDDAA66); // Clickable support entry
        DrawTextW(
            hdc,
            version_w.as_ptr(),
            version_w.len() as i32 - 1,
            &mut r_version,
            2 | 32,
        );

        SelectObject(hdc, old_font_txt);
    }
}
