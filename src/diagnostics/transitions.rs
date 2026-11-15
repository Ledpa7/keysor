// Diagnostic only: bounded capture of Caps transitions and at most three input
// categories immediately after mode OFF. Never enqueue characters or key codes.
use std::sync::{
    Mutex, OnceLock,
    mpsc::{SyncSender, sync_channel},
};
use std::time::{Duration, Instant};
static TRANSITIONS: OnceLock<SyncSender<Transition>> = OnceLock::new();
static TRACE_UNTIL: OnceLock<Instant> = OnceLock::new();
static TRACE_WINDOW: Mutex<TraceWindow> = Mutex::new(TraceWindow {
    off_at: None,
    inputs: 0,
});
#[derive(Default)]
struct TraceWindow {
    off_at: Option<Instant>,
    inputs: u8,
}
impl TraceWindow {
    fn select(
        &mut self,
        vk: u32,
        down: bool,
        up: bool,
        mode: bool,
        now: Instant,
    ) -> Option<&'static str> {
        if vk == 0x14 {
            if down {
                self.off_at = (!mode).then_some(now);
                self.inputs = 0;
                return Some("caps_down");
            }
            if up {
                return Some("caps_up");
            }
        }
        if !down
            || !self
                .off_at
                .is_some_and(|at| now.saturating_duration_since(at) <= Duration::from_secs(2))
        {
            return None;
        }
        if vk == 0x15 {
            return Some("ime_toggle");
        }
        if self.inputs >= 3 {
            return None;
        }
        let kind = match vk {
            0x41..=0x5A => "letter_input",
            0xE7 => "unicode_input",
            _ => return None,
        };
        self.inputs += 1;
        Some(kind)
    }
}
struct Transition {
    at: Instant,
    value: serde_json::Value,
}

pub fn record_transition(
    vk: u32,
    down: bool,
    up: bool,
    own: bool,
    injected: bool,
    remote: bool,
    passed: bool,
) {
    let Some(sender) = TRANSITIONS.get() else {
        return;
    };
    let now = Instant::now();
    if own || TRACE_UNTIL.get().is_none_or(|until| now >= *until) {
        return;
    }
    let Some(state) = crate::hook::APP_STATE.get().and_then(|s| s.try_lock().ok()) else {
        return;
    };
    let Ok(mut window) = TRACE_WINDOW.try_lock() else {
        return;
    };
    let Some(kind) = window.select(vk, down, up, state.is_mouse_mode, now) else {
        return;
    };
    let value = serde_json::json!({"kind":kind,"passed":passed,"mouse_mode":state.is_mouse_mode,
        "ctrl":state.ctrl_pressed,"alt":state.alt_pressed,"win":state.win_pressed,"shift":state.shift_pressed,
        "source":if remote {"chrome_remote"} else if injected {"injected"} else {"physical"},
        "since_mode_off_ms":window.off_at.map(|at| now.saturating_duration_since(at).as_millis())});
    // Never wait for the diagnostics worker on the low-level keyboard hook.
    let _ = sender.try_send(Transition { at: now, value });
}

fn ime_snapshot() -> serde_json::Value {
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetKeyboardLayout;
    use windows_sys::Win32::UI::WindowsAndMessaging::*;
    #[link(name = "imm32")]
    unsafe extern "system" {
        fn ImmGetDefaultIMEWnd(hwnd: HWND) -> HWND;
    }
    unsafe {
        let foreground = GetForegroundWindow();
        let mut pid = 0;
        let thread = GetWindowThreadProcessId(foreground, &mut pid);
        let mut gui: GUITHREADINFO = std::mem::zeroed();
        gui.cbSize = std::mem::size_of::<GUITHREADINFO>() as u32;
        let focus = if thread != 0 && GetGUIThreadInfo(thread, &mut gui) != 0 && gui.hwndFocus != 0
        {
            gui.hwndFocus
        } else {
            foreground
        };
        let ime = if focus != 0 {
            ImmGetDefaultIMEWnd(focus)
        } else {
            0
        };
        let query = |command| -> Option<usize> {
            if ime == 0 {
                return None;
            }
            let mut value = 0;
            (SendMessageTimeoutW(
                ime,
                0x0283,
                command,
                0,
                SMTO_ABORTIFHUNG | SMTO_ERRORONEXIT,
                10,
                &mut value,
            ) != 0)
                .then_some(value)
        };
        let open = query(0x0005); // IMC_GETOPENSTATUS (IMM compatibility view)
        let conversion = query(0x0001); // IMC_GETCONVERSIONMODE
        let layout = if thread != 0 {
            Some(GetKeyboardLayout(thread) as usize)
        } else {
            None
        };
        let (process, class, _) = crate::ui::win_gdi::shell_cursor_diagnostic(foreground);
        serde_json::json!({"foreground_pid":pid,"foreground_handle":foreground,"focus_handle":focus,
            "foreground_process":process,"foreground_class":class,"keyboard_layout":layout,
            "ime_window_present":ime != 0,"imm_open":open.map(|v| v != 0),"imm_conversion":conversion,
            "imm_native_mode":conversion.map(|v| v & 1 != 0),
            "foreground_changed_during_sample":GetForegroundWindow() != foreground})
    }
}

