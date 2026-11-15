//! Visible native Keysor artwork, never a blank/transparent replacement cursor.
use std::sync::Mutex;
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use windows_sys::Win32::Graphics::Gdi::{CreateBitmap, DeleteObject};

struct Scheme { artwork: isize, remote_artwork: isize, applied: bool }
static SCHEME: Mutex<Scheme> = Mutex::new(Scheme { artwork: 0, remote_artwork: 0, applied: false });
static RESTORES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
pub fn restore_count() -> u64 { RESTORES.load(std::sync::atomic::Ordering::Relaxed) }

pub(crate) fn transparency_mask(pixels: &[u32], width: usize, height: usize) -> Vec<u8> {
    let stride = width.div_ceil(16) * 2;
    let mut mask = vec![0u8; stride * height];
    for y in 0..height {
        for x in 0..width {
            if pixels[y * width + x] >> 24 == 0 {
                mask[y * stride + x / 8] |= 0x80 >> (x % 8);
            }
        }
    }
    mask
}

const IDS: &[u32] = &[32512, 32513, 32514, 32515, 32516, 32642, 32643, 32644, 32645, 32646, 32648, 32649, 32650, 32651];

pub fn cache_artwork(bitmap: isize, width: i32, height: i32, bits: *mut std::ffi::c_void) {
    let Ok(mut state) = SCHEME.lock() else { return; };
    if bits.is_null() { return; }
    unsafe {
        let pixels = std::slice::from_raw_parts(bits as *const u32, (width * height) as usize);
        if pixels.iter().filter(|pixel| **pixel >> 24 > 128).count() < 10 { return; }
        let mask_bits = transparency_mask(pixels, width as usize, height as usize);
        let mask = CreateBitmap(width, height, 1, 1, mask_bits.as_ptr() as *const _);
        if mask == 0 { return; }
        let icon = ICONINFO { fIcon: 0, xHotspot: (width / 2) as u32, yHotspot: (height / 2) as u32, hbmMask: mask, hbmColor: bitmap };
        let artwork = CreateIconIndirect(&icon);
        DeleteObject(mask);
        let remote_artwork = crate::ui::win_gdi::render_native_cursor_32();
        if artwork == 0 || remote_artwork == 0 {
            if artwork != 0 { DestroyCursor(artwork); }
            if remote_artwork != 0 { DestroyCursor(remote_artwork); }
            return;
        }
        let refresh = state.applied;
        if state.artwork != 0 { DestroyCursor(state.artwork); }
        if state.remote_artwork != 0 { DestroyCursor(state.remote_artwork); }
        state.artwork = artwork;
        state.remote_artwork = remote_artwork;
        // Reapply changed frames even when mouse mode itself has not changed.
        state.applied = false;
        drop(state);
        if refresh { set_custom(true); }
    }
}

fn guardian_ready() -> bool {
    let path = crate::config::get_config_path().with_file_name(format!("watchdog-{}.json", std::process::id()));
    std::fs::read_to_string(path).ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .is_some_and(|v| v["parent_pid"].as_u64() == Some(std::process::id() as u64) && v["ready"] == true)
}

