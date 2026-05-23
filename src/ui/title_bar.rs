// Custom title bar: colored circle buttons + right-click context menu.
// Drag (left-click) moves the window. Right-click opens the context menu.

use crate::app::PomodoroApp;
use eframe::egui;

pub const HEIGHT: f32 = 30.0;

pub const COLOR_CLOSE: egui::Color32 = egui::Color32::from_rgb(255, 95, 86);
pub const COLOR_MINIMIZE: egui::Color32 = egui::Color32::from_rgb(255, 189, 46);
pub const COLOR_MAXIMIZE: egui::Color32 = egui::Color32::from_rgb(39, 201, 63);

// ── Circle button ─────────────────────────────────────────────────────────────

/// A 16 px circle button. Always colored; lightens on hover.
pub fn wm_button(ui: &mut egui::Ui, icon: &str, color: egui::Color32) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let fill = if response.hovered() {
            egui::Color32::from_rgb(
                ((color.r() as u16 + 255) / 2) as u8,
                ((color.g() as u16 + 255) / 2) as u8,
                ((color.b() as u16 + 255) / 2) as u8,
            )
        } else {
            color
        };
        ui.painter().circle_filled(rect.center(), 7.0, fill);
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            icon,
            egui::FontId::proportional(10.0),
            egui::Color32::from_rgba_unmultiplied(0, 0, 0, 180),
        );
    }
    response
}

// ── Background: drag + right-click context menu ───────────────────────────────

/// Register the title bar background:
/// - Left-click drag → move the window.
/// - Right-click → context menu with window management options.
pub fn setup_background(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    rect: egui::Rect,
    app: &mut PomodoroApp,
) {
    let bg = ui.interact(rect, egui::Id::new("tb_bg"), egui::Sense::click_and_drag());

    // Left-drag moves the window.
    if bg.drag_started() {
        ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
    }

    // Right-click opens our custom context menu (replaces the OS WM menu).
    bg.context_menu(|ui| {
        let is_maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));

        if ui.button("Minimize").clicked() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
            ui.close_menu();
        }

        let max_label = if is_maximized { "Restore" } else { "Maximize" };
        if ui.button(max_label).clicked() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!is_maximized));
            ui.close_menu();
        }

        ui.separator();

        // Always-on-top toggle — greyed out on Wayland.
        let aot_supported = crate::platform::always_on_top_supported();
        let aot_label = if app.always_on_top {
            "✓ Always on top"
        } else {
            "Always on top"
        };
        if ui.add_enabled(aot_supported, egui::Button::new(aot_label)).clicked() {
            app.always_on_top = !app.always_on_top;
            PomodoroApp::apply_always_on_top(ctx, app.always_on_top);
            ui.close_menu();
        }
        if !aot_supported {
            ui.label(
                egui::RichText::new("(not supported on Wayland)")
                    .small()
                    .color(egui::Color32::GRAY),
            );
        }

        ui.separator();

        if ui.button("Close").clicked() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            ui.close_menu();
        }
    });
}
