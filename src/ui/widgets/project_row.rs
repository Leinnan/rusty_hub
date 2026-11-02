//! One row of the project list.

use std::time::SystemTime;

use eframe::egui::{
    Align, CornerRadius, Label, Layout, Response, RichText, Role, Sense, Ui, UiBuilder, Vec2,
    WidgetInfo, vec2,
};
use unity_hub::unity_project::UnityProject;

use super::{buttons::secondary_button, chip::chip, icons::Icon};
use crate::{
    commands::ProjectCmd,
    state::ProjectStatus,
    ui::{
        format::{abbreviate_home, relative_time},
        theme::colors::{
            CARD_BG, CARD_BG_HOVER, DELETE_RED_SOFT, HINT_TEXT, SELECTION_BG, SUCCESS, TEXT,
            TEXT_WEAK, WARNING,
        },
    },
};

/// Height of a row (px).
pub const ROW_HEIGHT: f32 = 52.0;
/// Rows narrower than this leave the branch to the details panel (px).
const BRANCH_MIN_WIDTH: f32 = 520.0;
/// Width of the modified-time column, so rows line up (px).
const TIME_WIDTH: f32 = 56.0;

/// What a project's status looks like.
#[must_use]
pub fn status_icon(status: ProjectStatus) -> (Icon, eframe::egui::Color32, &'static str) {
    match status {
        ProjectStatus::Ready => (Icon::CircleCheck, SUCCESS, "editor installed"),
        ProjectStatus::EditorMissing => (Icon::TriangleAlert, WARNING, "editor missing"),
        ProjectStatus::NotFound => (Icon::CircleX, DELETE_RED_SOFT, "folder not found"),
    }
}

/// One row. The returned response is the row's own, for selection and the
/// context menu; its Open button schedules the command itself.
pub fn project_row(
    ui: &mut Ui,
    project: &UnityProject,
    status: ProjectStatus,
    selected: bool,
    now: SystemTime,
) -> Response {
    let size = Vec2::new(ui.available_width(), ROW_HEIGHT);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    response.widget_info(|| WidgetInfo::selected(Role::Button, true, selected, &project.title));
    if !ui.is_rect_visible(rect) {
        return response;
    }

    let fill = if selected {
        SELECTION_BG
    } else if response.hovered() {
        CARD_BG_HOVER
    } else {
        CARD_BG
    };
    ui.painter().rect_filled(rect, CornerRadius::same(8), fill);

    let mut row = ui.new_child(
        UiBuilder::new()
            .max_rect(rect.shrink2(vec2(12.0, 6.0)))
            .layout(Layout::right_to_left(Align::Center)),
    );
    let open = row.add_enabled(status == ProjectStatus::Ready, secondary_button("Open"));
    if open.clicked() {
        ProjectCmd::Open(project.path.clone()).schedule();
    }

    let time_color = if selected { TEXT_WEAK } else { HINT_TEXT };
    let row_height = row.available_height();
    row.allocate_ui_with_layout(
        vec2(TIME_WIDTH, row_height),
        Layout::right_to_left(Align::Center),
        |ui| {
            ui.add(
                Label::new(
                    RichText::new(relative_time(project.edit_time, now))
                        .small()
                        .color(time_color),
                )
                .selectable(false),
            );
        },
    );

    let (icon, color, meaning) = status_icon(status);
    let version = if project.version.is_empty() {
        "Unknown"
    } else {
        project.version.as_str()
    };
    chip(
        &mut row,
        Some((icon, color)),
        version,
        &format!("{version}, {meaning}"),
    )
    .on_hover_text(meaning);
    if !project.branch.is_empty() && rect.width() >= BRANCH_MIN_WIDTH {
        chip(
            &mut row,
            Some((Icon::GitBranch, TEXT_WEAK)),
            &project.branch,
            &project.branch,
        );
    }

    // Title and path take the space left over, cut to fit.
    row.with_layout(Layout::top_down(Align::Min), |ui| {
        ui.spacing_mut().item_spacing.y = 2.0;
        ui.add_space(3.0);
        ui.add(
            Label::new(
                RichText::new(&project.title)
                    .size(13.5)
                    .strong()
                    .color(TEXT),
            )
            .truncate()
            .selectable(false),
        );
        ui.add(
            Label::new(
                RichText::new(abbreviate_home(&project.path))
                    .small()
                    .color(time_color),
            )
            .truncate()
            .selectable(false),
        );
    });
    response
}
