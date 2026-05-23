// Compact "tiny mode" overlay: just the timer, a progress bar, and controls.

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
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(4.0);

            if crate::platform::using_os_decorations() {
                // ── Wayland: simple top row, OS title bar handles the rest ────
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(self.mode.short_label().to_uppercase())
                            .small()
                            .color(self.mode.color()),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("⛶").on_hover_text("Expand (T)").clicked() {
                            self.toggle_tiny();
                        }
                    });
                });
            } else {
                // ── X11 / macOS / Windows: custom title bar ───────────────────
                let bar_rect = egui::Rect::from_min_size(
                    ui.cursor().min,
                    egui::vec2(ui.available_width(), title_bar::HEIGHT),
                );
                title_bar::setup_background(ui, ctx, bar_rect, self);
                ui.allocate_new_ui(egui::UiBuilder::new().max_rect(bar_rect), |ui| {
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        ui.add_space(8.0);
                        ui.label(
                            egui::RichText::new(self.mode.short_label().to_uppercase())
                                .small()
                                .color(self.mode.color()),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add_space(8.0);
                            if title_bar::wm_button(ui, "×", title_bar::COLOR_CLOSE).clicked() {
                                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                            ui.add_space(6.0);
                            if title_bar::wm_button(ui, "−", title_bar::COLOR_MINIMIZE).clicked() {
                                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                            }
                            ui.add_space(8.0);
                            ui.separator();
                            ui.add_space(4.0);
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
                                .on_hover_text("Always on top (A) — or right-click title bar");
                            if pin_resp.clicked() {
                                self.always_on_top = !self.always_on_top;
                                PomodoroApp::apply_always_on_top(ctx, self.always_on_top);
                            }
                            ui.add_space(4.0);
                            if ui.small_button("⛶").on_hover_text("Expand (T)").clicked() {
                                self.toggle_tiny();
                            }
                        });
                    });
                });
            }

            // ── Timer display (same on all platforms) ─────────────────────────
            let mins = self.seconds_left / 60;
            let color = if self.running {
                egui::Color32::GRAY
            } else {
                ui.visuals().strong_text_color()
            };
            ui.vertical_centered(|ui| {
                let mut job = egui::text::LayoutJob::default();
                job.append(
                    &format!("{}", mins),
                    0.0,
                    egui::text::TextFormat {
                        font_id: egui::FontId::monospace(40.0),
                        color,
                        ..Default::default()
                    },
                );
                job.append(
                    " min",
                    0.0,
                    egui::text::TextFormat {
                        font_id: egui::FontId::proportional(16.0),
                        color,
                        valign: egui::Align::Center,
                        ..Default::default()
                    },
                );
                ui.label(job);
            });

            // ── Progress bar ──────────────────────────────────────────────────
            let progress = if self.total_seconds > 0 {
                1.0 - (self.seconds_left as f32 / self.total_seconds as f32)
            } else {
                0.0
            };
            ui.add(
                egui::ProgressBar::new(progress)
                    .desired_width(ui.available_width())
                    .fill(egui::Color32::GRAY),
            );
            ui.add_space(4.0);

            // ── Controls + today summary ──────────────────────────────────────
            ui.horizontal(|ui| {
                let start_text = if self.running { "⏸" } else { "▶" };
                if ui.small_button(start_text).clicked() {
                    self.start_or_pause();
                }
                if ui.small_button("⟳").on_hover_text("Reset (R)").clicked() {
                    self.reset();
                }
                if ui.small_button("✓").on_hover_text("Complete & save").clicked() {
                    self.complete_session(false);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let (today_count, today_min, _) = self.today_stats();
                    ui.label(
                        egui::RichText::new(format!(
                            "{} · {}m",
                            today_count,
                            today_min.round() as i64
                        ))
                        .small()
                        .color(egui::Color32::GRAY),
                    );
                });
            });
        });
    }
}
