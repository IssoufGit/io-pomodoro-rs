// Timer and session logic: ticking, starting/pausing, completing sessions,
// and persisting state. No rendering here.

use crate::app::PomodoroApp;
use crate::model::{Mode, PersistedState, Session};
use crate::persistence::save_state;
use chrono::{Local, Utc};
use std::time::Instant;

impl PomodoroApp {
    pub fn duration_for(&self, mode: Mode) -> u32 {
        match mode {
            Mode::Focus => self.settings.focus_min,
            Mode::ShortBreak => self.settings.short_min,
            Mode::LongBreak => self.settings.long_min,
        }
    }

    pub fn set_mode(&mut self, m: Mode) {
        self.running = false;
        self.last_tick = None;
        self.mode = m;
        self.total_seconds = self.duration_for(m) * 60;
        self.seconds_left = self.total_seconds;
        self.status = "Ready".into();
    }

    pub fn start_or_pause(&mut self) {
        if self.running {
            self.running = false;
            self.last_tick = None;
            self.status = "Paused".into();
        } else {
            self.running = true;
            self.last_tick = Some(Instant::now());
            self.status = match self.mode {
                Mode::Focus => "Focusing".into(),
                _ => "On break".into(),
            };
        }
    }

    pub fn reset(&mut self) {
        self.running = false;
        self.last_tick = None;
        self.seconds_left = self.total_seconds;
        self.status = "Ready".into();
    }

    /// Called every frame while running. Advances the countdown by whole seconds
    /// and triggers session completion when the timer reaches zero.
    pub fn tick(&mut self) {
        if !self.running { return; }
        let Some(last) = self.last_tick else { return };
        let elapsed = last.elapsed().as_secs();
        if elapsed >= 1 {
            self.last_tick = Some(Instant::now());
            let to_subtract = elapsed.min(self.seconds_left as u64) as u32;
            self.seconds_left = self.seconds_left.saturating_sub(to_subtract);
            if self.seconds_left == 0 {
                self.complete_session(true);
            }
        }
    }

    /// Finalises the current session and appends it to history.
    /// `natural` = true means the timer ran to zero on its own.
    pub fn complete_session(&mut self, natural: bool) {
        self.running = false;
        self.last_tick = None;

        let elapsed = self.total_seconds - self.seconds_left;
        let duration_minutes = if natural {
            self.total_seconds as f64 / 60.0
        } else {
            elapsed as f64 / 60.0
        };

        if duration_minutes < 1.0 && !natural {
            self.status = "Too short to save".into();
            self.seconds_left = self.total_seconds;
            return;
        }

        let session = Session {
            id: format!("s-{}", Utc::now().timestamp_millis()),
            mode: self.mode,
            duration_minutes: (duration_minutes * 10.0).round() / 10.0,
            completed_at: Utc::now(),
            completed: natural,
        };
        self.sessions.insert(0, session);
        let _ = save_state(&PersistedState {
            sessions: self.sessions.clone(),
            settings: self.settings.clone(),
        });
        self.status = if natural { "Complete" } else { "Saved" }.into();
        self.seconds_left = self.total_seconds;
    }

    pub fn persist(&self) {
        let _ = save_state(&PersistedState {
            sessions: self.sessions.clone(),
            settings: self.settings.clone(),
        });
    }

    pub fn clear_history(&mut self) {
        self.sessions.clear();
        self.persist();
    }

    /// Returns (today_focus_count, today_focus_minutes, all_time_focus_count).
    pub fn today_stats(&self) -> (usize, f64, usize) {
        let today = Local::now().date_naive();
        let focus_total: Vec<&Session> = self.sessions
            .iter()
            .filter(|s| s.mode == Mode::Focus)
            .collect();
        let today_focus: Vec<&&Session> = focus_total
            .iter()
            .filter(|s| s.completed_at.with_timezone(&Local).date_naive() == today)
            .collect();
        let minutes: f64 = today_focus.iter().map(|s| s.duration_minutes).sum();
        (today_focus.len(), minutes, focus_total.len())
    }

    /// Called after any settings field changes. Refreshes the countdown display
    /// if the timer is not running, then persists.
    pub fn apply_settings_change(&mut self) {
        if !self.running {
            self.set_mode(self.mode);
        }
        self.persist();
    }
}