pub(super) fn start() {
    let started = Instant::now();
    let until = started + Duration::from_secs(1800);
    if TRACE_UNTIL.set(until).is_err() {
        return;
    }
    let (sender, receiver) = sync_channel::<Transition>(32);
    if TRANSITIONS.set(sender).is_err() {
        return;
    }
    std::thread::spawn(move || {
        use std::io::Write;
        let path = crate::config::get_config_path()
            .with_file_name(format!("input-transition-{}.jsonl", std::process::id()));
        let Ok(mut file) = std::fs::File::create(path) else {
            return;
        };
        let _ = writeln!(
            file,
            "{}",
            serde_json::json!({"pid":std::process::id(),"build":env!("KEYSOR_BUILD_ID"),
            "duration_seconds":1800,"max_records":2000,"typed_text_recorded":false,"key_codes_recorded":false,
            "ime_view":"IMM compatibility state; may not represent TSF composition in every app"})
        );
        let mut count = 0;
        while Instant::now() < until && count < 2000 {
            let Ok(mut entry) = receiver.recv_timeout(Duration::from_secs(1)) else {
                continue;
            };
            let delay = entry.at.elapsed().as_millis();
            entry.value["elapsed_ms"] =
                serde_json::json!(entry.at.saturating_duration_since(started).as_millis());
            entry.value["sample_delay_ms"] = serde_json::json!(delay);
            entry.value["ime_snapshot"] = if delay <= 100 {
                ime_snapshot()
            } else {
                serde_json::Value::Null
            };
            if writeln!(file, "{}", entry.value).is_err() {
                break;
            }
            let _ = file.flush();
            count += 1;
        }
    });
}

#[cfg(test)]
mod transition_tests {
    use super::*;
    #[test]
    #[ignore = "Reads foreground IME compatibility state on an interactive Windows desktop; no keys or clipboard changes"]
    fn native_ime_query_preserves_focus_and_has_bounded_wait() {
        use windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
        let foreground = unsafe { GetForegroundWindow() };
        let started = Instant::now();
        let snapshot = ime_snapshot();
        assert!(started.elapsed() < Duration::from_secs(1));
        assert_eq!(unsafe { GetForegroundWindow() }, foreground);
        assert!(snapshot.get("imm_open").is_some());
        assert!(snapshot.get("keyboard_layout").is_some());
        assert!(snapshot.get("typed_text").is_none());
        assert!(snapshot.get("key_code").is_none());
    }
    #[test]
    fn trace_only_samples_first_three_inputs_in_two_second_off_window() {
        let mut w = TraceWindow::default();
        let now = Instant::now();
        assert!(w.select(0x41, true, false, false, now).is_none());
        assert_eq!(w.select(0x14, true, false, false, now), Some("caps_down"));
        assert_eq!(
            w.select(0x41, true, false, false, now),
            Some("letter_input")
        );
        assert!(w.select(0x41, false, true, false, now).is_none());
        assert_eq!(w.select(0x15, true, false, false, now), Some("ime_toggle"));
        assert_eq!(
            w.select(0xE7, true, false, false, now),
            Some("unicode_input")
        );
        assert_eq!(
            w.select(0x5A, true, false, false, now),
            Some("letter_input")
        );
        assert!(w.select(0x42, true, false, false, now).is_none());
        w.select(0x14, true, false, false, now);
        assert!(
            w.select(0x41, true, false, false, now + Duration::from_millis(2001))
                .is_none()
        );
        w.select(0x14, true, false, true, now);
        assert!(w.select(0x41, true, false, true, now).is_none());
    }
}
