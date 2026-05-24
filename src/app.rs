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
    /// Last Instant when ui() ran with the window focused. Only updated while
    /// focused so a large gap at the next focused frame = just came back from
    /// minimize / Dock. Used to trigger the restore-focus fix on macOS.
    last_focused_time: std::time::Instant,
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
            last_focused_time: std::time::Instant::now(),
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

        // Detect window restore from minimize / Dock on macOS.
        //
        // winit's windowDidDeminiaturize: calls request_redraw() but does NOT
        // call makeKeyAndOrderFront:, so the window renders yet Cocoa doesn't
        // route mouse events to it — buttons appear live but do nothing.
        //
        // We track `last_focused_time` (updated only while focused). A gap
        // > 600 ms when focus returns means we just came back from the Dock or
        // a long alt-tab. We then:
        //   1. Re-assert key-window status (deferred to avoid Metal re-entrancy).
        //   2. Inject PointerGone to flush any stale egui button-down state
        //      left over from the Dock click that triggered the restore.
        let focused = ctx.input(|i| i.viewport().focused.unwrap_or(true));
        if focused {
            let gap = self.last_focused_time.elapsed();
            self.last_focused_time = std::time::Instant::now();
            if gap > std::time::Duration::from_millis(600) {
                #[cfg(target_os = "macos")]
                crate::platform::request_window_focus();
                ctx.input_mut(|i| i.events.push(egui::Event::PointerGone));
            }
        }

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
                let ctx_bg = ctx.clone();
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
                    // On macOS, eframe stops calling ui() while the window is in the
                    // Dock. Calling request_repaint() from this thread wakes the event
                    // loop so the first frame after un-minimise is processed immediately
                    // and buttons are responsive again.
                    ctx_bg.request_repaint();
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

        // Always schedule the next repaint. 200 ms when running (smooth countdown),
        // 500 ms when idle — just fast enough to process input after a window
        // restore/un-minimize without burning CPU while the timer is paused.
        ctx.request_repaint_after(std::time::Duration::from_millis(
            if self.running { 200 } else { 500 },
        ));



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
