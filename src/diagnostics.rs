//! Opt-in diagnostics; cursor snapshots and input transitions have separate workers.
mod transitions;
pub use transitions::record_transition;

use std::sync::atomic::{AtomicU64, Ordering};

static ACTIVATION_DOWN: AtomicU64 = AtomicU64::new(0);
static ACTIVATION_UP: AtomicU64 = AtomicU64::new(0);
static REMOTE_LOCK_SYNC: AtomicU64 = AtomicU64::new(0);

pub fn record_remote_lock_sync() {
    REMOTE_LOCK_SYNC.fetch_add(1, Ordering::Relaxed);
}

pub fn record_activation(down: bool, up: bool) {
    if down {
        ACTIVATION_DOWN.fetch_add(1, Ordering::Relaxed);
    }
    if up {
        ACTIVATION_UP.fetch_add(1, Ordering::Relaxed);
    }
}

pub fn start() {
    transitions::start();
    std::thread::spawn(run_cursor_trace);
}

fn run_cursor_trace() {
    use std::io::Write;
    let path = crate::config::get_config_path()
        .with_file_name(format!("input-diagnostic-{}.jsonl", std::process::id()));
    let Ok(mut file) = std::fs::File::create(path) else {
        return;
    };
    let _ = writeln!(
        file,
        "{}",
        serde_json::json!({"executable": std::env::current_exe().ok(), "pid": std::process::id(), "duration_seconds": 300})
    );
    let mut previous = String::new();
    // Bounded opt-in trace: no typed text, key stream, window titles or URLs.
    for tick in 0..3000 {
        if let Some(value) = capture_cursor_snapshot() {
            let snapshot = value.to_string();
            if snapshot != previous {
                let _ = writeln!(
                    file,
                    "{}",
                    serde_json::json!({"elapsed_ms":tick * 100, "state":value})
                );
                let _ = file.flush();
                previous = snapshot;
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

#[derive(Clone, Copy)]
struct InputSnapshot {
    mouse_mode: bool,
    activation_held: bool,
    ctrl: bool,
    alt: bool,
    win: bool,
}

impl InputSnapshot {
    fn from_state(state: &crate::hook::AppState) -> Self {
        Self {
            mouse_mode: state.is_mouse_mode,
            activation_held: state.caps_lock_press_time.is_some(),
            ctrl: state.ctrl_pressed,
            alt: state.alt_pressed,
            win: state.win_pressed,
        }
    }
}

fn snapshot_input(shared: &std::sync::Mutex<crate::hook::AppState>) -> Option<InputSnapshot> {
    let state = shared.try_lock().ok()?;
    Some(InputSnapshot::from_state(&state))
}

fn capture_cursor_snapshot() -> Option<serde_json::Value> {
    let input = snapshot_input(crate::hook::APP_STATE.get()?)?;
    // The shared input lock is already released before any Windows/JSON work.
    Some(collect_cursor_snapshot(input))
}

fn collect_cursor_snapshot(input: InputSnapshot) -> serde_json::Value {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CURSORINFO, GetCursorInfo, GetForegroundWindow, IsWindowVisible,
    };
    let mut cursor: CURSORINFO = unsafe { std::mem::zeroed() };
    cursor.cbSize = std::mem::size_of::<CURSORINFO>() as u32;
    let cursor_ok = unsafe { GetCursorInfo(&mut cursor) != 0 };
    let overlay_visible = crate::ui::win_gdi::INDICATOR_HWND
        .get()
        .map(|&hwnd| unsafe { IsWindowVisible(hwnd) != 0 });
    let foreground = unsafe { GetForegroundWindow() };
    let (foreground_process, foreground_class, native_fallback) =
        crate::ui::win_gdi::shell_cursor_diagnostic(foreground);
    let value = serde_json::json!({
        "activation_down": ACTIVATION_DOWN.load(Ordering::Relaxed),
        "remote_lock_sync": REMOTE_LOCK_SYNC.load(Ordering::Relaxed),
        "activation_up": ACTIVATION_UP.load(Ordering::Relaxed),
        "mouse_mode": input.mouse_mode,
        "native_keysor_artwork_ready": crate::cursor_scheme::status().0,
        "native_keysor_applied": crate::cursor_scheme::status().1,
        "native_cursor_restore_count": crate::cursor_scheme::restore_count(),
        "caps_lock_on": crate::platform::get_system_controller().is_caps_lock_on(),
        "activation_held": input.activation_held,
        "ctrl": input.ctrl, "alt": input.alt, "win": input.win,
        "cursor_query_ok": cursor_ok, "cursor_flags": cursor.flags,
        "cursor_handle": cursor.hCursor, "overlay_visible": overlay_visible,
        "foreground_handle": foreground,
        "foreground_process": foreground_process,
        "foreground_class": foreground_class,
        "native_fallback": native_fallback,
    });
    value
}

#[cfg(test)]
pub(crate) fn benchmark_input_snapshot(shared: &std::sync::Mutex<crate::hook::AppState>) {
    use std::time::Instant;
    let mut old_lock = Vec::new();
    let mut new_lock = Vec::new();
    let mut old_total = Vec::new();
    let mut new_total = Vec::new();
    for _ in 0..100 {
        let start = Instant::now();
        let guard = shared.lock().unwrap();
        let original = collect_cursor_snapshot(InputSnapshot::from_state(&guard));
        drop(guard);
        old_lock.push(start.elapsed().as_nanos() as u64);
        old_total.push(start.elapsed().as_nanos() as u64);
        let start = Instant::now();
        let input = snapshot_input(shared).unwrap();
        new_lock.push(start.elapsed().as_nanos() as u64);
        assert!(
            shared.try_lock().is_ok(),
            "Windows collection must run outside the input lock"
        );
        let optimized = collect_cursor_snapshot(input);
        new_total.push(start.elapsed().as_nanos() as u64);
        for key in ["mouse_mode", "activation_held", "ctrl", "alt", "win"] {
            assert_eq!(original[key], optimized[key]);
        }
    }
    fn median(values: &mut [u64]) -> u64 {
        values.sort_unstable();
        values[values.len() / 2]
    }
    let report = serde_json::json!({"samples":100,"unit":"nanoseconds",
        "before_lock_median":median(&mut old_lock),"after_lock_median":median(&mut new_lock),
        "before_total_median":median(&mut old_total),"after_total_median":median(&mut new_total)});
    std::fs::write(
        "target/priority-perf-input-benchmark.json",
        report.to_string(),
    )
    .unwrap();
}
