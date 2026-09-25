// PomodoroApp: struct definition, construction, and the eframe update loop.
// Timer logic lives in timer.rs; UI rendering lives in ui/.

use crate::model::{Mode, Session, Settings, FULL_MIN_SIZE, TINY_MIN_SIZE};
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
    /// Bidirectional timer state shared with the background status-bar
    /// thread, which is the only code that keeps running while the window
    /// is minimized/occluded (eframe skips `App::ui()` entirely then). The
    /// background thread applies menu-bar commands and natural session
    /// completion directly to this when the window isn't around to do it;
    /// `ui()` adopts those changes into `self` once it runs again.
    #[cfg(target_os = "macos")]
    shared_timer: std::sync::Arc<std::sync::Mutex<SharedTimer>>,
    /// Set when a restore transition is detected; cleared by raw_input_hook()
    /// which injects PointerGone before begin_frame().
    #[cfg(target_os = "macos")]
    needs_pointer_reset: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl PomodoroApp {
    pub fn new() -> Self {
        let persisted = load_state().unwrap_or_default();
        let total = persisted.settings.focus_min * 60;
        #[cfg(target_os = "macos")]
        let shared_timer = std::sync::Arc::new(std::sync::Mutex::new(SharedTimer::new(
            &persisted.settings,
            persisted.sessions.clone(),
            Mode::Focus,
            total,
        )));
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
            shared_timer,
            #[cfg(target_os = "macos")]
            needs_pointer_reset: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }
}

// ── SharedTimer (macOS only) ────────────────────────────────────────────────
//
// Owns the timer fields the background thread needs to apply menu-bar
// commands and detect natural session completion on its own, independent of
// whether `App::ui()` is currently running.

#[cfg(target_os = "macos")]
struct SharedTimer {
    mode: Mode,
    focus_min: u32,
    short_min: u32,
    long_min: u32,
    total_seconds: u32,
    seconds_left: u32,
    running: bool,
    tick_started_at: Option<Instant>,
    sessions: Vec<Session>,
    /// Set by the background thread after it applies a command; cleared by
    /// `ui()` once it has adopted these values into `self`.
    dirty: bool,
}

#[cfg(target_os = "macos")]
impl SharedTimer {
    fn new(settings: &Settings, sessions: Vec<Session>, mode: Mode, total_seconds: u32) -> Self {
        Self {
            mode,
            focus_min: settings.focus_min,
            short_min: settings.short_min,
            long_min: settings.long_min,
            total_seconds,
            seconds_left: total_seconds,
            running: false,
            tick_started_at: None,
            sessions,
            dirty: false,
        }
    }

    /// Seconds remaining right now, accounting for elapsed time since
    /// `tick_started_at` while running.
    fn current_seconds(&self) -> u32 {
        if self.running {
            let elapsed = self
                .tick_started_at
                .map(|t| t.elapsed().as_secs() as u32)
                .unwrap_or(0);
            self.seconds_left.saturating_sub(elapsed)
        } else {
            self.seconds_left
        }
    }

    fn apply_command(&mut self, cmd: crate::platform::MacMenuCommand) {
        use crate::platform::MacMenuCommand;
        match cmd {
            MacMenuCommand::TogglePause => {
                if self.running {
                    self.seconds_left = self.current_seconds();
                    self.running = false;
                    self.tick_started_at = None;
                } else {
                    self.running = true;
                    self.tick_started_at = Some(Instant::now());
                }
            }
            MacMenuCommand::Reset => {
                self.running = false;
                self.tick_started_at = None;
                self.seconds_left = self.total_seconds;
            }
            MacMenuCommand::StartNewFocus => {
                self.mode = Mode::Focus;
                self.total_seconds = self.focus_min * 60;
                self.seconds_left = self.total_seconds;
                self.running = true;
                self.tick_started_at = Some(Instant::now());
            }
            MacMenuCommand::SetFocusDuration(min) => {
                self.focus_min = min;
                if !self.running && self.mode == Mode::Focus {
                    self.total_seconds = min * 60;
                    self.seconds_left = self.total_seconds;
                }
            }
        }
        self.dirty = true;
    }

    /// If the countdown has reached zero while running, finalizes the
    /// session (mirrors `PomodoroApp::complete_session(true)`) and resets
    /// for the next run. Returns true if it did so (caller should persist).
    fn finalize_if_complete(&mut self) -> bool {
        if !self.running || self.current_seconds() > 0 {
            return false;
        }
        let duration_minutes = (self.total_seconds as f64 / 60.0 * 10.0).round() / 10.0;
        self.sessions.insert(
            0,
            Session {
                id: format!("s-{}", chrono::Utc::now().timestamp_millis()),
                mode: self.mode,
                duration_minutes,
                completed_at: chrono::Utc::now(),
                completed: true,
            },
        );
        self.running = false;
        self.tick_started_at = None;
        self.seconds_left = self.total_seconds;
        self.dirty = true;
        true
    }

