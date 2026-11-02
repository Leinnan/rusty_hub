//! Unified macOS-style window toolbar.
//!
//! Layout: traffic-light inset · search | refresh · add · panel toggles.

use eframe::egui::{Align, Id, Layout, Rect, RichText, Sense, TextEdit, Ui, ViewportCommand};

use super::{
    theme::colors::{HINT_TEXT, TEXT},
    widgets::{
        buttons::{toolbar_action, toolbar_toggle},
        icons::{Icon, glyph},
    },
};
use crate::{
    commands::{EditorCmd, ProjectCmd, ViewCmd},
    state::{Page, UiState},
};

/// Toolbar height (px): a compact unified title bar + toolbar on macOS, so
/// the traffic-light buttons line up with the toolbar controls.
pub const TOOLBAR_HEIGHT: f32 = 38.0;

/// Leading inset (px) reserved for the macOS traffic-light buttons when the
/// content view extends under the transparent title bar.
pub const TRAFFIC_LIGHT_INSET: f32 = if cfg!(target_os = "macos") { 84.0 } else { 8.0 };

/// Width of the search box (px).
const SEARCH_WIDTH: f32 = 260.0;

/// The search box's id, which the app needs to tell whether it has focus.
#[must_use]
pub fn search_id() -> Id {
    Id::unique("project_search")
}

/// Render the toolbar contents.
pub fn render(ui: &mut Ui, state: &mut UiState) {
    let full = ui.max_rect();
    handle_window_drag(ui, full);
    ui.horizontal_centered(|ui| {
        ui.add_space(TRAFFIC_LIGHT_INSET);
        if state.page == Page::Projects {
            search_box(ui, state);
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.add_space(8.0);
            if state.page == Page::Projects
                && toolbar_toggle(ui, Icon::PanelRight, state.details_visible, "Details").clicked()
            {
                ViewCmd::ToggleDetails.schedule();
            }
            if toolbar_toggle(ui, Icon::PanelLeft, state.sidebar_visible, "Sidebar").clicked() {
                ViewCmd::ToggleSidebar.schedule();
            }
            ui.separator();
            add_button(ui, state);
            let idle = state.busy.is_none();
            if toolbar_action(ui, Icon::RefreshCw, idle, "Refresh").clicked() {
                ViewCmd::Refresh.schedule();
            }
        });
    });
}

fn search_box(ui: &mut Ui, state: &mut UiState) {
    ui.label(glyph(Icon::Search).size(14.0).color(HINT_TEXT));
    let response = ui.add(
        TextEdit::singleline(&mut state.search)
            .id(search_id())
            .hint_text("Search projects")
            .desired_width(SEARCH_WIDTH),
    );
    if std::mem::take(&mut state.focus_search) {
        response.request_focus();
    }
    if response.changed() {
        state.refresh_visible();
    }
}

fn add_button(ui: &mut Ui, state: &UiState) {
    let enabled = state.busy.is_none();
    ui.add_enabled_ui(enabled, |ui| match state.page {
        Page::Projects => {
            ui.menu_button(icon_text(Icon::Plus, "Add"), |ui| {
                if ui.button("Scan Folder…").clicked() {
                    ProjectCmd::ScanFolder.schedule();
                    ui.close();
                }
                if ui.button("Import from Unity Hub").clicked() {
                    ProjectCmd::ImportFromHub.schedule();
                    ui.close();
                }
            });
        }
        Page::Editors => {
            if ui
                .button(icon_text(Icon::Plus, "Add Search Path…"))
                .clicked()
            {
                EditorCmd::AddSearchPath.schedule();
            }
        }
    });
}

fn icon_text(icon: Icon, text: &str) -> RichText {
    RichText::new(format!("{}  {text}", char::from(icon))).color(TEXT)
}

/// Drag the window by empty toolbar space, as macOS title bars do. Other
/// platforms keep their native title bar for this.
fn handle_window_drag(ui: &Ui, rect: Rect) {
    if !cfg!(target_os = "macos") {
        return;
    }
    let response = ui.interact(
        rect,
        ui.scope_id().with("toolbar_drag"),
        Sense::click_and_drag(),
    );
    if response.drag_started() {
        ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
    }
    if response.double_clicked() {
        let maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
        ui.ctx()
            .send_viewport_cmd(ViewportCommand::Maximized(!maximized));
    }
}
