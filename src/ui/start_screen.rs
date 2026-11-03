//! What the projects page shows with no projects: new, scan and import
//! buttons, and a drop hint.

use eframe::egui::{self, RichText, Ui, UiBuilder};

use super::{
    theme::colors::{HINT_TEXT, TEXT, TEXT_WEAK},
    widgets::{
        buttons::{primary_button, secondary_button},
        icons::{Icon, glyph},
    },
};
use crate::commands::ProjectCmd;

/// Height of the content, for centering (points).
const HEIGHT: f32 = 150.0;
/// Width of the button row, for centering (points).
const BUTTONS_WIDTH: f32 = 400.0;

/// Render the start screen centered in `ui`'s rect.
pub fn render(ui: &mut Ui, enabled: bool) {
    let full = ui.max_rect();
    let top = (full.center().y - HEIGHT / 2.0).max(full.top() + 24.0);
    let rect = egui::Rect::from_min_max(egui::pos2(full.left(), top), full.right_bottom());
    ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| {
        ui.vertical_centered(|ui| {
            ui.label(glyph(Icon::FolderOpen).size(40.0).color(HINT_TEXT));
            ui.add_space(6.0);
            ui.label(
                RichText::new("No projects yet")
                    .size(17.0)
                    .strong()
                    .color(TEXT),
            );
            ui.label(RichText::new("Drop a folder here to add its projects").color(TEXT_WEAK));
            ui.add_space(14.0);
            ui.horizontal(|ui| {
                // Center the buttons.
                ui.add_space((ui.available_width() - BUTTONS_WIDTH).max(0.0) / 2.0);
                if ui
                    .add_enabled(enabled, primary_button("New Project…"))
                    .clicked()
                {
                    ProjectCmd::RequestNew(None).schedule();
                }
                if ui
                    .add_enabled(enabled, secondary_button("Scan Folder…"))
                    .clicked()
                {
                    ProjectCmd::ScanFolder.schedule();
                }
                if ui
                    .add_enabled(enabled, secondary_button("Import from Unity Hub"))
                    .clicked()
                {
                    ProjectCmd::ImportFromHub.schedule();
                }
            });
        });
    });
}
