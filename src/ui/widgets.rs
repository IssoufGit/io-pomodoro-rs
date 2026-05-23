// Reusable UI widgets shared between full and tiny views.

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
