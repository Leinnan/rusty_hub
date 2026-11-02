//! Asks before removing a project from the list.

use eframe::egui::{self, Align, Layout, RichText};

use super::{
    theme::colors::{TEXT, TEXT_WEAK},
    widgets::buttons::{destructive_button, secondary_button},
};
use crate::{commands::ProjectCmd, state::UiState};

/// Width of the sheet (px).
const WIDTH: f32 = 340.0;

/// Show the sheet if a project is waiting for confirmation.
pub fn render(ctx: &egui::Context, state: &mut UiState) {
    let Some(path) = state.confirm_remove.clone() else {
        return;
    };
    let title = state
        .project(&path)
        .map_or(path.as_str(), |p| p.title.as_str());
    let mut close = false;
    let frame = egui::Frame::window(&ctx.global_style()).inner_margin(egui::Margin::same(16));
    let modal = egui::Modal::new(egui::Id::unique("confirm_remove")).frame(frame);
    let response = modal.show(ctx, |ui| {
        ui.set_width(WIDTH);
        ui.label(
            RichText::new(format!("Remove “{title}” from the list?"))
                .heading()
                .color(TEXT),
        );
        ui.label(RichText::new("The project's files stay on disk.").color(TEXT_WEAK));
        ui.add_space(10.0);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui.add(destructive_button("Remove")).clicked() {
                ProjectCmd::ConfirmRemove(path.clone()).schedule();
                close = true;
            }
            if ui.add(secondary_button("Cancel")).clicked() {
                close = true;
            }
        });
    });
    if close || response.should_close() {
        state.confirm_remove = None;
    }
}
