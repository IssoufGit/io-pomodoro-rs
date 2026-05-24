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
    /// True while the window was miniaturized on the previous frame.
    /// Lets ui() detect the false→true→false transition on the main thread.
    #[cfg(target_os = "macos")]
    was_minimized: bool,
    /// Set when a minimize→restore transition is detected in ui().
    /// Cleared by raw_input_hook() which injects PointerGone before the
    /// next begin_frame() so egui starts with a clean pointer position.
    #[cfg(target_os = "macos")]
    needs_pointer_reset: bool,
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
            #[cfg(target_os = "macos")]
            was_minimized: false,
            #[cfg(target_os = "macos")]
            needs_pointer_reset: false,
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

    /// Runs before begin_frame(), so PointerGone here is actually processed by
    /// egui's input machinery. Adding events via input_mut() inside ui() has no
    /// effect on already-computed pointer state — this hook is the right place.
    #[cfg(target_os = "macos")]
    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        if self.needs_pointer_reset {
            self.needs_pointer_reset = false;
            raw_input.events.push(egui::Event::PointerGone);
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        // macOS: detect minimize→restore on every frame (main thread, no lock needed).
        //
        // winit's windowDidDeminiaturize: fires request_redraw() but omits
        // makeKeyAndOrderFront:. Without that call the window renders but is not
        // the key window, so Cocoa stops routing mouse events to it — buttons freeze.
        //
        // We poll isMiniaturized directly (via [NSApplication windows], because
        // [NSApplication mainWindow] returns nil while a window is miniaturized).
        // On the true→false transition we schedule makeKeyAndOrderFront: via
        // performSelectorOnMainThread:waitUntilDone:NO, which defers it to the
        // next run-loop turn (safe to call from within the render loop).
        #[cfg(target_os = "macos")]
        {
            let is_mini = crate::platform::window_is_minimized();
            if self.was_minimized && !is_mini {
                crate::platform::make_window_key_on_main_thread();
                self.needs_pointer_reset = true;
            }
            self.was_minimized = is_mini;
        }

        // First-frame macOS setup.
        if self.first_frame {
            self.first_frame = false;
            #[cfg(target_os = "macos")]
            crate::platform::apply_macos_transparency();
            #[cfg(target_os = "macos")]
            crate::platform::setup_macos_status_bar();

            // Background thread: keep the menu-bar status item ticking while
            // the window is minimised (ui() doesn't run during that time).
            #[cfg(target_os = "macos")]
            {
                let shared = std::sync::Arc::clone(&self.status_bar_state);
                let ctx_bg = ctx.clone();
                std::thread::spawn(move || {
                    loop {
                        std::thread::sleep(std::time::Duration::from_millis(500));
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
                        ctx_bg.request_repaint();
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
