//! A separate, event-driven gesture. It never reads or changes CapsLock mode.
use std::sync::Mutex;
use std::time::{Duration, Instant};

const HOLD_TIME: Duration = Duration::from_millis(500);
static HOLD: Mutex<CapsHold> = Mutex::new(CapsHold { generation: 0, started: None, fired: false });

struct CapsHold {
    generation: u64,
    started: Option<Instant>,
    fired: bool,
}

impl CapsHold {
    fn down(&mut self, now: Instant) -> Option<u64> {
        if self.started.is_some() { return None; }
        self.generation = self.generation.wrapping_add(1);
        self.started = Some(now);
        self.fired = false;
        Some(self.generation)
    }

    fn up(&mut self) { self.started = None; }

    fn take_due(&mut self, generation: u64, now: Instant) -> bool {
        if self.generation != generation || self.fired
            || !self.started.is_some_and(|start| now.saturating_duration_since(start) >= HOLD_TIME) {
            return false;
        }
        self.fired = true;
        true
    }
}

pub fn observe(event: &crate::platform::KeyEvent) {
    if event.vk_code != 0x14 || event.is_injected_by_keysor { return; }
    let generation = {
        let Ok(mut hold) = HOLD.lock() else { return; };
        if event.is_keyup { hold.up(); return; }
        if !event.is_keydown { return; }
        hold.down(Instant::now())
    };
    if let Some(generation) = generation {
        // One timer for each real press, with cancellation by generation on release.
        // No keyboard-state polling and no extra CapsLock key injection.
        let _ = std::thread::Builder::new().name("keysor-caps-hold".into()).spawn(move || {
            std::thread::sleep(HOLD_TIME);
            let due = HOLD.lock().map(|mut hold| hold.take_due(generation, Instant::now())).unwrap_or(false);
            if due { crate::ui::win_gdi::request_open_hud(); }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn hold() -> CapsHold { CapsHold { generation: 0, started: None, fired: false } }

    #[test]
    fn fires_at_half_second_once_while_still_held() {
        let mut hold = hold();
        let now = Instant::now();
        let generation = hold.down(now).unwrap();
        assert!(!hold.take_due(generation, now + Duration::from_millis(499)));
        assert!(hold.take_due(generation, now + HOLD_TIME));
        assert!(!hold.take_due(generation, now + HOLD_TIME * 2));
    }

    #[test]
    fn release_and_repress_cancel_the_old_timer() {
        let mut hold = hold();
        let now = Instant::now();
        let old = hold.down(now).unwrap();
        hold.up();
        assert!(!hold.take_due(old, now + HOLD_TIME));
        let new = hold.down(now + Duration::from_millis(500)).unwrap();
        assert!(!hold.take_due(old, now + HOLD_TIME));
        assert!(!hold.take_due(new, now + HOLD_TIME));
        assert!(hold.take_due(new, now + HOLD_TIME + Duration::from_millis(500)));
    }

    #[test]
    fn repeat_does_not_restart_or_duplicate_timer() {
        let mut hold = hold();
        let now = Instant::now();
        let generation = hold.down(now).unwrap();
        assert!(hold.down(now + Duration::from_millis(250)).is_none());
        assert!(hold.take_due(generation, now + HOLD_TIME));
        hold.up();
        let next = hold.down(now + HOLD_TIME).unwrap();
        assert!(hold.take_due(next, now + HOLD_TIME * 2));
    }
}
