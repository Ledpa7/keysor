//! Rendering only; key gestures and window lifetime stay in the parent module.
use super::{HEIGHT, WIDTH, rect, wide};
use windows_sys::Win32::Foundation::{HWND, LRESULT, RECT};
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::UI::WindowsAndMessaging::GetClientRect;

unsafe fn text(dc: HDC, content: &str, bounds: RECT, size: i32, color: u32, bold: bool) {
    unsafe {
        let font = CreateFontW(
            -size,
            0,
            0,
            0,
            if bold { 600 } else { 400 },
            0,
            0,
            0,
            DEFAULT_CHARSET as u32,
            0,
            0,
            CLEARTYPE_QUALITY as u32,
            0,
            wide("Segoe UI").as_ptr(),
        );
        let old = SelectObject(dc, font);
        SetTextColor(dc, color);
        let mut bounds = bounds;
        let value = wide(content);
        DrawTextW(
            dc,
            value.as_ptr(),
            value.len() as i32 - 1,
            &mut bounds,
            DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
        );
        SelectObject(dc, old);
        DeleteObject(font);
    }
}

unsafe fn panel(dc: HDC, r: RECT, fill: u32, border: u32, radius: i32) {
    unsafe {
        let brush = CreateSolidBrush(fill);
        let pen = CreatePen(PS_SOLID, 1, border);
        let old_brush = SelectObject(dc, brush);
        let old_pen = SelectObject(dc, pen);
        RoundRect(dc, r.left, r.top, r.right, r.bottom, radius, radius);
        SelectObject(dc, old_brush);
        SelectObject(dc, old_pen);
        DeleteObject(brush);
        DeleteObject(pen);
    }
}

unsafe fn line(dc: HDC, x1: i32, y1: i32, x2: i32, y2: i32, color: u32, width: i32) {
    unsafe {
        let pen = CreatePen(PS_SOLID, width, color);
        let old = SelectObject(dc, pen);
        MoveToEx(dc, x1, y1, std::ptr::null_mut());
        LineTo(dc, x2, y2);
        SelectObject(dc, old);
        DeleteObject(pen);
    }
}

unsafe fn windows_icon(dc: HDC, x: i32, y: i32, size: i32) {
    unsafe {
        let brush = CreateSolidBrush(0xADFF2F);
        let cell = (size - 3) / 2;
        for row in 0..2 {
            for col in 0..2 {
                let left = x + col * (cell + 3);
                let top = y + row * (cell + 3);
                FillRect(dc, &rect(left, top, left + cell, top + cell), brush);
            }
        }
        DeleteObject(brush);
    }
}

unsafe fn ring(dc: HDC, r: RECT, color: u32, width: i32) {
    unsafe {
        let pen = CreatePen(PS_SOLID, width, color);
        let old_pen = SelectObject(dc, pen);
        let old_brush = SelectObject(dc, GetStockObject(HOLLOW_BRUSH));
        Ellipse(dc, r.left, r.top, r.right, r.bottom);
        SelectObject(dc, old_pen);
        SelectObject(dc, old_brush);
        DeleteObject(pen);
    }
}

fn local_clock() -> (String, String) {
    use windows_sys::Win32::Foundation::SYSTEMTIME;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetLocalTime(value: *mut SYSTEMTIME);
    }
    let mut now: SYSTEMTIME = unsafe { std::mem::zeroed() };
    unsafe {
        GetLocalTime(&mut now);
    }
    (
        format!("{:02}:{:02}", now.wHour, now.wMinute),
        format!("{:04}.{:02}.{:02}", now.wYear, now.wMonth, now.wDay),
    )
}

