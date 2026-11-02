//! Toolbar and sheet buttons.

use eframe::egui::{
    self, Align2, Button, Color32, CornerRadius, FontId, Rect, Response, RichText, Role, Sense,
    Stroke, Ui, Vec2, WidgetInfo,
};

use super::icons::Icon;
use crate::ui::theme::colors::{
    ACCENT_BLUE, BORDER, CARD_BG, CARD_BG_HOVER, CONTROL_TRACK, DESTRUCTIVE_BG, HINT_TEXT, TEXT,
    TEXT_WEAK,
};

/// Icon button corner rounding (px).
const ICON_ROUNDING: u8 = 6;

/// Sheet button corner rounding (px).
const CTA_ROUNDING: u8 = 6;

/// Filled accent button for the default action ("Open").
pub fn primary_button(text: &str) -> Button<'_> {
    Button::new(RichText::new(text).color(TEXT).strong())
        .fill(ACCENT_BLUE)
        .stroke(Stroke::new(1.0, ACCENT_BLUE))
        .min_size(Vec2::new(72.0, 24.0))
        .corner_radius(CornerRadius::same(CTA_ROUNDING))
}

/// Red button for an action that destroys something ("Remove").
pub fn destructive_button(text: &str) -> Button<'_> {
    Button::new(RichText::new(text).color(TEXT).strong())
        .fill(DESTRUCTIVE_BG)
        .stroke(Stroke::new(1.0, DESTRUCTIVE_BG))
        .min_size(Vec2::new(72.0, 24.0))
        .corner_radius(CornerRadius::same(CTA_ROUNDING))
}

/// Outlined low-emphasis button ("Cancel").
pub fn secondary_button(text: &str) -> Button<'_> {
    Button::new(RichText::new(text).color(TEXT))
        .fill(CARD_BG)
        .stroke(Stroke::new(1.0, BORDER))
        .min_size(Vec2::new(72.0, 24.0))
        .corner_radius(CornerRadius::same(CTA_ROUNDING))
}

/// How an icon button shows its state.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Look {
    /// A plain action.
    Plain,
    /// An on/off option: "on" keeps a fill.
    Toggle(bool),
}

/// The fill and content color for a button in `look` and `response`'s
/// interaction state.
fn button_colors(look: Look, response: &Response, enabled: bool) -> (Option<Color32>, Color32) {
    if !enabled {
        return (None, HINT_TEXT);
    }
    let (hovered, pressed) = (response.hovered(), response.is_pointer_button_down_on());
    match look {
        Look::Toggle(true) => (
            Some(if hovered || pressed {
                CONTROL_TRACK
            } else {
                CARD_BG_HOVER
            }),
            TEXT,
        ),
        Look::Plain | Look::Toggle(false) => {
            let fill = if pressed {
                Some(CONTROL_TRACK)
            } else if hovered {
                Some(CARD_BG_HOVER)
            } else {
                None
            };
            let color = if look == Look::Plain || hovered {
                TEXT
            } else {
                TEXT_WEAK
            };
            (fill, color)
        }
    }
}

/// An icon button: `icon` at `glyph_size` centred in `size`, with a state
/// fill behind it. Unlike egui's `Button`, whose frame (and so its content)
/// shifts when hover adds a stroke, nothing here depends on the state but the
/// colors. `tip` is both the hover text and the accessible name.
fn icon_button(
    ui: &mut Ui,
    icon: Icon,
    size: Vec2,
    glyph_size: f32,
    look: Look,
    enabled: bool,
    tip: &str,
) -> Response {
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(size, sense);
    if ui.is_rect_visible(rect) {
        let (fill, color) = button_colors(look, &response, enabled);
        let painter = ui.painter();
        if let Some(fill) = fill {
            painter.rect_filled(rect, CornerRadius::same(ICON_ROUNDING), fill);
        }
        paint_icon(painter, rect, icon, glyph_size, color);
    }
    let response = response.on_hover_text(tip);
    response.widget_info(|| WidgetInfo::labeled(Role::Button, enabled, tip));
    response
}

fn paint_icon(painter: &egui::Painter, rect: Rect, icon: Icon, size: f32, color: Color32) {
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        char::from(icon),
        FontId::proportional(size),
        color,
    );
}

/// Borderless toolbar icon button, filled while `active` (an option that is
/// on).
pub fn toolbar_toggle(ui: &mut Ui, icon: Icon, active: bool, tip: &str) -> Response {
    icon_button(
        ui,
        icon,
        Vec2::new(28.0, 24.0),
        16.0,
        Look::Toggle(active),
        true,
        tip,
    )
}

/// Toolbar icon button for an action that may be unavailable.
pub fn toolbar_action(ui: &mut Ui, icon: Icon, enabled: bool, tip: &str) -> Response {
    icon_button(
        ui,
        icon,
        Vec2::new(28.0, 24.0),
        16.0,
        Look::Plain,
        enabled,
        tip,
    )
}

/// Small borderless icon button for inline actions, sized to its glyph.
pub fn small_icon_button(
    ui: &mut Ui,
    icon: Icon,
    glyph_size: f32,
    enabled: bool,
    tip: &str,
) -> Response {
    let size = Vec2::splat((glyph_size + 8.0).ceil());
    icon_button(ui, icon, size, glyph_size, Look::Plain, enabled, tip)
}
