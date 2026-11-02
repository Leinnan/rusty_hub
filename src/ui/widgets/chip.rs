//! Small rounded labels: a branch, a Unity version, a platform.

use eframe::egui::{
    Align2, Color32, CornerRadius, FontId, Response, Role, Sense, Ui, Vec2, WidgetInfo,
};

use super::icons::Icon;
use crate::ui::theme::colors::{CARD_BG_HOVER, TEXT_WEAK};

/// Chip height (px).
const HEIGHT: f32 = 20.0;
/// Space inside the chip's ends (px).
const PAD_X: f32 = 7.0;
/// Space between the icon and the text (px).
const ICON_GAP: f32 = 4.0;
/// Icon size (px).
const ICON_SIZE: f32 = 12.0;
/// Widest a chip gets before its text is cut (px).
const MAX_TEXT_WIDTH: f32 = 130.0;

/// A chip showing `text`, after `icon` in its color if there is one. The
/// hover text says the whole of a long `text`; `label` is the accessible name.
pub fn chip(ui: &mut Ui, icon: Option<(Icon, Color32)>, text: &str, label: &str) -> Response {
    let font = FontId::proportional(11.0);
    let galley =
        ui.fonts_mut(|fonts| fonts.layout_no_wrap(text.to_owned(), font.clone(), TEXT_WEAK));
    let text_width = galley.size().x.min(MAX_TEXT_WIDTH);
    let icon_width = if icon.is_some() {
        ICON_SIZE + ICON_GAP
    } else {
        0.0
    };
    let size = Vec2::new(PAD_X * 2.0 + icon_width + text_width, HEIGHT);
    let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        painter.rect_filled(rect, CornerRadius::same(5), CARD_BG_HOVER);
        let mut x = rect.left() + PAD_X;
        if let Some((icon, color)) = icon {
            painter.text(
                eframe::egui::pos2(x + ICON_SIZE / 2.0, rect.center().y),
                Align2::CENTER_CENTER,
                char::from(icon),
                FontId::proportional(ICON_SIZE),
                color,
            );
            x += ICON_SIZE + ICON_GAP;
        }
        let top = rect.center().y - galley.size().y / 2.0;
        painter
            .with_clip_rect(rect.shrink2(Vec2::new(PAD_X, 0.0)))
            .galley(eframe::egui::pos2(x, top), galley.clone(), TEXT_WEAK);
    }
    response.widget_info(|| WidgetInfo::labeled(Role::Label, true, label));
    if galley.size().x > MAX_TEXT_WIDTH {
        response.on_hover_text(text)
    } else {
        response
    }
}
