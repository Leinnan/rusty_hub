//! Search paths and installed editors.

use eframe::egui::{
    Align, CornerRadius, Label, Layout, Response, RichText, Role, ScrollArea, Sense, Ui, UiBuilder,
    Vec2, WidgetInfo, vec2,
};
use unity_hub::unity_editor::UnityEditor;

use super::{
    format::abbreviate_home,
    theme::colors::{CARD_BG, CARD_BG_HOVER, HINT_TEXT, TEXT, TEXT_WEAK},
    widgets::{
        buttons::small_icon_button,
        chip::chip,
        icons::{Icon, glyph},
        section::section_header,
    },
};
use crate::{commands::EditorCmd, platform::REVEAL_LABEL, state::UiState};

/// Height of a search path row (px).
const PATH_ROW_HEIGHT: f32 = 36.0;
/// Height of an editor row (px).
const EDITOR_ROW_HEIGHT: f32 = 52.0;
/// Room a platform chip takes on average, for deciding how many fit (px).
const CHIP_SLOT: f32 = 66.0;
/// Room an editor row keeps for its version and path (px).
const EDITOR_TEXT_WIDTH: f32 = 260.0;

/// Render the page.
pub fn render(ui: &mut Ui, state: &UiState) {
    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 4.0;
            section_header(ui, "Search paths");
            let idle = state.busy.is_none();
            if state.hub.config.unity_search_paths.is_empty() {
                hint(
                    ui,
                    "No search paths. Add the folder that holds your Unity installs.",
                );
            }
            for path in &state.hub.config.unity_search_paths {
                search_path_row(ui, path, idle);
            }

            ui.add_space(16.0);
            section_header(ui, "Installed editors");
            let editors = &state.hub.config.editors_configurations;
            if editors.is_empty() {
                hint(ui, "No editors found in the search paths.");
            }
            for editor in editors {
                let response = editor_row(ui, editor);
                response.context_menu(|ui| {
                    if ui.button(REVEAL_LABEL).clicked() {
                        EditorCmd::Reveal(editor.base_path.clone()).schedule();
                        ui.close();
                    }
                });
            }
        });
}

fn hint(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).color(HINT_TEXT));
}

/// A rounded card of `height` across the available width, and a right-to-left
/// `Ui` inside it.
fn card(ui: &mut Ui, height: f32, sense: Sense) -> (Response, Ui) {
    let size = Vec2::new(ui.available_width(), height);
    let (rect, response) = ui.allocate_exact_size(size, sense);
    let fill = if sense.senses_click() && response.hovered() {
        CARD_BG_HOVER
    } else {
        CARD_BG
    };
    ui.painter().rect_filled(rect, CornerRadius::same(8), fill);
    let inner = ui.new_child(
        UiBuilder::new()
            .max_rect(rect.shrink2(vec2(12.0, 4.0)))
            .layout(Layout::right_to_left(Align::Center)),
    );
    (response, inner)
}

fn search_path_row(ui: &mut Ui, path: &str, idle: bool) {
    let (_, mut row) = card(ui, PATH_ROW_HEIGHT, Sense::hover());
    let tip = format!("Remove {path}");
    if small_icon_button(&mut row, Icon::Trash2, 14.0, idle, &tip).clicked() {
        EditorCmd::RemoveSearchPath(path.to_owned()).schedule();
    }
    row.with_layout(Layout::left_to_right(Align::Center), |ui| {
        ui.label(glyph(Icon::Folder).size(14.0).color(TEXT_WEAK));
        ui.add(
            Label::new(RichText::new(abbreviate_home(path)).color(TEXT))
                .truncate()
                .selectable(false),
        );
    });
}

fn editor_row(ui: &mut Ui, editor: &UnityEditor) -> Response {
    let (response, mut row) = card(ui, EDITOR_ROW_HEIGHT, Sense::click());
    response.widget_info(|| WidgetInfo::labeled(Role::Group, true, &editor.version));
    let reveal = small_icon_button(&mut row, Icon::FolderOpen, 14.0, true, REVEAL_LABEL);
    if reveal.clicked() {
        EditorCmd::Reveal(editor.base_path.clone()).schedule();
    }
    platform_chips(&mut row, &editor.platforms);
    row.with_layout(Layout::top_down(Align::Min), |ui| {
        ui.spacing_mut().item_spacing.y = 2.0;
        ui.add_space(3.0);
        ui.add(
            Label::new(
                RichText::new(&editor.version)
                    .size(13.5)
                    .strong()
                    .color(TEXT),
            )
            .truncate()
            .selectable(false),
        );
        ui.add(
            Label::new(
                RichText::new(abbreviate_home(&editor.base_path))
                    .small()
                    .color(HINT_TEXT),
            )
            .truncate()
            .selectable(false),
        );
    });
    response
}

/// As many platform chips as fit, then a "+N" chip naming the rest. The row
/// lays out right to left, so they go on in reverse.
fn platform_chips(ui: &mut Ui, platforms: &[String]) {
    let room = (ui.available_width() - EDITOR_TEXT_WIDTH).max(0.0);
    let fit = ((room / CHIP_SLOT) as usize).min(platforms.len());
    let (shown, rest) = if fit < platforms.len() {
        // Leave a slot for the "+N" chip.
        platforms.split_at(fit.saturating_sub(1))
    } else {
        (platforms, &[][..])
    };
    if !rest.is_empty() {
        let more = format!("+{}", rest.len());
        chip(ui, None, &more, &format!("{more} more platforms")).on_hover_text(rest.join(", "));
    }
    for platform in shown.iter().rev() {
        chip(ui, None, platform, platform);
    }
}
