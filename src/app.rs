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

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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
            Self::apply_always_on_top(ctx, self.always_on_top);
        }

        if self.tiny_mode {
            self.render_tiny(ctx);
        } else {
            self.render_full(ctx);
        }
    }
}