    fn persisted_state(&self) -> crate::model::PersistedState {
        crate::model::PersistedState {
            sessions: self.sessions.clone(),
            settings: Settings {
                focus_min: self.focus_min,
                short_min: self.short_min,
                long_min: self.long_min,
            },
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

impl PomodoroApp {
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
    /// Runs before begin_frame() so PointerGone is actually processed by egui.
    #[cfg(target_os = "macos")]
    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        if self.needs_pointer_reset.swap(false, std::sync::atomic::Ordering::Relaxed) {
            raw_input.events.push(egui::Event::PointerGone);
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        if self.first_frame {
            self.first_frame = false;
            #[cfg(target_os = "macos")]
            crate::platform::setup_macos_status_bar();
            #[cfg(target_os = "macos")]
            crate::platform::setup_macos_status_menu(self.settings.focus_min);

            #[cfg(target_os = "macos")]
            {
                let shared = std::sync::Arc::clone(&self.shared_timer);
                let ctx_bg = ctx.clone();
                let needs_reset = std::sync::Arc::clone(&self.needs_pointer_reset);
                std::thread::spawn(move || {
                    let mut was_mini = false;
                    loop {
                        std::thread::sleep(std::time::Duration::from_millis(300));

                        // ── Menu commands, natural completion, status bar + menu ──
                        // The only place these are applied while the window is
                        // minimized/occluded, since App::ui() doesn't run then.
                        if let Ok(mut s) = shared.lock() {
                            let mut needs_persist = false;
                            for cmd in crate::platform::poll_macos_menu_commands() {
                                if matches!(cmd, crate::platform::MacMenuCommand::SetFocusDuration(_)) {
                                    needs_persist = true;
                                }
                                s.apply_command(cmd);
                            }
                            if s.finalize_if_complete() {
                                needs_persist = true;
                            }
                            if needs_persist {
                                let _ = crate::persistence::save_state(&s.persisted_state());
                            }

                            let current = s.current_seconds();
                            let mins = current / 60;
                            let indicator = if s.running { " ▶" } else { "" };
                            crate::platform::update_macos_status_bar(
                                &format!("{} {} min{}", s.mode.emoji(), mins, indicator)
                            );
                            crate::platform::refresh_macos_menu(s.running, s.focus_min);
                        }

                        // ── Restore detection ─────────────────────────────────
                        // egui-winit never updates viewport_info.minimized at
                        // runtime on macOS (deadlock prevention). After a Dock
                        // restore only Occluded(false) fires, leaving
                        // info.minimized = Some(true) permanently, which keeps
                        // is_visible = false so ui() never runs again.
                        // Sending Minimized(false) resets that flag; Focus then
                        // calls activateIgnoringOtherApps + makeKeyAndOrderFront.
                        let is_mini = crate::platform::window_is_minimized();
                        if was_mini && !is_mini {
                            ctx_bg.send_viewport_cmd_to(
                                egui::ViewportId::ROOT,
                                egui::ViewportCommand::Minimized(false),
                            );
                            ctx_bg.send_viewport_cmd_to(
                                egui::ViewportId::ROOT,
                                egui::ViewportCommand::Focus,
                            );
                            needs_reset.store(true, std::sync::atomic::Ordering::Relaxed);
                        }
                        was_mini = is_mini;

                        ctx_bg.request_repaint();
                    }
                });
            }
        }

        // Adopt any state the background thread applied (menu-bar commands or
        // a natural session completion) while the window was minimized/occluded
        // and this frame's `ui()` wasn't running to handle them itself.
        #[cfg(target_os = "macos")]
        {
            let mut adopted = false;
            if let Ok(mut shared) = self.shared_timer.lock() {
                if shared.dirty {
                    self.mode = shared.mode;
                    self.settings.focus_min = shared.focus_min;
                    self.total_seconds = shared.total_seconds;
                    self.seconds_left = shared.seconds_left;
                    self.running = shared.running;
                    self.last_tick = shared.tick_started_at;
                    self.sessions = shared.sessions.clone();
                    shared.dirty = false;
                    adopted = true;
                }
            }
            if adopted {
                self.status = if self.running {
                    match self.mode {
                        crate::model::Mode::Focus => "Focusing",
                        _ => "On break",
                    }
                    .into()
                } else {
                    "Ready".into()
                };
                self.persist();
            }
        }

        self.tick();

        // Push fresh state to the background thread and update the status
        // bar text immediately, for instant feedback while the window is
        // visible (the background thread also does this every ~300ms).
        #[cfg(target_os = "macos")]
        {
            let mins = self.seconds_left / 60;
            let indicator = if self.running { " ▶" } else { "" };
            crate::platform::update_macos_status_bar(
                &format!("{} {} min{}", self.mode.emoji(), mins, indicator)
            );
            crate::platform::refresh_macos_menu(self.running, self.settings.focus_min);
            if let Ok(mut shared) = self.shared_timer.lock() {
                shared.mode = self.mode;
                shared.focus_min = self.settings.focus_min;
                shared.short_min = self.settings.short_min;
                shared.long_min = self.settings.long_min;
                shared.total_seconds = self.total_seconds;
                shared.seconds_left = self.seconds_left;
                shared.running = self.running;
                shared.tick_started_at = self.last_tick;
                shared.sessions = self.sessions.clone();
            }
        }

        ctx.request_repaint_after(std::time::Duration::from_millis(
            if self.running { 200 } else { 500 },
        ));

        if let Some(size) = self.pending_resize.take() {
            // Min size first, otherwise the full-mode minimum clamps the strip.
            let min = if self.tiny_mode { TINY_MIN_SIZE } else { FULL_MIN_SIZE };
            ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(
                egui::vec2(min[0], min[1]),
            ));
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(
                egui::vec2(size[0], size[1]),
            ));
            crate::platform::set_rounded_corners(self.tiny_mode);
        }

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
