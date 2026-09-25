// Compact "tiny mode" strip: a single horizontal row with the timer and all
// controls, meant to sit out of the way (e.g. just above the Dock).

use crate::app::PomodoroApp;
use crate::model::{FULL_SIZE, TINY_SIZE};
use crate::ui::title_bar;
use eframe::egui;

impl PomodoroApp {
    pub fn toggle_tiny(&mut self) {
        self.tiny_mode = !self.tiny_mode;
        self.pending_resize = Some(if self.tiny_mode { TINY_SIZE } else { FULL_SIZE });
    }

    pub fn render_tiny(&mut self, ctx: &egui::Context) {
        let frame = egui::Frame::NONE.fill(ctx.global_style().visuals.panel_fill);
        egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
            let rect = ui.max_rect();

            // The whole strip is the drag handle / right-click menu. Registered
            // first so the widgets added on top of it win hit-testing.
            title_bar::setup_background(ui, ctx, rect, self);

            // ── Progress: thin line along the bottom edge ─────────────────────
            let progress = if self.total_seconds > 0 {
                1.0 - (self.seconds_left as f32 / self.total_seconds as f32)
            } else {
                0.0
            };
            // Inset by the corner radius so the rounded window mask doesn't clip it.
            let track = egui::Rect::from_min_max(
                egui::pos2(rect.left() + 12.0, rect.bottom() - 3.0),
                egui::pos2(rect.right() - 12.0, rect.bottom() - 1.0),
            );
            ui.painter().rect_filled(track, 1.0, egui::Color32::from_gray(45));
            let mut fill = track;
            fill.set_width(track.width() * progress.clamp(0.0, 1.0));
            ui.painter().rect_filled(fill, 1.0, self.mode.color());

            let row = rect.shrink2(egui::vec2(12.0, 0.0));
            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(row), |ui| {
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    ui.spacing_mut().button_padding = egui::vec2(6.0, 2.0);

                    // ── Mode dot + time ───────────────────────────────────────
                    let (dot, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
                    ui.painter().circle_filled(dot.center(), 4.0, self.mode.color());
                    ui.add_space(4.0);

                    let color = if self.running {
                        egui::Color32::GRAY
                    } else {
                        ui.visuals().strong_text_color()
                    };
                    let (today_count, today_min, _) = self.today_stats();
                    // Whole minutes only — ticking seconds are distracting in
                    // an always-visible strip. Matches the status bar text.
                    let mut job = egui::text::LayoutJob::default();
                    job.append(
                        &format!("{}", self.seconds_left / 60),
                        0.0,
                        egui::text::TextFormat {
                            font_id: egui::FontId::monospace(18.0),
                            color,
                            ..Default::default()
                        },
                    );
                    job.append(
                        " min",
                        0.0,
                        egui::text::TextFormat {
                            font_id: egui::FontId::proportional(12.0),
                            color,
                            valign: egui::Align::Center,
                            ..Default::default()
                        },
                    );
                    ui.label(job)
                    .on_hover_text(format!(
                        "{} · today: {} sessions, {}m",
                        self.mode.label(),
                        today_count,
                        today_min.round() as i64
                    ));
                    ui.add_space(6.0);

                    // ── Controls ──────────────────────────────────────────────
                    let start_text = if self.running { "⏸" } else { "▶" };
                    if ui.small_button(start_text).on_hover_text("Start / pause (Space)").clicked() {
                        self.start_or_pause();
                    }
                    if ui.small_button("⟳").on_hover_text("Reset (R)").clicked() {
                        self.reset();
                    }
                    if ui.small_button("✓").on_hover_text("Complete & save").clicked() {
                        self.complete_session(false);
                    }
                    crate::ui::widgets::focus_duration_stepper(ui, self);

                    // ── Window controls (right-aligned) ───────────────────────
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // On Wayland the OS title bar provides close/minimize.
                        if !crate::platform::using_os_decorations() {
                            if title_bar::wm_button(ui, "×", title_bar::COLOR_CLOSE).clicked() {
                                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                            if title_bar::wm_button(ui, "−", title_bar::COLOR_MINIMIZE).clicked() {
                                crate::platform::minimize_window(ctx);
                            }
                            ui.add_space(2.0);
                        }
                        if ui.small_button("⛶").on_hover_text("Expand (T)").clicked() {
                            self.toggle_tiny();
                        }
                        let aot_supported = crate::platform::always_on_top_supported();
                        let pin_color = if self.always_on_top {
                            egui::Color32::from_rgb(255, 189, 46)
                        } else {
                            egui::Color32::GRAY
                        };
                        let pin_resp = ui
                            .add_enabled(
                                aot_supported,
                                egui::Button::new(
                                    egui::RichText::new("📌").color(pin_color).small(),
                                )
                                .frame(false),
                            )
                            .on_hover_text("Always on top (A) — or right-click the strip");
                        let pin_resp =
                            if let Some(reason) = crate::platform::always_on_top_unsupported_reason() {
                                pin_resp.on_disabled_hover_text(format!("Always on top — {reason}"))
                            } else {
                                pin_resp
                            };
                        if pin_resp.clicked() {
                            self.always_on_top = !self.always_on_top;
                            PomodoroApp::apply_always_on_top(ctx, self.always_on_top);
                        }
                    });
                });
            });
        });
    }
}