pub(super) unsafe fn paint(hwnd: HWND) -> LRESULT {
    unsafe {
        let mut ps = std::mem::zeroed();
        let target = BeginPaint(hwnd, &mut ps);
        let mut client = std::mem::zeroed();
        GetClientRect(hwnd, &mut client);
        let dc = CreateCompatibleDC(target);
        let bitmap = CreateCompatibleBitmap(target, client.right.max(1), client.bottom.max(1));
        let old_bitmap = SelectObject(dc, bitmap);
        SetMapMode(dc, MM_ANISOTROPIC);
        SetWindowExtEx(dc, WIDTH, HEIGHT, std::ptr::null_mut());
        SetViewportExtEx(dc, client.right, client.bottom, std::ptr::null_mut());
        let bg = CreateSolidBrush(0x121210);
        FillRect(dc, &rect(0, 0, WIDTH, HEIGHT), bg);
        DeleteObject(bg);
        SetBkMode(dc, TRANSPARENT as i32);
        panel(
            dc,
            rect(1, 1, WIDTH - 1, HEIGHT - 1),
            0x121210,
            0x526044,
            24,
        );
        draw_windows_key(dc);
        draw_taskbar(dc);
        draw_number_mapping(dc);
        SetMapMode(dc, MM_TEXT);
        BitBlt(target, 0, 0, client.right, client.bottom, dc, 0, 0, SRCCOPY);
        SelectObject(dc, old_bitmap);
        DeleteObject(bitmap);
        DeleteDC(dc);
        EndPaint(hwnd, &ps);
        0
    }
}

unsafe fn draw_windows_key(dc: HDC) {
    unsafe {
        // Language-free diagram: Windows + number keys map to taskbar positions.
        panel(dc, rect(30, 40, 86, 98), 0x242824, 0xADFF2F, 12);
        windows_icon(dc, 43, 53, 31);
        line(dc, 97, 69, 117, 69, 0x909990, 2);
        line(dc, 107, 59, 107, 79, 0x909990, 2);
    }
}

unsafe fn draw_taskbar(dc: HDC) {
    unsafe {
        // Full-width taskbar: Start/search, app group, tray and clock.
        panel(dc, rect(18, 150, 678, 212), 0x302F2B, 0x63715A, 2);
        line(dc, 19, 151, 677, 151, 0x91978A, 1);
        panel(dc, rect(24, 155, 66, 207), 0x363A32, 0x363A32, 2);
        windows_icon(dc, 32, 169, 25);
        ring(dc, rect(76, 168, 92, 184), 0xC8D0C5, 2);
        line(dc, 89, 181, 97, 189, 0xC8D0C5, 2);
        line(dc, 112, 158, 112, 204, 0x56604F, 1);
        line(dc, 530, 158, 530, 204, 0x56604F, 1);
        line(dc, 543, 183, 548, 178, 0xC8D0C5, 2);
        line(dc, 548, 178, 553, 183, 0xC8D0C5, 2);
        for n in 0..3 {
            line(dc, 571 + n * 5, 188, 571 + n * 5, 184 - n * 4, 0xC8D0C5, 2);
        }
        let (time, date) = local_clock();
        text(dc, &time, rect(599, 157, 668, 181), 16, 0xEFEFEF, false);
        text(dc, &date, rect(599, 182, 668, 203), 11, 0xDFE3DA, false);
        line(dc, 673, 157, 673, 205, 0x828A79, 1);
    }
}

unsafe fn draw_number_mapping(dc: HDC) {
    unsafe {
        for n in 0..9 {
            let x = 128 + n * 44;
            panel(dc, rect(x, 40, x + 38, 98), 0x242824, 0x687E58, 10);
            text(
                dc,
                &(n + 1).to_string(),
                rect(x, 40, x + 38, 98),
                25,
                0xADFF2F,
                true,
            );
            let center = x + 19;
            line(dc, center, 111, center, 129, 0x65765D, 2);
            line(dc, center - 5, 124, center, 129, 0x65765D, 2);
            line(dc, center + 5, 124, center, 129, 0x65765D, 2);
            if n == 0 {
                panel(dc, rect(x - 1, 155, x + 39, 207), 0x343D2F, 0x57694C, 7);
            }
            panel(
                dc,
                rect(center - 16, 165, center + 16, 194),
                0x41483A,
                0x839574,
                5,
            );
            text(
                dc,
                &(n + 1).to_string(),
                rect(center - 16, 165, center + 16, 194),
                20,
                0xADFF2F,
                true,
            );
            line(dc, center - 5, 202, center + 5, 202, 0xADFF2F, 2);
        }
    }
}
