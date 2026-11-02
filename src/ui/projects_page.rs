//! The project list, or the start screen when there are no projects.

use std::time::SystemTime;

use eframe::egui::{Align, Layout, RichText, ScrollArea, Ui};

use super::{
    details_panel::project_menu,
    start_screen,
    status_bar::plural,
    theme::colors::TEXT_WEAK,
    widgets::{
        project_row::{ROW_HEIGHT, project_row},
        segmented::segmented_control,
    },
};
use crate::{
    commands::{ProjectCmd, ViewCmd},
    state::{SortKey, UiState},
};

/// Space between rows (px).
const ROW_GAP: f32 = 4.0;

/// Render the page.
pub fn render(ui: &mut Ui, state: &mut UiState) {
    if state.hub.projects.is_empty() {
        start_screen::render(ui, state.busy.is_none());
        return;
    }
    header(ui, state);
    ui.add_space(8.0);
    if state.visible.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(40.0);
            ui.label(
                RichText::new(format!("No projects match “{}”", state.search)).color(TEXT_WEAK),
            );
        });
        return;
    }
    list(ui, state);
}

fn header(ui: &mut Ui, state: &UiState) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(plural(state.visible.len(), "project")).color(TEXT_WEAK));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let labels = SortKey::ALL.map(SortKey::label);
            let selected = SortKey::ALL.iter().position(|&key| key == state.sort);
            let segments = segmented_control(ui, "project_sort", &labels, selected);
            for (segment, key) in segments.iter().zip(SortKey::ALL) {
                if segment.clicked() {
                    ViewCmd::SetSort(key).schedule();
                }
            }
            ui.label(RichText::new("Sort").small().color(TEXT_WEAK));
        });
    });
}

fn list(ui: &mut Ui, state: &mut UiState) {
    let mut area = ScrollArea::vertical().auto_shrink([false, false]);
    if std::mem::take(&mut state.scroll_to_selected)
        && let Some(offset) = offset_showing_selection(state)
    {
        area = area.vertical_scroll_offset(offset);
    }

    ui.spacing_mut().item_spacing.y = ROW_GAP;
    let now = SystemTime::now();
    let shown: &UiState = state;
    let output = area.show_rows(ui, ROW_HEIGHT, shown.visible.len(), |ui, rows| {
        for row in rows {
            let project = &shown.hub.projects[shown.visible[row]];
            let selected = shown.selected.as_deref() == Some(project.path.as_str());
            let response = project_row(ui, project, shown.status(project), selected, now);
            if response.clicked() {
                ViewCmd::Select(project.path.clone()).schedule();
            }
            if response.double_clicked() {
                ProjectCmd::Open(project.path.clone()).schedule();
            }
            response.context_menu(|ui| project_menu(ui, shown, project));
        }
    });
    state.list_offset = output.state.clamped_offset().y;
    state.list_viewport = output.inner_rect.height();
}

/// The scroll offset that brings the selected row into view, if it isn't.
/// Rows off screen aren't laid out, so this works from the row height.
fn offset_showing_selection(state: &UiState) -> Option<f32> {
    let path = state.selected.as_deref()?;
    let row = state
        .visible
        .iter()
        .position(|&ix| state.hub.projects[ix].path == path)?;
    let top = row as f32 * (ROW_HEIGHT + ROW_GAP);
    let bottom = top + ROW_HEIGHT;
    if top < state.list_offset {
        Some(top)
    } else if bottom > state.list_offset + state.list_viewport {
        Some(bottom - state.list_viewport)
    } else {
        None
    }
}
