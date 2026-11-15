//! Passive Windows shortcut discovery. Observes keys without blocking/injecting input.
mod renderer;
use std::collections::HashSet;
use std::sync::{
    Mutex,
    atomic::{AtomicIsize, Ordering},
};
use std::time::{Duration, Instant};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const HOLD_TIME: Duration = Duration::from_secs(1);
const REFRESH: u32 = 0x8005;
const HOLD_TIMER: usize = 0x5747;
const DISMISS_TIMER: usize = 0x5748;
const CLOCK_TIMER: usize = 0x5749;
const WIDTH: i32 = 696;
const HEIGHT: i32 = 240;
static GESTURE: Mutex<Gesture> = Mutex::new(Gesture {
    down: None,
    started: None,
    fired: false,
});
static WINDOW: AtomicIsize = AtomicIsize::new(0);

#[derive(Default)]
struct Gesture {
    down: Option<HashSet<u32>>,
    started: Option<Instant>,
    fired: bool,
}

fn is_win(vk: u32) -> bool {
    matches!(vk, 0x5B | 0x5C)
}

impl Gesture {
    fn event(&mut self, vk: u32, pressed: bool, now: Instant) -> bool {
        let down = self.down.get_or_insert_with(HashSet::new);
        if pressed {
            if !down.insert(vk) {
                return false;
            } // autorepeat never restarts the timer
            if is_win(vk) {
                if down.len() == 1 {
                    self.started = Some(now);
                    self.fired = false;
                }
            } else if self.started.take().is_some() {
                self.fired = false;
                return true; // dismiss before Windows handles the actual shortcut
            }
        } else {
            down.remove(&vk);
            if is_win(vk) && !down.iter().any(|vk| is_win(*vk)) {
                self.started = None;
                self.fired = false;
                return true;
            }
        }
        is_win(vk)
    }

    fn remaining(&self, now: Instant) -> Option<Duration> {
        self.started
            .filter(|_| !self.fired)
            .map(|start| HOLD_TIME.saturating_sub(now.saturating_duration_since(start)))
    }

    fn take_due(&mut self, now: Instant, physical_win_down: bool) -> bool {
        if !physical_win_down {
            self.started = None;
            self.fired = false;
            return false;
        }
        if self.remaining(now) != Some(Duration::ZERO) {
            return false;
        }
        self.fired = true;
        true
    }
}

pub fn observe(event: &crate::platform::KeyEvent) {
    if event.is_injected_by_keysor || (!event.is_keydown && !event.is_keyup) {
        return;
    }
    let changed = GESTURE
        .lock()
        .map(|mut g| g.event(event.vk_code, event.is_keydown, Instant::now()))
        .unwrap_or(false);
    if changed {
        if let Some(&main) = crate::ui::win_gdi::MAIN_HWND.get() {
            unsafe {
                PostMessageW(main, REFRESH, 0, 0);
            }
        }
    }
}

