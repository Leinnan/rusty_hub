//! The selected project's details, and the actions a project offers.

use std::time::SystemTime;

use eframe::egui::{self, Align, Button, Grid, Label, Layout, RichText, ScrollArea, Ui};
use unity_hub::unity_project::UnityProject;

use super::{
    format::{abbreviate_home, relative_time},
    theme::colors::{DELETE_RED_SOFT, HINT_TEXT, SUCCESS, TEXT, TEXT_WEAK, WARNING},
    widgets::{
        buttons::{primary_button, secondary_button},
        section::section_header,
    },
};
use crate::{
    commands::ProjectCmd,
    platform::REVEAL_LABEL,
    state::{ProjectStatus, UiState},
};

/// Render the panel contents.
pub fn render(ui: &mut Ui, state: &UiState) {
    ScrollArea::vertical()
        .auto_shrink([false, true])
        .show(ui, |ui| contents(ui, state));
}

fn contents(ui: &mut Ui, state: &UiState) {
    section_header(ui, "Details");
    let Some(project) = state.selected_project() else {
        ui.add_space(4.0);
        ui.label(RichText::new("Select a project to see its details.").color(HINT_TEXT));
        return;
    };
    let status = state.status(project);

    ui.add_space(4.0);
    ui.add(
        Label::new(
            RichText::new(&project.title)
                .size(15.0)
                .strong()
                .color(TEXT),
        )
        .selectable(false),
    );
    ui.add(
        Label::new(
            RichText::new(abbreviate_home(&project.path))
                .small()
                .color(TEXT_WEAK),
        )
        .wrap()
        .selectable(false),
    );
    ui.add_space(8.0);
    facts(ui, project, status);
    ui.add_space(12.0);

    ui.with_layout(Layout::top_down_justified(Align::Center), |ui| {
        let open = ui.add_enabled(status == ProjectStatus::Ready, primary_button("Open"));
        if open.clicked() {
            ProjectCmd::Open(project.path.clone()).schedule();
        }
        ui.add_enabled_ui(project.is_valid, |ui| {
            ui.menu_button("Open in", |ui| open_in_items(ui, state, project));
        });
        if ui.add(secondary_button(REVEAL_LABEL)).clicked() {
            ProjectCmd::Reveal(project.path.clone()).schedule();
        }
        if ui.add(secondary_button("Copy Path")).clicked() {
            ProjectCmd::CopyPath(project.path.clone()).schedule();
        }
        let idle = state.busy.is_none();
        if ui
            .add_enabled(idle, secondary_button("Project Settings…"))
            .clicked()
        {
            ProjectCmd::RequestSettings(project.path.clone()).schedule();
        }
        let can_change = idle && project.is_valid && !state.editors_newest_first().is_empty();
        if ui
            .add_enabled(can_change, secondary_button("Change Version…"))
            .clicked()
        {
            ProjectCmd::RequestChangeVersion(project.path.clone()).schedule();
        }
        ui.add_space(8.0);
        let remove = ui.add_enabled(
            state.busy.is_none(),
            Button::new(RichText::new("Remove from List…").color(DELETE_RED_SOFT)),
        );
        if remove.clicked() {
            ProjectCmd::RequestRemove(project.path.clone()).schedule();
        }
    });
}

