// Reusable UI widgets shared between full and tiny views.

use crate::app::PomodoroApp;
use eframe::egui;

/// A small three-line stat card used in the stats row (Today / Focus Time / All Time).
pub fn stat_card(ui: &mut egui::Ui, label: &str, value: &str, unit: &str) {
    egui::Frame::group(ui.style())
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.label(egui::RichText::new(label).small().color(egui::Color32::GRAY));
                ui.add_space(2.0);
                ui.label(egui::RichText::new(value).size(20.0));
                ui.add_space(1.0);
                ui.label(egui::RichText::new(unit).small().color(egui::Color32::GRAY));
            });
        });
}

/// Popup-free focus-duration control for the tiny strip, where a dropdown
/// would be clipped by the short window: "-5min" / "+5min" buttons that snap
/// to multiples of 5 within 5..=180 and grey out at either end.
pub fn focus_duration_stepper(ui: &mut egui::Ui, app: &mut PomodoroApp) {
    const STEP: u32 = 5;
    const MIN: u32 = 5;
    const MAX: u32 = 180;
    let current = app.settings.focus_min;
    let hover = format!("Focus duration: {current} min");

    // Snap to the neighbouring multiple of 5 (a custom 23 goes to 20 or 25).
    let down = (current.div_ceil(STEP) - 1) * STEP;
    let up = (current / STEP + 1) * STEP;

    let mut next = None;
    if ui
        .add_enabled(current > MIN, egui::Button::new("-5min").small())
        .on_hover_text(&hover)
        .clicked()
    {
        next = Some(down.max(MIN));
    }
    if ui
        .add_enabled(current < MAX, egui::Button::new("+5min").small())
        .on_hover_text(&hover)
        .clicked()
    {
        next = Some(up.min(MAX));
    }
    if let Some(min) = next {
        app.settings.focus_min = min;
        app.apply_settings_change();
    }
}
