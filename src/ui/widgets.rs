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
/// would be clipped by the short window: each click adds 5 minutes, wrapping
/// back to 5 after the 180-minute maximum.
pub fn focus_duration_stepper(ui: &mut egui::Ui, app: &mut PomodoroApp) {
    const STEP: u32 = 5;
    const MAX: u32 = 180;
    if ui
        .small_button(format!("{}m +", app.settings.focus_min))
        .on_hover_text("Focus duration — click to add 5 min")
        .clicked()
    {
        // Snap to the next multiple of 5 (a custom value like 23 goes to 25).
        let next = (app.settings.focus_min / STEP + 1) * STEP;
        app.settings.focus_min = if next > MAX { STEP } else { next };
        app.apply_settings_change();
    }
}
