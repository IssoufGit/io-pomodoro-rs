// Reusable UI widgets shared between full and tiny views.

use crate::app::PomodoroApp;
use crate::model::FOCUS_PRESETS;
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

/// Compact focus-duration picker for tight spaces (tiny mode): a menu button
/// showing the current duration that opens a popup with the same presets and
/// custom drag-value used in the full view's "Focus duration" group.
pub fn focus_duration_menu(ui: &mut egui::Ui, app: &mut PomodoroApp) {
    ui.menu_button(format!("{}m ▾", app.settings.focus_min), |ui| {
        let mut changed = false;
        ui.horizontal_wrapped(|ui| {
            for preset in FOCUS_PRESETS {
                let selected = app.settings.focus_min == preset;
                if ui.selectable_label(selected, format!("{}m", preset)).clicked() && !selected {
                    app.settings.focus_min = preset;
                    changed = true;
                    ui.close_menu();
                }
            }
        });
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label("Custom:");
            let resp = ui.add(
                egui::DragValue::new(&mut app.settings.focus_min)
                    .range(1..=180)
                    .speed(1.0)
                    .suffix(" min"),
            );
            changed |= resp.changed();
        });
        if changed {
            app.apply_settings_change();
        }
    });
}
