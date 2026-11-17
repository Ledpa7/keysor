//! GDI/GDI+ primitives and font ownership for the main HUD.
use super::encode_wide;
use windows_sys::Win32::Foundation::RECT;
use windows_sys::Win32::Graphics::{Gdi::*, GdiPlus::*};

pub(super) fn draw_key_cap(
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
            1 => 0xADFF2F, // Caps Lock: Keysor mint (#2FFFAD, BGR)
            2 => 0x40E000, // WASD: Grass Green
            3 => 0xFFFF00, // QERF: 네온 파랑 (Cyan)
            4 => 0x3300FF, // Spacebar: former Caps Lock neon red (#FF0033, BGR)
            5 => 0x00FFFF, // G Key: 네온 노랑 (Yellow)
            6 => 0xFFFFFF, // Shift: 흰색
            7 => 0xADFF2F, // Windows shortcut discovery: Keysor mint
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
        if desc_text.is_empty() {
            let mut r_key = RECT {
                left: x,
                top: y,
                right: x + w,
                bottom: y + h,
            };
            DrawTextW(
                hdc,
                key_w.as_ptr(),
                key_w.len() as i32 - 1,
                &mut r_key,
                1 | 4 | 32,
            ); // DT_CENTER | DT_VCENTER | DT_SINGLELINE
        } else {
            let mut r_key = RECT {
                left: x,
                top: y + 4,
                right: x + w,
                bottom: y + 21,
            };
            DrawTextW(
                hdc,
                key_w.as_ptr(),
                key_w.len() as i32 - 1,
                &mut r_key,
                1 | 32,
            );

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
            let is_multiline = desc_text.contains('\n');
            let top_offset = if is_multiline { 18 } else { 23 }; // 2줄 설명(Shift 등)만 18, 1줄 설명(J, K, WASD 등)은 23으로 완벽 통일
            let bottom_offset = if is_multiline { 2 } else { 4 };
            let mut r_desc = RECT {
                left: x + 2,
                top: y + top_offset,
                right: x + w - 2,
                bottom: y + h - bottom_offset,
            };
            let align = if is_multiline { 1 } else { 1 | 32 };
            DrawTextW(
                hdc,
                desc_w.as_ptr(),
                desc_w.len() as i32 - 1,
                &mut r_desc,
                align,
            );

            if desc_font != 0 {
                SelectObject(hdc, old_desc_font);
                DeleteObject(desc_font);
            }
        }

        SelectObject(hdc, old_font);
        SelectObject(hdc, old_brush);
        SelectObject(hdc, old_pen);

        DeleteObject(brush);
        DeleteObject(pen);
    }
}

pub(super) struct HudFonts {
    pub(super) title: windows_sys::Win32::Graphics::Gdi::HFONT,
    pub(super) number: windows_sys::Win32::Graphics::Gdi::HFONT,
    pub(super) key: windows_sys::Win32::Graphics::Gdi::HFONT,
    pub(super) text: windows_sys::Win32::Graphics::Gdi::HFONT,
}

impl HudFonts {
    pub(super) unsafe fn create(font_name_ptr: *const u16) -> Self {
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

pub(super) unsafe fn draw_hud_button(
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
            DrawTextW(
                hdc,
                wide_text.as_ptr(),
                wide_text.len() as i32 - 1,
                &mut r_dark,
                align_flags,
            );

            // 2. 네온 라임 그린 메인 레이어
            SetTextColor(hdc, 0xADFF2F);
            let mut r_neon = RECT {
                left: x1,
                top: y1 + text_top_offset,
                right: x2,
                bottom: y2,
            };
            DrawTextW(
                hdc,
                wide_text.as_ptr(),
                wide_text.len() as i32 - 1,
                &mut r_neon,
                align_flags,
            );
        } else {
            SetTextColor(hdc, text_color);
            let wide_text = encode_wide(text);
            let mut r_text = RECT {
                left: x1,
                top: y1 + text_top_offset,
                right: x2,
                bottom: y2,
            };
            DrawTextW(
                hdc,
                wide_text.as_ptr(),
                wide_text.len() as i32 - 1,
                &mut r_text,
                align_flags,
            );
        }

        SelectObject(hdc, old_font);
    }
}

