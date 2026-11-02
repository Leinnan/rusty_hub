//! The left sidebar: the page list with counts.

use eframe::egui::{
    Align2, CornerRadius, FontId, Response, Role, Sense, Ui, Vec2, WidgetInfo, pos2,
};

use super::{
    theme::colors::{CARD_BG_HOVER, HINT_TEXT, SELECTION_BG, TEXT, TEXT_WEAK},
    widgets::{icons::Icon, section::section_header},
};
use crate::{
    commands::ViewCmd,
    state::{Page, UiState},
};

/// Height of a page row (px).
const ROW_HEIGHT: f32 = 30.0;

/// Render the sidebar contents.
pub fn render(ui: &mut Ui, state: &UiState) {
    section_header(ui, "Library");
    ui.spacing_mut().item_spacing.y = 2.0;
    let pages = [
        (
            Page::Projects,
            Icon::FolderOpen,
            "Projects",
            state.hub.projects.len(),
        ),
        (
            Page::Editors,
            Icon::Hammer,
            "Editors",
            state.hub.config.editors_configurations.len(),
        ),
    ];
    for (page, icon, label, count) in pages {
        if page_row(ui, icon, label, count, state.page == page).clicked() {
            ViewCmd::ShowPage(page).schedule();
        }
    }
}

fn page_row(ui: &mut Ui, icon: Icon, label: &str, count: usize, selected: bool) -> Response {
    let size = Vec2::new(ui.available_width(), ROW_HEIGHT);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    response.widget_info(|| WidgetInfo::selected(Role::Button, true, selected, label));
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        if selected {
            painter.rect_filled(rect, CornerRadius::same(6), SELECTION_BG);
        } else if response.hovered() {
            painter.rect_filled(rect, CornerRadius::same(6), CARD_BG_HOVER);
        }
        let color = if selected { TEXT } else { TEXT_WEAK };
        let middle = rect.center().y;
        painter.text(
            pos2(rect.left() + 10.0, middle),
            Align2::LEFT_CENTER,
            char::from(icon),
            FontId::proportional(15.0),
            color,
        );
        painter.text(
            pos2(rect.left() + 34.0, middle),
            Align2::LEFT_CENTER,
            label,
            FontId::proportional(13.0),
            color,
        );
        painter.text(
            pos2(rect.right() - 10.0, middle),
            Align2::RIGHT_CENTER,
            count.to_string(),
            FontId::proportional(11.0),
            if selected { TEXT } else { HINT_TEXT },
        );
    }
    response
}
