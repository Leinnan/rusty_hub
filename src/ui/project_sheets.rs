//! Smaller dialogs about projects: their settings, moving one to another
//! Unity version, and clearing out missing ones.

use eframe::egui::{self, Align, ComboBox, Layout, RichText, TextEdit, Ui};
use unity_hub::{new_project::build_target, unity_project::parse_launch_args};

use super::{
    theme::colors::{DELETE_RED_SOFT, TEXT, TEXT_WEAK, WARNING},
    widgets::buttons::{destructive_button, primary_button, secondary_button},
};
use crate::{
    commands::ProjectCmd,
    state::{ProjectSettingsForm, UiState},
};

/// Width of a sheet (px).
const WIDTH: f32 = 400.0;

/// Show whichever of these is open.
pub fn render(ctx: &egui::Context, state: &mut UiState) {
    project_settings(ctx, state);
    change_version(ctx, state);
    remove_missing(ctx, state);
}

/// A sheet with `body`. Returns whether it asked to close.
fn sheet(ctx: &egui::Context, id: &str, body: impl FnOnce(&mut Ui) -> bool) -> bool {
    let frame = egui::Frame::window(&ctx.global_style()).inner_margin(egui::Margin::same(16));
    let modal = egui::Modal::new(egui::Id::unique(id)).frame(frame);
    let response = modal.show(ctx, |ui| {
        ui.set_width(WIDTH);
        body(ui)
    });
    response.inner || response.should_close()
}

fn project_settings(ctx: &egui::Context, state: &mut UiState) {
    let Some(mut form) = state.project_settings.take() else {
        return;
    };
    let platforms: Vec<String> = state
        .project(&form.project)
        .and_then(|project| state.hub.editor_for_project(project))
        .map(|editor| editor.platforms.clone())
        .unwrap_or_default();
    let idle = state.busy.is_none();
    let close = sheet(ctx, "project_settings", |ui| {
        settings_body(ui, &mut form, &platforms, idle)
    });
    if !close {
        state.project_settings = Some(form);
    }
}

fn settings_body(
    ui: &mut Ui,
    form: &mut ProjectSettingsForm,
    platforms: &[String],
    idle: bool,
) -> bool {
    ui.label(RichText::new("Project Settings").heading().color(TEXT));
    ui.add_space(8.0);
    let label = ui.label(RichText::new("Title").color(TEXT_WEAK));
    ui.add(
        TextEdit::singleline(&mut form.title)
            .hint_text("The project's own name")
            .desired_width(f32::INFINITY),
    )
    .labelled_by(label.id);
    ui.add_space(4.0);
    let label = ui.label(RichText::new("Launch arguments").color(TEXT_WEAK));
    ui.add(
        TextEdit::singleline(&mut form.launch_args)
            .hint_text("-force-metal")
            .desired_width(f32::INFINITY),
    )
    .labelled_by(label.id);
    let problem = parse_launch_args(&form.launch_args).err();
    if let Some(problem) = problem {
        ui.label(RichText::new(problem).small().color(DELETE_RED_SOFT));
    }
    ui.add_space(4.0);
    let label = ui.label(RichText::new("Platform").color(TEXT_WEAK));
    ComboBox::from_id_salt("project_platform")
        .selected_text(form.platform.as_deref().unwrap_or("Default"))
        .width(ui.available_width())
        .show_ui(ui, |ui| {
            if ui
                .selectable_label(form.platform.is_none(), "Default")
                .clicked()
            {
                form.platform = None;
            }
            for platform in platforms.iter().filter(|p| build_target(p).is_some()) {
                let selected = form.platform.as_ref() == Some(platform);
                if ui.selectable_label(selected, platform.as_str()).clicked() {
                    form.platform = Some(platform.clone());
                }
            }
        })
        .response
        .labelled_by(label.id);
    ui.add_space(10.0);
    let mut close = false;
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        let ready = problem.is_none() && idle;
        if ui.add_enabled(ready, primary_button("Save")).clicked() {
            ProjectCmd::SaveSettings(form.clone()).schedule();
            close = true;
        }
        if ui.add(secondary_button("Cancel")).clicked() {
            close = true;
        }
    });
    close
}

fn change_version(ctx: &egui::Context, state: &mut UiState) {
    let Some(mut form) = state.change_version.take() else {
        return;
    };
    let Some(project) = state.project(&form.project) else {
        return;
    };
    let (title, current) = (project.title.clone(), project.version.clone());
    let editors = state.editors_newest_first();
    let chosen = state.editor(&form.editor).map(|e| e.version.clone());
    let idle = state.busy.is_none();
    let close = sheet(ctx, "change_version", |ui| {
        ui.label(
            RichText::new(format!("Change “{title}”'s Unity version"))
                .heading()
                .color(TEXT),
        );
        ui.label(RichText::new(format!("It's on {current}.")).color(TEXT_WEAK));
        ui.add_space(8.0);
        let label = ui.label(RichText::new("Open with").color(TEXT_WEAK));
        ComboBox::from_id_salt("change_version_editor")
            .selected_text(chosen.clone().unwrap_or_default())
            .width(ui.available_width())
            .show_ui(ui, |ui| {
                for editor in &editors {
                    let selected = form.editor == editor.exe_path;
                    if ui.selectable_label(selected, &editor.version).clicked() {
                        form.editor.clone_from(&editor.exe_path);
                    }
                }
            })
            .response
            .labelled_by(label.id);
        ui.add_space(8.0);
        ui.label(
            RichText::new(
                "Unity converts the project when it opens, and it usually can't go back. \
                 Commit or back up first.",
            )
            .color(WARNING),
        );
        ui.add_space(10.0);
        let mut close = false;
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let changes = chosen.as_deref().is_some_and(|version| version != current);
            let ready = changes && idle;
            if ui
                .add_enabled(ready, primary_button("Change and Open"))
                .clicked()
            {
                ProjectCmd::ChangeVersion {
                    project: form.project.clone(),
                    editor: form.editor.clone(),
                }
                .schedule();
                close = true;
            }
            if ui.add(secondary_button("Cancel")).clicked() {
                close = true;
            }
        });
        close
    });
    drop(editors);
    if !close {
        state.change_version = Some(form);
    }
}

fn remove_missing(ctx: &egui::Context, state: &mut UiState) {
    if !state.confirm_remove_missing {
        return;
    }
    let missing = state.hub.missing_projects();
    let count = super::status_bar::plural(missing.len(), "project");
    let close = sheet(ctx, "remove_missing", |ui| {
        ui.label(
            RichText::new(format!("Remove {count} whose folders are gone?"))
                .heading()
                .color(TEXT),
        );
        for path in missing.iter().take(5) {
            ui.label(RichText::new(path).small().color(TEXT_WEAK));
        }
        if missing.len() > 5 {
            ui.label(
                RichText::new(format!("and {} more", missing.len() - 5))
                    .small()
                    .color(TEXT_WEAK),
            );
        }
        ui.add_space(10.0);
        let mut close = false;
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui.add(destructive_button("Remove")).clicked() {
                ProjectCmd::RemoveMissing.schedule();
                close = true;
            }
            if ui.add(secondary_button("Cancel")).clicked() {
                close = true;
            }
        });
        close
    });
    if close {
        state.confirm_remove_missing = false;
    }
}
