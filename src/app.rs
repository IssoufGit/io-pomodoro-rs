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
    #[cfg(target_os = "macos")]
    status_bar_state: std::sync::Arc<std::sync::Mutex<(u32, bool, &'static str, std::time::Instant)>>,
    /// True while the window was miniaturized on the previous ui() frame.
    #[cfg(target_os = "macos")]
    was_minimized: bool,
    /// Set when a restore transition is detected; cleared by raw_input_hook()
    /// which injects PointerGone before begin_frame().
    #[cfg(target_os = "macos")]
    needs_pointer_reset: std::sync::Arc<std::sync::atomic::AtomicBool>,
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
            needs_pointer_reset: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
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
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    /// Runs before begin_frame() so PointerGone is actually processed by egui.
    #[cfg(target_os = "macos")]
    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        if self.needs_pointer_reset.swap(false, std::sync::atomic::Ordering::Relaxed) {
            raw_input.events.push(egui::Event::PointerGone);
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        // macOS: detect minimize→restore every frame.
        //
        // Root cause: NSStatusItem creates a backing NSStatusBarWindow that
        // appears in [NSApplication windows].  AppKit then reports
        // hasVisibleWindows=YES and skips the automatic makeKeyAndOrderFront:
        // on Dock-icon clicks.  The window appears but is not the key window,
        // so Cocoa stops routing mouse events to it — buttons freeze.
        //
        // Fix: send ViewportCommand::Focus on each restore transition.
        // eframe routes this through winit's focus_window() which calls
        // activateIgnoringOtherApps + makeKeyAndOrderFront on the main thread.
        // Dual detection: main-thread (immediate, same frame as restore) and
        // background thread (catches the race where frames stop before
        // isMiniaturized flips to true).
        #[cfg(target_os = "macos")]
        {
            let is_mini = crate::platform::window_is_minimized();
            if self.was_minimized && !is_mini {
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                self.needs_pointer_reset.store(true, std::sync::atomic::Ordering::Relaxed);
            }
            self.was_minimized = is_mini;
        }

        if self.first_frame {
            self.first_frame = false;
            #[cfg(target_os = "macos")]
            crate::platform::apply_macos_transparency();
            #[cfg(target_os = "macos")]
            crate::platform::setup_macos_status_bar();

            #[cfg(target_os = "macos")]
            {
                let shared = std::sync::Arc::clone(&self.status_bar_state);
                let ctx_bg = ctx.clone();
                let needs_reset = std::sync::Arc::clone(&self.needs_pointer_reset);
                std::thread::spawn(move || {
                    let mut was_mini = false;
                    loop {
                        std::thread::sleep(std::time::Duration::from_millis(300));

                        // ── Status bar update ─────────────────────────────────
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

                        // ── Restore detection (backup path) ───────────────────
                        // Covers the race where ui() frames stop before
                        // isMiniaturized flips to true.
                        let is_mini = crate::platform::window_is_minimized();
                        if was_mini && !is_mini {
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

        // Update macOS menu bar each frame.
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
            if let Ok(mut s) = self.status_bar_state.lock() {
                *s = (self.seconds_left, self.running, emoji, std::time::Instant::now());
            }
        }

        self.tick();

        ctx.request_repaint_after(std::time::Duration::from_millis(
            if self.running { 200 } else { 500 },
        ));

        if let Some(size) = self.pending_resize.take() {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(
                egui::vec2(size[0], size[1]),
            ));
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
