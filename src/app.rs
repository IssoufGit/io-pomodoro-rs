// PomodoroApp: struct definition, construction, and the eframe update loop.
// Timer logic lives in timer.rs; UI rendering lives in ui/.

use crate::model::{Mode, Session, Settings};
use crate::persistence::load_state;
use eframe::egui;
use std::time::Instant;

// ── Struct ────────────────────────────────────────────────────────────────────

pub struct PomodoroApp {
    // Session state
    pub(crate) mode: Mode,
    pub(crate) settings: Settings,
    pub(crate) sessions: Vec<Session>,
    // Timer state
    pub(crate) total_seconds: u32,
    pub(crate) seconds_left: u32,
    pub(crate) running: bool,
    pub(crate) last_tick: Option<Instant>,
    pub(crate) status: String,
    // UI state
    pub(crate) confirm_clear: bool,
    pub(crate) tiny_mode: bool,
    pub(crate) pending_resize: Option<[f32; 2]>,
    pub(crate) always_on_top: bool,
    first_frame: bool,
    /// Shared state for the background status bar ticker:
    /// (seconds_left, running, mode_emoji, captured_at)
    /// The thread re-computes the current value using elapsed time so it
    /// stays accurate even when the window is minimised.
    #[cfg(target_os = "macos")]
    status_bar_state: std::sync::Arc<std::sync::Mutex<(u32, bool, &'static str, std::time::Instant)>>,
}

impl PomodoroApp {
    pub fn new() -> Self {
        let persisted = load_state().unwrap_or_default();
        let total = persisted.settings.focus_min * 60;
        Self {
            mode: Mode::Focus,
            settings: persisted.settings,
            sessions: persisted.sessions,
            total_seconds: total,
            seconds_left: total,
            running: false,
            last_tick: None,
            status: "Ready".into(),
            confirm_clear: false,
            tiny_mode: false,
            pending_resize: None,
            always_on_top: false,
            first_frame: true,
            #[cfg(target_os = "macos")]
            status_bar_state: std::sync::Arc::new(std::sync::Mutex::new(
                (0u32, false, "🍅", std::time::Instant::now())
            )),
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

impl PomodoroApp {
    /// Apply always-on-top via wmctrl (X11) and egui's ViewportCommand.
    /// Both are tried; whichever the compositor honours wins.
    pub fn apply_always_on_top(ctx: &egui::Context, enable: bool) {
        crate::platform::set_always_on_top("Pomodoro", enable);
        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
            if enable {
                egui::WindowLevel::AlwaysOnTop
            } else {
                egui::WindowLevel::Normal
            },
        ));
    }
}

// ── eframe::App ───────────────────────────────────────────────────────────────

impl eframe::App for PomodoroApp {
    /// Fully transparent GPU clear so the OS compositor can show through.
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        // First-frame macOS setup.
        if self.first_frame {
            self.first_frame = false;
            #[cfg(target_os = "macos")]
            crate::platform::apply_macos_transparency();
            #[cfg(target_os = "macos")]
            crate::platform::setup_macos_status_bar();
            // Background thread: keeps the status bar current while minimized.
            // It re-computes the remaining minutes from the captured state +
            // elapsed time, so the display stays accurate without calling tick().
            #[cfg(target_os = "macos")]
            {
                let shared = std::sync::Arc::clone(&self.status_bar_state);
                std::thread::spawn(move || loop {
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    if let Ok(s) = shared.lock() {
                        let (secs, running, emoji, captured_at) = &*s;
                        let current = if *running {
                            secs.saturating_sub(captured_at.elapsed().as_secs() as u32)
                        } else {
                            *secs
                        };
                        let mins = current / 60;
                        let indicator = if *running { " ▶" } else { "" };
                        crate::platform::update_macos_status_bar(
                            &format!("{} {} min{}", emoji, mins, indicator)
                        );
                    }
                });
            }
        }

        // Update macOS menu bar each frame and refresh the shared state so
        // the background thread can extrapolate accurately while minimised.
        #[cfg(target_os = "macos")]
        {
            let emoji: &'static str = match self.mode {
                crate::model::Mode::Focus      => "🍅",
                crate::model::Mode::ShortBreak => "☕",
                crate::model::Mode::LongBreak  => "🌙",
            };
            let mins = self.seconds_left / 60;
            let indicator = if self.running { " ▶" } else { "" };
            crate::platform::update_macos_status_bar(
                &format!("{} {} min{}", emoji, mins, indicator)
            );
            // Snapshot current state + timestamp for the background thread.
            if let Ok(mut s) = self.status_bar_state.lock() {
                *s = (self.seconds_left, self.running, emoji, std::time::Instant::now());
            }
        }

        self.tick();

        // Keep repainting while the timer is counting down.
        if self.running {
            ctx.request_repaint_after(std::time::Duration::from_millis(200));
        }



        // Deferred viewport resize requested by toggle_tiny().
        if let Some(size) = self.pending_resize.take() {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(
                egui::vec2(size[0], size[1]),
            ));
        }

        // Keyboard shortcuts
        let (space, r_key, t_key, a_key) = ctx.input(|i| {
            (
                i.key_pressed(egui::Key::Space),
                i.key_pressed(egui::Key::R),
                i.key_pressed(egui::Key::T),
                i.key_pressed(egui::Key::A),
            )
        });
        if space { self.start_or_pause(); }
        if r_key  { self.reset(); }
        if t_key  { self.toggle_tiny(); }
        if a_key && crate::platform::always_on_top_supported() {
            self.always_on_top = !self.always_on_top;
            Self::apply_always_on_top(&ctx, self.always_on_top);
        }

        if self.tiny_mode {
            self.render_tiny(&ctx);
        } else {
            self.render_full(&ctx);
        }
    }
}