// All window/timer work stays on the existing UI thread. No key-state polling.
pub fn handle_message(main: HWND, msg: u32, w: WPARAM) -> Option<LRESULT> {
    if msg != REFRESH && !(msg == WM_TIMER && w == HOLD_TIMER) {
        return None;
    }
    unsafe {
        KillTimer(main, HOLD_TIMER);
    }
    let Ok(mut g) = GESTURE.lock() else {
        return Some(0);
    };
    if msg == WM_TIMER {
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
        // One physical check at the deadline protects against a lost release event.
        let physical = unsafe { GetAsyncKeyState(0x5B) < 0 || GetAsyncKeyState(0x5C) < 0 };
        if g.take_due(Instant::now(), physical) {
            show();
        }
    }
    if let Some(remaining) = g.remaining(Instant::now()) {
        unsafe {
            SetTimer(
                main,
                HOLD_TIMER,
                remaining.as_millis().clamp(1, 1000) as u32,
                None,
            );
        }
    }
    if g.started.is_none() {
        hide();
    }
    Some(0)
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

fn hide() {
    let hwnd = WINDOW.load(Ordering::SeqCst);
    if hwnd != 0 {
        unsafe {
            KillTimer(hwnd, DISMISS_TIMER);
            KillTimer(hwnd, CLOCK_TIMER);
            ShowWindow(hwnd, SW_HIDE);
        }
    }
}

fn placement(work: RECT, dpi: u32) -> (i32, i32, i32, i32) {
    let scale = (dpi as f64 / 96.0)
        .min((work.right - work.left - 32).max(1) as f64 / WIDTH as f64)
        .min((work.bottom - work.top - 32).max(1) as f64 / HEIGHT as f64);
    let w = (WIDTH as f64 * scale).round().max(1.0) as i32;
    let h = (HEIGHT as f64 * scale).round().max(1.0) as i32;
    (
        work.left + (work.right - work.left - w) / 2,
        work.top + (work.bottom - work.top - h) / 2,
        w,
        h,
    )
}

fn show() {
    unsafe {
        let mut hwnd = WINDOW.load(Ordering::SeqCst);
        if hwnd == 0 {
            let name = wide("KeysorShortcutHint");
            let instance =
                windows_sys::Win32::System::LibraryLoader::GetModuleHandleW(std::ptr::null());
            let class = WNDCLASSW {
                lpfnWndProc: Some(window_proc),
                hInstance: instance,
                lpszClassName: name.as_ptr(),
                ..std::mem::zeroed()
            };
            RegisterClassW(&class);
            hwnd = CreateWindowExW(
                WS_EX_LAYERED
                    | WS_EX_TOPMOST
                    | WS_EX_TOOLWINDOW
                    | WS_EX_NOACTIVATE
                    | WS_EX_TRANSPARENT,
                name.as_ptr(),
                wide("Keysor · Windows shortcuts").as_ptr(),
                WS_POPUP,
                // Independent tool window: the main taskbar window may be minimized.
                0,
                0,
                1,
                1,
                0,
                0,
                instance,
                std::ptr::null(),
            );
            if hwnd == 0 {
                return;
            }
            WINDOW.store(hwnd, Ordering::SeqCst);
            SetLayeredWindowAttributes(hwnd, 0, 230, LWA_ALPHA);
        }
        let foreground = GetForegroundWindow();
        let monitor = MonitorFromWindow(foreground, MONITOR_DEFAULTTONEAREST);
        let mut info: MONITORINFO = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        if GetMonitorInfoW(monitor, &mut info) == 0 {
            return;
        }
        #[link(name = "user32")]
        unsafe extern "system" {
            fn GetDpiForWindow(hwnd: HWND) -> u32;
        }
        let dpi = GetDpiForWindow(foreground).max(96);
        let (x, y, w, h) = placement(info.rcWork, dpi);
        let radius = (24 * w / WIDTH).max(8);
        let region = CreateRoundRectRgn(0, 0, w + 1, h + 1, radius, radius);
        if region != 0 && SetWindowRgn(hwnd, region, 0) == 0 {
            DeleteObject(region);
        }
        SetWindowPos(hwnd, HWND_TOPMOST, x, y, w, h, SWP_NOACTIVATE);
        InvalidateRect(hwnd, std::ptr::null(), 0);
        ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        UpdateWindow(hwnd);
        // Bounded lifetime if a release is lost during lock/remote disconnect.
        SetTimer(hwnd, DISMISS_TIMER, 8000, None);
        SetTimer(hwnd, CLOCK_TIMER, 1000, None);
    }
}

fn rect(left: i32, top: i32, right: i32, bottom: i32) -> RECT {
    RECT {
        left,
        top,
        right,
        bottom,
    }
}

unsafe extern "system" fn window_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        match msg {
            WM_NCHITTEST => HTTRANSPARENT as LRESULT,
            WM_MOUSEACTIVATE => MA_NOACTIVATE as LRESULT,
            WM_ERASEBKGND => 1,
            WM_TIMER if w == DISMISS_TIMER => {
                hide();
                0
            }
            WM_TIMER if w == CLOCK_TIMER => {
                InvalidateRect(hwnd, std::ptr::null(), 0);
                0
            }
            WM_PAINT => renderer::paint(hwnd),
            WM_DESTROY => {
                WINDOW.store(0, Ordering::SeqCst);
                0
            }
            _ => DefWindowProcW(hwnd, msg, w, l),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn short_press_repeat_and_repress_do_not_open_early() {
        let mut g = Gesture::default();
        let now = Instant::now();
        g.event(0x5B, true, now);
        assert!(!g.event(0x5B, true, now + Duration::from_millis(500)));
        assert!(!g.take_due(now + Duration::from_millis(999), true));
        g.event(0x5B, false, now + Duration::from_millis(999));
        assert!(!g.take_due(now + HOLD_TIME, true));
        g.event(0x5C, true, now + HOLD_TIME);
        assert!(!g.take_due(now + HOLD_TIME, true));
        assert!(g.take_due(now + HOLD_TIME * 2, true));
        assert!(!g.take_due(now + HOLD_TIME * 3, true));
    }
    #[test]
    fn shortcuts_cancel_before_or_after_show_and_require_fresh_win_press() {
        let now = Instant::now();
        for key in [0x31, 0x39, 0x44, 0x10, 0x11, 0x1B] {
            for shown in [false, true] {
                let mut g = Gesture::default();
                g.event(0x5B, true, now);
                if shown {
                    assert!(g.take_due(now + HOLD_TIME, true));
                }
                assert!(g.event(key, true, now + HOLD_TIME));
                assert!(g.started.is_none());
                g.event(key, false, now + HOLD_TIME);
                assert!(!g.take_due(now + HOLD_TIME * 2, true));
            }
        }
    }
    #[test]
    fn modifiers_already_held_and_lost_release_never_show() {
        let now = Instant::now();
        let mut g = Gesture::default();
        g.event(0x11, true, now);
        g.event(0x5B, true, now);
        g.event(0x11, false, now);
        assert!(!g.take_due(now + HOLD_TIME, true));
        g.event(0x5B, false, now);
        g.event(0x5B, true, now);
        assert!(!g.take_due(now + HOLD_TIME, false));
        assert!(g.started.is_none());
    }
    #[test]
    fn releasing_one_win_side_keeps_other_side_held() {
        let now = Instant::now();
        let mut g = Gesture::default();
        g.event(0x5B, true, now);
        g.event(0x5C, true, now);
        g.event(0x5B, false, now);
        assert!(g.take_due(now + HOLD_TIME, true));
        g.event(0x5C, false, now);
        assert!(g.started.is_none());
    }
    #[test]
    fn panel_is_centered_and_fits_scaled_and_negative_origin_monitors() {
        for work in [
            rect(0, 0, 1920, 1040),
            rect(-2560, 0, 0, 1400),
            rect(0, 0, 640, 440),
        ] {
            for dpi in [96, 144, 192] {
                let (x, y, w, h) = placement(work, dpi);
                assert!(
                    x >= work.left && y >= work.top && x + w <= work.right && y + h <= work.bottom
                );
                assert!((x * 2 + w - work.left - work.right).abs() <= 1);
                assert!((y * 2 + h - work.top - work.bottom).abs() <= 1);
            }
        }
    }

    #[test]
    #[ignore = "Requires an interactive Windows desktop; shows a passive hint, no keyboard hook or clipboard changes"]
    fn native_shortcut_hint_is_passive_and_dismisses_on_release() {
        unsafe {
            let foreground = GetForegroundWindow();
            show();
            let hwnd = WINDOW.load(Ordering::SeqCst);
            assert_ne!(hwnd, 0);
            assert_ne!(IsWindowVisible(hwnd), 0);
            assert_eq!(
                GetForegroundWindow(),
                foreground,
                "Hint took keyboard focus"
            );
            let styles = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
            for flag in [
                WS_EX_NOACTIVATE,
                WS_EX_TRANSPARENT,
                WS_EX_TOOLWINDOW,
                WS_EX_LAYERED,
            ] {
                assert_ne!(styles & flag, 0);
            }
            assert_eq!(
                SendMessageW(hwnd, WM_NCHITTEST, 0, 0),
                HTTRANSPARENT as isize
            );
            let mut bounds: RECT = std::mem::zeroed();
            GetWindowRect(hwnd, &mut bounds);
            let mut info: MONITORINFO = std::mem::zeroed();
            info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
            assert_ne!(
                GetMonitorInfoW(
                    MonitorFromWindow(foreground, MONITOR_DEFAULTTONEAREST),
                    &mut info
                ),
                0
            );
            assert!((bounds.left + bounds.right - info.rcWork.left - info.rcWork.right).abs() <= 1);
            assert!((bounds.top + bounds.bottom - info.rcWork.top - info.rcWork.bottom).abs() <= 1);
            std::thread::sleep(Duration::from_millis(200));
            crate::ui::test_capture::save_window_bitmap(hwnd, "target/win-shortcuts-preview.bmp");
            let now = Instant::now();
            {
                let mut g = GESTURE.lock().unwrap();
                g.event(0x5B, true, now);
                g.event(0x5B, false, now);
            }
            handle_message(0, REFRESH, 0);
            assert_eq!(IsWindowVisible(hwnd), 0, "Release did not hide the hint");
            show();
            SendMessageW(hwnd, WM_TIMER, DISMISS_TIMER, 0);
            assert_eq!(
                IsWindowVisible(hwnd),
                0,
                "Lost-release safety timeout did not hide hint"
            );
            DestroyWindow(hwnd);
        }
    }
}
