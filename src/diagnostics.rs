use std::sync::atomic::{AtomicU64, Ordering};

static ACTIVATION_DOWN: AtomicU64 = AtomicU64::new(0);
static ACTIVATION_UP: AtomicU64 = AtomicU64::new(0);
static REMOTE_LOCK_SYNC: AtomicU64 = AtomicU64::new(0);

pub fn record_remote_lock_sync() {
    REMOTE_LOCK_SYNC.fetch_add(1, Ordering::Relaxed);
}

pub fn record_activation(down: bool, up: bool) {
    if down { ACTIVATION_DOWN.fetch_add(1, Ordering::Relaxed); }
    if up { ACTIVATION_UP.fetch_add(1, Ordering::Relaxed); }
}

pub fn start() {
    std::thread::spawn(|| {
        use std::io::Write;
        use windows_sys::Win32::UI::WindowsAndMessaging::{CURSORINFO, GetCursorInfo, GetForegroundWindow, IsWindowVisible};
        let path = crate::config::get_config_path().with_file_name(format!("input-diagnostic-{}.jsonl", std::process::id()));
        let Ok(mut file) = std::fs::File::create(path) else { return; };
        let _ = writeln!(file, "{}", serde_json::json!({"executable": std::env::current_exe().ok(), "pid": std::process::id(), "duration_seconds": 300}));
        let mut previous = String::new();
        // Bounded opt-in trace: no typed text, key stream, window titles or URLs.
        for tick in 0..3000 {
            if let Some(state) = crate::hook::APP_STATE.get() {
                if let Ok(state) = state.try_lock() {
                    let mut cursor: CURSORINFO = unsafe { std::mem::zeroed() };
                    cursor.cbSize = std::mem::size_of::<CURSORINFO>() as u32;
                    let cursor_ok = unsafe { GetCursorInfo(&mut cursor) != 0 };
                    let overlay_visible = crate::ui::win_gdi::INDICATOR_HWND.get()
                        .map(|&hwnd| unsafe { IsWindowVisible(hwnd) != 0 });
                    let foreground = unsafe { GetForegroundWindow() };
                    let (foreground_process, foreground_class, native_fallback) =
                        crate::ui::win_gdi::shell_cursor_diagnostic(foreground);
                    let value = serde_json::json!({
                        "activation_down": ACTIVATION_DOWN.load(Ordering::Relaxed),
                        "remote_lock_sync": REMOTE_LOCK_SYNC.load(Ordering::Relaxed),
                        "activation_up": ACTIVATION_UP.load(Ordering::Relaxed),
                        "mouse_mode": state.is_mouse_mode,
                        "activation_held": state.caps_lock_press_time.is_some(),
                        "ctrl": state.ctrl_pressed, "alt": state.alt_pressed, "win": state.win_pressed,
                        "cursor_query_ok": cursor_ok, "cursor_flags": cursor.flags,
                        "cursor_handle": cursor.hCursor, "overlay_visible": overlay_visible,
                        "foreground_handle": foreground,
                        "foreground_process": foreground_process,
                        "foreground_class": foreground_class,
                        "native_fallback": native_fallback,
                    });
                    drop(state);
                    let snapshot = value.to_string();
                    if snapshot != previous {
                        let _ = writeln!(file, "{}", serde_json::json!({"elapsed_ms":tick * 100, "state":value}));
                        let _ = file.flush();
                        previous = snapshot;
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    });
}