pub(super) unsafe fn draw_hud_toggle_row(
    hdc: windows_sys::Win32::Graphics::Gdi::HDC,
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    radius: i32,
    is_active: bool,
    is_hovered: bool,
    is_enabled: bool,
    label: &str,
    font: windows_sys::Win32::Graphics::Gdi::HFONT,
) {
    unsafe {
        if !is_enabled {
            let brush = CreateSolidBrush(0x121414);
            let old_brush = SelectObject(hdc, brush);
            let pen = CreatePen(0, 1, 0x222222);
            let old_pen = SelectObject(hdc, pen);
            RoundRect(hdc, x1, y1, x2, y2, radius, radius);
            SelectObject(hdc, old_pen);
            DeleteObject(pen);
            SelectObject(hdc, old_brush);
            DeleteObject(brush);
            return;
        }

        // 1. 버튼 배경 컨테이너 패널
        let bg_color = if is_hovered { 0x2A2E2E } else { 0x202424 };
        let brush = CreateSolidBrush(bg_color);
        let old_brush = SelectObject(hdc, brush);

        let border_color = if is_hovered {
            0xADFF2F
        } else if is_active {
            0x6EAA20
        } else {
            0x3C4040
        };

        if border_color == 0xADFF2F {
            let dark_pen = CreatePen(0, 2, 0x004D20);
            let old_pen = SelectObject(hdc, dark_pen);
            RoundRect(hdc, x1 + 1, y1 + 1, x2 + 1, y2 + 1, radius, radius);
            SelectObject(hdc, old_pen);
            DeleteObject(dark_pen);

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

        // 2. 좌측 텍스트 라벨 (좌측 정렬, 수직 중앙)
        let old_font = SelectObject(hdc, font);
        let text_color = if is_active {
            0xADFF2F
        } else if is_hovered {
            0xFFFFFF
        } else {
            0xCCCCCC
        };

        let wide_text = encode_wide(label);
        if text_color == 0xADFF2F {
            SetTextColor(hdc, 0x004D20);
            let mut r_dark = RECT {
                left: x1 + 10,
                top: y1 + 1,
                right: x2 - 44,
                bottom: y2 + 1,
            };
            DrawTextW(
                hdc,
                wide_text.as_ptr(),
                wide_text.len() as i32 - 1,
                &mut r_dark,
                0 | 4 | 32,
            );

            SetTextColor(hdc, 0xADFF2F);
            let mut r_neon = RECT {
                left: x1 + 10,
                top: y1,
                right: x2 - 44,
                bottom: y2,
            };
            DrawTextW(
                hdc,
                wide_text.as_ptr(),
                wide_text.len() as i32 - 1,
                &mut r_neon,
                0 | 4 | 32,
            );
        } else {
            SetTextColor(hdc, text_color);
            let mut r_text = RECT {
                left: x1 + 10,
                top: y1,
                right: x2 - 44,
                bottom: y2,
            };
            DrawTextW(
                hdc,
                wide_text.as_ptr(),
                wide_text.len() as i32 - 1,
                &mut r_text,
                0 | 4 | 32,
            );
        }
        SelectObject(hdc, old_font);

        // 3. 우측 고해상도 안티앨리어싱(GDI+) 토글 스위치 렌더링
        let mut graphics = std::ptr::null_mut();
        if GdipCreateFromHDC(hdc, &mut graphics) == 0 {
            GdipSetSmoothingMode(graphics, SmoothingModeAntiAlias);

            let sw_w: f32 = 32.0;
            let sw_h: f32 = 18.0;
            let sw_x: f32 = (x2 as f32) - sw_w - 7.0;
            let sw_y: f32 = (y1 as f32) + ((y2 - y1) as f32 - sw_h) / 2.0;

            let mut path = std::ptr::null_mut();
            if GdipCreatePath(0, &mut path) == 0 {
                let d = sw_h;
                // 왼쪽 반원 (90도부터 180도)
                GdipAddPathArc(path, sw_x, sw_y, d, d, 90.0, 180.0);
                // 오른쪽 반원 (270도부터 180도)
                GdipAddPathArc(path, sw_x + sw_w - d, sw_y, d, d, 270.0, 180.0);
                GdipClosePathFigure(path);

                // 트랙 배경 채우기
                let track_argb: u32 = if is_active {
                    0xFF2FFFAD // 네온 라임/그린
                } else {
                    0xFF32363A // 세련된 다크 트랙
                };
                let mut track_brush = std::ptr::null_mut();
                if GdipCreateSolidFill(track_argb, &mut track_brush) == 0 {
                    GdipFillPath(graphics, track_brush, path);
                    GdipDeleteBrush(track_brush);
                }

                // OFF 상태일 때 부드러운 외곽선 추가
                if !is_active {
                    let mut track_pen = std::ptr::null_mut();
                    if GdipCreatePen1(0xFF4C5258, 1.0, 0, &mut track_pen) == 0 {
                        GdipDrawPath(graphics, track_pen, path);
                        GdipDeletePen(track_pen);
                    }
                }

                GdipDeletePath(path);
            }

            // 4. 슬라이딩 원형 노브 (Knob)
            let knob_d: f32 = sw_h - 4.0; // 14.0px
            let knob_x: f32 = if is_active {
                sw_x + sw_w - knob_d - 2.0
            } else {
                sw_x + 2.0
            };
            let knob_y: f32 = sw_y + 2.0;

            // 노브 부드러운 드롭 섀도우
            let mut shadow_brush = std::ptr::null_mut();
            if GdipCreateSolidFill(0x38000000, &mut shadow_brush) == 0 {
                GdipFillEllipse(graphics, shadow_brush, knob_x, knob_y + 0.8, knob_d, knob_d);
                GdipDeleteBrush(shadow_brush);
            }

            // 노브 본체 (순백색)
            let mut knob_brush = std::ptr::null_mut();
            if GdipCreateSolidFill(0xFFFFFFFF, &mut knob_brush) == 0 {
                GdipFillEllipse(graphics, knob_brush, knob_x, knob_y, knob_d, knob_d);
                GdipDeleteBrush(knob_brush);
            }

            // 노브 섬세한 테두리
            let mut knob_pen = std::ptr::null_mut();
            if GdipCreatePen1(0x20000000, 0.8, 0, &mut knob_pen) == 0 {
                GdipDrawEllipse(graphics, knob_pen, knob_x, knob_y, knob_d, knob_d);
                GdipDeletePen(knob_pen);
            }

            GdipDeleteGraphics(graphics);
        }
    }
}
