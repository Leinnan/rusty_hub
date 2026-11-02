//! Bottom status bar: a hint on the left; job progress, counts and the
//! version link on the right.

use eframe::egui::{self, Align, Layout, RichText, Spinner, Ui};

use super::{HOMEPAGE, VERSION, theme::colors::TEXT_WEAK};
use crate::state::{Page, UiState};

/// Render the status bar contents.
pub fn render(ui: &mut Ui, state: &UiState) {
    ui.horizontal_centered(|ui| {
        let hint = match state.page {
            Page::Projects if state.hub.projects.is_empty() => "Drop a folder to add its projects",
            Page::Projects => "Enter to open · right-click for more",
            Page::Editors => "Editors are found in the search paths",
        };
        ui.label(small(hint));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.hyperlink_to(
                RichText::new(format!("{} v{VERSION}", egui::special_emojis::GITHUB)).small(),
                HOMEPAGE,
            );
            ui.label(small(&counts(state)));
            if let Some(job) = state.busy {
                ui.label(small(job.label()));
                ui.add(Spinner::new().size(12.0));
            }
        });
    });
}

fn counts(state: &UiState) -> String {
    match state.page {
        Page::Projects => {
            let total = state.hub.projects.len();
            let shown = state.visible.len();
            if shown == total {
                plural(total, "project")
            } else {
                format!("{shown} of {}", plural(total, "project"))
            }
        }
        Page::Editors => plural(state.hub.config.editors_configurations.len(), "editor"),
    }
}

/// "1 project", "2 projects".
#[must_use]
pub fn plural(count: usize, noun: &str) -> String {
    if count == 1 {
        format!("1 {noun}")
    } else {
        format!("{count} {noun}s")
    }
}

fn small(text: &str) -> RichText {
    RichText::new(text).small().color(TEXT_WEAK)
}
