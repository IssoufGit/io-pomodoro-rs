// Local-only pomodoro timer with persistent history.
// No network. State saved to ~/.local/share/pomodoro/history.json (XDG_DATA_HOME).

mod app;
mod icon;
mod model;
mod persistence;
mod timer;
mod platform;
mod ui;

use app::PomodoroApp;
use eframe::egui;
use icon::make_icon;
use model::FULL_SIZE;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: {
            // On Wayland let the compositor draw decorations — it provides
            // the right-click title bar menu including "Always on top" on KDE.
            // On X11, macOS, and Windows we use our own custom title bar.
            let os_decorations = crate::platform::using_os_decorations();
            egui::ViewportBuilder::default()
                .with_inner_size(FULL_SIZE)
                .with_min_inner_size([220.0, 100.0])
                .with_title("Pomodoro")
                .with_app_id("pomodoro")
                .with_icon(std::sync::Arc::new(make_icon()))
                .with_decorations(os_decorations)
                .with_transparent(true)
        },
        ..Default::default()
    };

    eframe::run_native(
        "Pomodoro",
        options,
        Box::new(|cc| {
            // Spacing and padding
            let mut style = (*cc.egui_ctx.style()).clone();
            style.spacing.item_spacing = egui::vec2(6.0, 6.0);
            style.spacing.button_padding = egui::vec2(10.0, 5.0);
            cc.egui_ctx.set_style(style);

            // Semi-transparent visuals so the OS compositor shows through.
            // Each egui layer has its own color slot; all must be set.
            let mut visuals = egui::Visuals::dark();
            // Top-level panels and floating windows
            let panel_bg = egui::Color32::from_rgba_unmultiplied(20, 20, 20, 200);
            visuals.panel_fill = panel_bg;
            visuals.window_fill = panel_bg;
            // Frame::group fills (stat cards, settings boxes, history list)
            visuals.widgets.noninteractive.bg_fill =
                egui::Color32::from_rgba_unmultiplied(35, 35, 35, 180);
            // DragValue, TextEdit, ScrollArea track
            visuals.extreme_bg_color =
                egui::Color32::from_rgba_unmultiplied(10, 10, 10, 160);
            // ScrollArea trough and alternating-row tint
            visuals.faint_bg_color =
                egui::Color32::from_rgba_unmultiplied(255, 255, 255, 5);
            cc.egui_ctx.set_visuals(visuals);

            Ok(Box::new(PomodoroApp::new()) as Box<dyn eframe::App>)
        }),
    )
}
