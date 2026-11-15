//! Shared bitmap capture for native rendering regression tests only.
use windows_sys::Win32::{
    Foundation::{HWND, RECT},
    Graphics::Gdi::*,
    UI::WindowsAndMessaging::GetClientRect,
};

pub(crate) unsafe fn save_window_bitmap(hwnd: HWND, path: &str) {
    unsafe {
        let mut bounds: RECT = std::mem::zeroed();
        GetClientRect(hwnd, &mut bounds);
        let width = bounds.right;
        let height = bounds.bottom;
        let dc = GetDC(hwnd);
        let copy = CreateCompatibleDC(dc);
        let mut bits = std::ptr::null_mut();
        let bitmap_info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: 40,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB,
                ..std::mem::zeroed()
            },
            ..std::mem::zeroed()
        };
        let bitmap = CreateDIBSection(dc, &bitmap_info, DIB_RGB_COLORS, &mut bits, 0, 0);
        assert_ne!(bitmap, 0);
        assert!(!bits.is_null());
        let previous = SelectObject(copy, bitmap);
        assert_ne!(BitBlt(copy, 0, 0, width, height, dc, 0, 0, SRCCOPY), 0);
        GdiFlush();
        let pixels = std::slice::from_raw_parts(bits as *const u8, (width * height * 4) as usize);
        let mut file = Vec::new();
        file.extend_from_slice(b"BM");
        file.extend_from_slice(&(54 + pixels.len() as u32).to_le_bytes());
        file.extend_from_slice(&[0; 4]);
        file.extend_from_slice(&54u32.to_le_bytes());
        file.extend_from_slice(&40u32.to_le_bytes());
        file.extend_from_slice(&width.to_le_bytes());
        file.extend_from_slice(&(-height).to_le_bytes());
        file.extend_from_slice(&1u16.to_le_bytes());
        file.extend_from_slice(&32u16.to_le_bytes());
        file.extend_from_slice(&[0; 24]);
        file.extend_from_slice(pixels);
        std::fs::write(path, file).unwrap();
        SelectObject(copy, previous);
        DeleteObject(bitmap);
        DeleteDC(copy);
        ReleaseDC(hwnd, dc);
    }
}