pub fn set_custom(custom: bool) {
    let Ok(mut state) = SCHEME.lock() else { return; };
    if state.applied == custom { return; }
    unsafe {
        if !custom {
            if SystemParametersInfoW(SPI_SETCURSORS, 0, std::ptr::null_mut(), 0) != 0 {
                state.applied = false;
                RESTORES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
            return;
        }
        let artwork = if crate::platform::windows::chrome_remote_mode() { state.remote_artwork } else { state.artwork };
        if artwork == 0 || !guardian_ready() { return; }
        for &id in IDS {
            // SetSystemCursor consumes the copy. Keep our owned master intact.
            let copy = CopyIcon(artwork);
            if copy == 0 || SetSystemCursor(copy, id) == 0 {
                SystemParametersInfoW(SPI_SETCURSORS, 0, std::ptr::null_mut(), 0);
                state.applied = false;
                return;
            }
        }
        state.applied = true;
        windows_sys::Win32::UI::Input::KeyboardAndMouse::mouse_event(
            windows_sys::Win32::UI::Input::KeyboardAndMouse::MOUSEEVENTF_MOVE, 0, 0, 0, 0);
    }
}

pub fn restore() -> bool {
    let Ok(mut state) = SCHEME.lock() else { return false; };
    let restored = unsafe { SystemParametersInfoW(SPI_SETCURSORS, 0, std::ptr::null_mut(), 0) != 0 };
    if restored { state.applied = false; RESTORES.fetch_add(1, std::sync::atomic::Ordering::Relaxed); }
    restored
}

pub fn status() -> (bool, bool) {
    SCHEME.lock().map(|s| (s.artwork != 0, s.applied)).unwrap_or((false, false))
}

#[cfg(test)]
pub(crate) fn verify_native_artwork_and_restore() {
    use std::time::Duration;
    let guardian = std::env::var_os("KEYSOR_GUARDIAN_TEST_PATH").expect("Independent guardian path for native cursor test");
    let report = crate::config::get_config_path().with_file_name(format!("watchdog-{}.json", std::process::id()));
    let _child = std::process::Command::new(guardian)
        .arg("--watchdog").arg(std::process::id().to_string()).arg(&report).spawn().unwrap();
    for _ in 0..100 { if guardian_ready() { break; } std::thread::sleep(Duration::from_millis(20)); }
    assert!(guardian_ready());
    assert!(status().0, "Visible Keysor cursor artwork was not created");
    let old_remote = crate::platform::windows::chrome_remote_mode();
    crate::platform::windows::set_chrome_remote_mode(true);
    set_custom(true);
    let applied = status().1;
    let dimensions = unsafe {
        let cursor = LoadCursorW(0, IDC_ARROW);
        let mut info: ICONINFO = std::mem::zeroed();
        if GetIconInfo(cursor, &mut info) != 0 {
            let mut bitmap: windows_sys::Win32::Graphics::Gdi::BITMAP = std::mem::zeroed();
            let ok = windows_sys::Win32::Graphics::Gdi::GetObjectW(info.hbmColor,
                std::mem::size_of_val(&bitmap) as i32, &mut bitmap as *mut _ as *mut _) != 0;
            let mut bmi: windows_sys::Win32::Graphics::Gdi::BITMAPINFO = std::mem::zeroed();
            bmi.bmiHeader.biSize = std::mem::size_of::<windows_sys::Win32::Graphics::Gdi::BITMAPINFOHEADER>() as u32;
            bmi.bmiHeader.biWidth = bitmap.bmWidth;
            bmi.bmiHeader.biHeight = -bitmap.bmHeight;
            bmi.bmiHeader.biPlanes = 1;
            bmi.bmiHeader.biBitCount = 32;
            let mut pixels = vec![0u32; (bitmap.bmWidth * bitmap.bmHeight).max(0) as usize];
            let dc = windows_sys::Win32::Graphics::Gdi::GetDC(0);
            let read = windows_sys::Win32::Graphics::Gdi::GetDIBits(dc, info.hbmColor, 0,
                bitmap.bmHeight as u32, pixels.as_mut_ptr() as *mut _, &mut bmi,
                windows_sys::Win32::Graphics::Gdi::DIB_RGB_COLORS);
            windows_sys::Win32::Graphics::Gdi::ReleaseDC(0, dc);
            let clipped = (0..32).any(|i| [i, 31 * 32 + i, i * 32, i * 32 + 31].into_iter().any(|index| pixels[index] >> 24 != 0));
            let hotspot_ok = info.xHotspot == 14 && info.yHotspot == 15;
            let smooth_edges = pixels.iter().filter(|pixel| { let alpha = **pixel >> 24; alpha > 0 && alpha < 255 }).count();
            DeleteObject(info.hbmColor);
            DeleteObject(info.hbmMask);
            (ok && read != 0 && !clipped && hotspot_ok).then_some((bitmap.bmWidth, bitmap.bmHeight, smooth_edges))
        } else { None }
    };
    // Restore before any assertion can unwind. The guardian also restores on process exit.
    let restored = restore();
    crate::platform::windows::set_chrome_remote_mode(old_remote);
    assert_eq!(dimensions.map(|(w, h, _)| (w, h)), Some((32, 32)), "Remote cursor must keep the historical 32px logical canvas");
    assert!(dimensions.is_some_and(|(_, _, edges)| edges > 10), "Final cursor must retain anti-aliased alpha edges");
    assert!(applied, "Could not apply visible Keysor system cursors");
    assert!(restored);
    assert!(!status().1);
}

#[cfg(test)]
mod mask_tests {
    #[test]
    fn transparency_mask_preserves_soft_edges_and_handles_row_padding() {
        let mut pixels = vec![0u32; 33 * 2];
        pixels[0] = 0xff00ff00;
        pixels[1] = 0x80008000;
        pixels[32] = 0xffffffff;
        let mask = super::transparency_mask(&pixels, 33, 2);
        assert_eq!(mask.len(), 12);
        assert_eq!(mask[0], 0x3f); // opaque and partially transparent edge remain visible
        assert_eq!(mask[4], 0); // opaque last pixel, padding unused
        assert_eq!(mask[6], 0xff); // the next row is fully transparent
        assert_eq!(mask[10], 0x80);
    }
}
