//! Visuals, spacing and fonts for the macOS-style dark look.

pub mod colors;
pub mod fonts;

use eframe::egui::{self, Context, CornerRadius, FontFamily, FontId, TextStyle};

use colors::{
    ACCENT_BLUE, BORDER, BORDER_SUBTLE, CARD_BG, CARD_BG_HOVER, CONTROL_TRACK, EXTREME_BG,
    FAINT_BG, HINT_TEXT, PANEL_BG, SELECTION_BG, TEXT, WINDOW_BG,
};

/// Install the dark theme on `ctx`.
pub fn configure_theme(ctx: &Context) {
    ctx.set_theme(egui::Theme::Dark);

    let mut visuals = egui::Visuals::dark();
    // `override_text_color` stays unset so hint text keeps `weak_text_color`
    // and reads dimmer than typed text.
    for widget in [
        &mut visuals.widgets.noninteractive,
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        widget.fg_stroke.color = TEXT;
    }
    visuals.weak_text_color = Some(HINT_TEXT);
    visuals.panel_fill = PANEL_BG;
    visuals.window_fill = WINDOW_BG;
    visuals.faint_bg_color = FAINT_BG;
    visuals.extreme_bg_color = EXTREME_BG;
    visuals.selection.bg_fill = SELECTION_BG;
    visuals.selection.stroke.color = ACCENT_BLUE;
    visuals.widgets.active.bg_fill = ACCENT_BLUE;
    visuals.widgets.active.weak_bg_fill = ACCENT_BLUE;
    visuals.widgets.active.bg_stroke.color = ACCENT_BLUE;
    visuals.widgets.hovered.bg_fill = CARD_BG_HOVER;
    visuals.widgets.hovered.weak_bg_fill = CARD_BG_HOVER;
    visuals.widgets.hovered.bg_stroke.color = BORDER;
    visuals.widgets.open.weak_bg_fill = CARD_BG_HOVER;
    visuals.widgets.noninteractive.bg_fill = CARD_BG;
    visuals.widgets.noninteractive.bg_stroke.color = BORDER_SUBTLE;
    // `bg_fill` paints slider rails and checkbox wells, `weak_bg_fill`
    // button faces.
    visuals.widgets.inactive.bg_fill = CONTROL_TRACK;
    visuals.widgets.inactive.weak_bg_fill = CARD_BG;
    visuals.widgets.inactive.bg_stroke.color = BORDER;
    visuals.widgets.inactive.bg_stroke.width = 1.0;
    visuals.window_stroke.color = BORDER;
    visuals.window_stroke.width = 1.0;
    visuals.window_shadow = egui::Shadow {
        offset: [0, 12],
        blur: 40,
        spread: 0,
        color: egui::Color32::from_black_alpha(140),
    };
    visuals.popup_shadow = egui::Shadow {
        offset: [0, 6],
        blur: 18,
        spread: 0,
        color: egui::Color32::from_black_alpha(110),
    };
    ctx.set_visuals(visuals);

    let mut style = (*ctx.global_style()).clone();
    // macOS text sizes: body 13 pt, small/caption 11 pt, headline 15 pt.
    style.text_styles = [
        (
            TextStyle::Small,
            FontId::new(11.0, FontFamily::Proportional),
        ),
        (TextStyle::Body, FontId::new(13.0, FontFamily::Proportional)),
        (
            TextStyle::Button,
            FontId::new(13.0, FontFamily::Proportional),
        ),
        (
            TextStyle::Heading,
            FontId::new(15.0, FontFamily::Proportional),
        ),
        (
            TextStyle::Monospace,
            FontId::new(12.0, FontFamily::Monospace),
        ),
    ]
    .into();
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(8.0, 3.0);
    style.spacing.interact_size.y = 22.0;
    style.spacing.window_margin = egui::Margin::same(12);
    style.spacing.menu_margin = egui::Margin::same(5);
    style.interaction.selectable_labels = false;
    // Controls get the macOS 6 px radius; windows/sheets a softer one.
    style.visuals.widgets.active.corner_radius = CornerRadius::same(6);
    style.visuals.widgets.hovered.corner_radius = CornerRadius::same(6);
    style.visuals.widgets.inactive.corner_radius = CornerRadius::same(6);
    style.visuals.widgets.open.corner_radius = CornerRadius::same(6);
    style.visuals.widgets.noninteractive.corner_radius = CornerRadius::same(10);
    style.visuals.menu_corner_radius = CornerRadius::same(8);
    style.visuals.window_corner_radius = CornerRadius::same(12);
    ctx.set_global_style(style);
}
