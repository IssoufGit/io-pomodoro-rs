// Full expanded view: timer, controls, settings, stats, and history.

use crate::app::PomodoroApp;
use crate::model::{FOCUS_PRESETS, Mode};
use crate::persistence::{data_file_path, export_csv};
use crate::ui::{title_bar, widgets::stat_card};
use chrono::Local;
use eframe::egui;

impl PomodoroApp {
    pub fn render_full(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {

            if crate::platform::using_os_decorations() {
                // ── Wayland: OS title bar is drawn by the compositor.
                // We just render a lightweight in-app header row.
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.heading("pomodoro");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("⛶").on_hover_text("Tiny mode (T)").clicked() {
                            self.toggle_tiny();
                        }
                        // Pin button (greyed out on Wayland with a tooltip)
                        let pin_color = egui::Color32::from_gray(60);
                        ui.add_enabled(
                            false,
                            egui::Button::new(
                                egui::RichText::new("📌").color(pin_color).small(),
                            ).frame(false),
                        ).on_disabled_hover_text(
                            "Always on top — right-click the OS title bar on KDE Wayland"
                        );
                        ui.label(
                            egui::RichText::new(Local::now().format("%a, %b %-d").to_string())
                                .small()
                                .color(egui::Color32::GRAY),
                        );
                    });
                });
                ui.separator();
                ui.add_space(4.0);
            } else {
                // ── X11 / macOS / Windows: draw our custom title bar.
                let bar_rect = egui::Rect::from_min_size(
                    ui.cursor().min,
                    egui::vec2(ui.available_width(), title_bar::HEIGHT),
                );
                title_bar::setup_background(ui, ctx, bar_rect, self);
                ui.allocate_new_ui(egui::UiBuilder::new().max_rect(bar_rect), |ui| {
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        ui.add_space(10.0);
                        ui.label(
                            egui::RichText::new("pomodoro")
                                .small()
                                .color(egui::Color32::from_gray(130)),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add_space(8.0);
                            if title_bar::wm_button(ui, "×", title_bar::COLOR_CLOSE).clicked() {
                                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                            ui.add_space(6.0);
                            let is_maximized =
                                ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                            let max_icon = if is_maximized { "❐" } else { "□" };
                            if title_bar::wm_button(ui, max_icon, title_bar::COLOR_MAXIMIZE)
                                .clicked()
                            {
                                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(
                                    !is_maximized,
                                ));
                            }
                            ui.add_space(6.0);
                            if title_bar::wm_button(ui, "−", title_bar::COLOR_MINIMIZE).clicked() {
                                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                            }
                            ui.add_space(8.0);
                            ui.separator();
                            ui.add_space(4.0);
                            // Always-on-top pin
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
                            if ui.small_button("⛶").on_hover_text("Tiny mode (T)").clicked() {
                                self.toggle_tiny();
                            }
                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new(
                                    Local::now().format("%a, %b %-d").to_string(),
                                )
                                .small()
                                .color(egui::Color32::GRAY),
                            );
                        });
                    });
                });
                ui.separator();
            }

            // ── Scrollable content (same on all platforms) ────────────────────
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add_space(8.0);

                // ── Mode tabs ─────────────────────────────────────────────────
                ui.horizontal(|ui| {
                    ui.with_layout(
                        egui::Layout::top_down(egui::Align::Center).with_cross_justify(false),
                        |ui| {
                            ui.horizontal(|ui| {
                                for m in [Mode::Focus, Mode::ShortBreak, Mode::LongBreak] {
                                    let selected = self.mode == m;
                                    if ui.selectable_label(selected, m.label()).clicked()
                                        && !selected
                                    {
                                        self.set_mode(m);
                                    }
                                }
                            });
                        },
                    );
                });
                ui.add_space(12.0);

                // ── Timer display ─────────────────────────────────────────────
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
                            font_id: egui::FontId::monospace(64.0),
                            color,
                            ..Default::default()
                        },
                    );
                    job.append(
                        " min",
                        0.0,
                        egui::text::TextFormat {
                            font_id: egui::FontId::proportional(26.0),
                            color,
                            valign: egui::Align::BOTTOM,
                            ..Default::default()
                        },
                    );
                    ui.label(job);
                    ui.add_space(2.0);
                    ui.label(
                        egui::RichText::new(self.status.to_uppercase())
                            .small()
                            .color(egui::Color32::GRAY),
                    );
                });

                // ── Progress bar ──────────────────────────────────────────────
                let progress = if self.total_seconds > 0 {
                    1.0 - (self.seconds_left as f32 / self.total_seconds as f32)
                } else {
                    0.0
                };
                ui.add_space(8.0);
                ui.vertical_centered(|ui| {
                    ui.add(
                        egui::ProgressBar::new(progress)
                            .desired_width(220.0)
                            .fill(egui::Color32::GRAY),
                    );
                });
                ui.add_space(12.0);

                // ── Controls ──────────────────────────────────────────────────
                ui.vertical_centered(|ui| {
                    ui.horizontal(|ui| {
                        let total_w = 92.0 + 70.0 + 92.0 + 8.0 * 2.0;
                        ui.add_space((ui.available_width() - total_w).max(0.0) / 2.0);
                        let start_text = if self.running { "⏸  Pause" } else { "▶  Start" };
                        if ui.add_sized([92.0, 30.0], egui::Button::new(start_text)).clicked() {
                            self.start_or_pause();
                        }
                        if ui.add_sized([70.0, 30.0], egui::Button::new("Reset")).clicked() {
                            self.reset();
                        }
                        if ui
                            .add_sized([92.0, 30.0], egui::Button::new("Complete"))
                            .on_hover_text("Save partial session to history")
                            .clicked()
                        {
                            self.complete_session(false);
                        }
                    });
                });
                ui.add_space(14.0);

                // ── Focus duration ────────────────────────────────────────────
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Focus duration").strong());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(format!("{} min", self.settings.focus_min))
                                    .small()
                                    .color(egui::Color32::GRAY),
                            );
                        });
                    });
                    ui.add_space(4.0);

                    let mut changed = false;
                    ui.horizontal_wrapped(|ui| {
                        for preset in FOCUS_PRESETS {
                            let selected = self.settings.focus_min == preset;
                            if ui.selectable_label(selected, format!("{}m", preset)).clicked()
                                && !selected
                            {
                                self.settings.focus_min = preset;
                                changed = true;
                            }
                        }
                    });
                    ui.add_space(4.0);

                    ui.horizontal(|ui| {
                        ui.label("Custom:");
                        let resp = ui.add(
                            egui::DragValue::new(&mut self.settings.focus_min)
                                .range(1..=180)
                                .speed(1.0)
                                .suffix(" min"),
                        );
                        changed |= resp.changed();
                        ui.label(
                            egui::RichText::new("(drag or double-click to type)")
                                .small()
                                .color(egui::Color32::GRAY),
                        );
                    });
                    if changed {
                        self.apply_settings_change();
                    }
                });
                ui.add_space(8.0);

                // ── Break durations (collapsible) ─────────────────────────────
                egui::CollapsingHeader::new("Break durations")
                    .default_open(false)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let mut changed = false;
                            ui.label("Short");
                            changed |= ui
                                .add(
                                    egui::DragValue::new(&mut self.settings.short_min)
                                        .range(1..=60)
                                        .suffix(" min"),
                                )
                                .changed();
                            ui.add_space(12.0);
                            ui.label("Long");
                            changed |= ui
                                .add(
                                    egui::DragValue::new(&mut self.settings.long_min)
                                        .range(1..=60)
                                        .suffix(" min"),
                                )
                                .changed();
                            if changed {
                                self.apply_settings_change();
                            }
                        });
                    });
                ui.add_space(12.0);

                // ── Stats row ─────────────────────────────────────────────────
                let (today_count, today_min, all_time) = self.today_stats();
                ui.columns(3, |cols| {
                    stat_card(&mut cols[0], "TODAY", &format!("{}", today_count), "sessions");
                    stat_card(
                        &mut cols[1],
                        "FOCUS TIME",
                        &format!("{}m", today_min.round() as i64),
                        "today",
                    );
                    stat_card(&mut cols[2], "ALL TIME", &format!("{}", all_time), "sessions");
                });
                ui.add_space(20.0);

                // ── History ───────────────────────────────────────────────────
                ui.horizontal(|ui| {
                    ui.heading("History");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if self.confirm_clear {
                            if ui.button("Yes, clear").clicked() {
                                self.clear_history();
                                self.confirm_clear = false;
                            }
                            if ui.button("Cancel").clicked() {
                                self.confirm_clear = false;
                            }
                        } else {
                            if ui.button("Clear").clicked() {
                                self.confirm_clear = true;
                            }
                            if ui.button("Export CSV").clicked() {
                                let _ = export_csv(&self.sessions);
                            }
                        }
                    });
                });
                ui.add_space(4.0);

                if self.sessions.is_empty() {
                    ui.add_space(20.0);
                    ui.vertical_centered(|ui| {
                        ui.label(
                            egui::RichText::new(
                                "No sessions yet — start your first focus block above.",
                            )
                            .italics()
                            .color(egui::Color32::GRAY),
                        );
                    });
                    ui.add_space(20.0);
                } else {
                    let mut current_day: Option<chrono::NaiveDate> = None;
                    egui::Frame::group(ui.style()).inner_margin(8.0).show(ui, |ui| {
                        for s in self.sessions.iter().take(200) {
                            let local = s.completed_at.with_timezone(&Local);
                            let day = local.date_naive();
                            if Some(day) != current_day {
                                current_day = Some(day);
                                let today = Local::now().date_naive();
                                let yesterday = today.pred_opt().unwrap_or(today);
                                let day_label = if day == today {
                                    "Today".to_string()
                                } else if day == yesterday {
                                    "Yesterday".to_string()
                                } else {
                                    day.format("%A, %b %-d").to_string()
                                };
                                ui.add_space(4.0);
                                ui.label(
                                    egui::RichText::new(day_label.to_uppercase())
                                        .small()
                                        .color(egui::Color32::GRAY),
                                );
                                ui.separator();
                            }
                            ui.horizontal(|ui| {
                                let badge_color = s.mode.color();
                                let (rect, _) = ui.allocate_exact_size(
                                    egui::vec2(60.0, 18.0),
                                    egui::Sense::hover(),
                                );
                                let (bg, text_color) = if s.mode == Mode::Focus {
                                    (egui::Color32::from_rgb(46, 139, 87), egui::Color32::GRAY)
                                } else {
                                    (badge_color.linear_multiply(0.18), badge_color)
                                };
                                ui.painter().rect_filled(rect, 3.0, bg);
                                ui.painter().text(
                                    rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    s.mode.short_label(),
                                    egui::FontId::proportional(11.0),
                                    text_color,
                                );
                                ui.label(
                                    egui::RichText::new(local.format("%H:%M").to_string())
                                        .monospace()
                                        .color(egui::Color32::GRAY),
                                );
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        let suffix = if s.completed { "" } else { " · partial" };
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "{}m{}",
                                                s.duration_minutes.round() as i64,
                                                suffix,
                                            ))
                                            .monospace(),
                                        );
                                    },
                                );
                            });
                        }
                    });
                }

                // ── Footer ────────────────────────────────────────────────────
                ui.add_space(16.0);
                ui.separator();
                ui.vertical_centered(|ui| {
                    ui.label(
                        egui::RichText::new(format!(
                            "Local-only · No network · History: {}",
                            data_file_path().display()
                        ))
                        .small()
                        .color(egui::Color32::GRAY),
                    );
                });
                ui.add_space(8.0);
            });
        });
    }
}