fn facts(ui: &mut Ui, project: &UnityProject, status: ProjectStatus) {
    Grid::new("project_facts")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            fact(ui, "Version", RichText::new(&project.version).color(TEXT));
            let (editor, color) = match status {
                ProjectStatus::Ready => ("Installed", SUCCESS),
                ProjectStatus::EditorMissing => ("Not installed", WARNING),
                ProjectStatus::NotFound => ("Folder not found", DELETE_RED_SOFT),
            };
            fact(ui, "Editor", RichText::new(editor).color(color));
            if !project.branch.is_empty() {
                fact(ui, "Branch", RichText::new(&project.branch).color(TEXT));
            }
            if !project.render_pipeline.is_empty() {
                fact(
                    ui,
                    "Pipeline",
                    RichText::new(&project.render_pipeline).color(TEXT),
                );
            }
            let folder = std::path::Path::new(&project.path).file_name();
            if folder.is_some_and(|name| name.to_string_lossy() != project.title) {
                let folder = folder.map(|name| name.to_string_lossy().into_owned());
                fact(
                    ui,
                    "Folder",
                    RichText::new(folder.unwrap_or_default()).color(TEXT),
                );
            }
            if !project.launch_args.is_empty() {
                fact(
                    ui,
                    "Arguments",
                    RichText::new(&project.launch_args).color(TEXT),
                );
            }
            if let Some(platform) = &project.launch_platform {
                fact(ui, "Platform", RichText::new(platform).color(TEXT));
            }
            let modified = relative_time(project.edit_time, SystemTime::now());
            fact(ui, "Modified", RichText::new(modified).color(TEXT));
        });
}

fn fact(ui: &mut Ui, name: &str, value: RichText) {
    ui.label(RichText::new(name).color(TEXT_WEAK));
    ui.add(Label::new(value).wrap().selectable(false));
    ui.end_row();
}

/// A project's right-click menu.
pub fn project_menu(ui: &mut Ui, state: &UiState, project: &UnityProject) {
    let open = ui.add_enabled(
        state.status(project) == ProjectStatus::Ready,
        Button::new("Open"),
    );
    if open.clicked() {
        ProjectCmd::Open(project.path.clone()).schedule();
        ui.close();
    }
    ui.add_enabled_ui(project.is_valid, |ui| {
        ui.menu_button("Open in", |ui| open_in_items(ui, state, project));
    });
    ui.separator();
    if ui.button(REVEAL_LABEL).clicked() {
        ProjectCmd::Reveal(project.path.clone()).schedule();
        ui.close();
    }
    if ui.button("Copy Path").clicked() {
        ProjectCmd::CopyPath(project.path.clone()).schedule();
        ui.close();
    }
    if ui.button("Show Logs").clicked() {
        let logs = std::path::Path::new(&project.path).join("Logs");
        ProjectCmd::Reveal(logs.to_string_lossy().into_owned()).schedule();
        ui.close();
    }
    ui.separator();
    let idle = state.busy.is_none();
    if ui
        .add_enabled(idle, Button::new("Project Settings…"))
        .clicked()
    {
        ProjectCmd::RequestSettings(project.path.clone()).schedule();
        ui.close();
    }
    let can_change = idle && project.is_valid && !state.editors_newest_first().is_empty();
    if ui
        .add_enabled(can_change, Button::new("Change Version…"))
        .clicked()
    {
        ProjectCmd::RequestChangeVersion(project.path.clone()).schedule();
        ui.close();
    }
    ui.separator();
    let remove = ui.add_enabled(
        state.busy.is_none(),
        Button::new(RichText::new("Remove from List…").color(DELETE_RED_SOFT)),
    );
    if remove.clicked() {
        ProjectCmd::RequestRemove(project.path.clone()).schedule();
        ui.close();
    }
}

/// Every installed editor, with the one the project asks for marked.
fn open_in_items(ui: &mut Ui, state: &UiState, project: &UnityProject) {
    let editors = &state.hub.config.editors_configurations;
    if editors.is_empty() {
        ui.add_enabled(false, Button::new("No editors found"));
        return;
    }
    let wanted = state.hub.editor_for_project(project);
    for editor in editors {
        let mut text = RichText::new(&editor.version);
        if wanted == Some(editor) {
            text = text.strong().color(SUCCESS);
        }
        if ui
            .add(Button::new(text).wrap_mode(egui::TextWrapMode::Extend))
            .clicked()
        {
            ProjectCmd::OpenWith {
                project: project.path.clone(),
                editor: editor.exe_path.clone(),
            }
            .schedule();
            ui.close();
        }
    }
}
