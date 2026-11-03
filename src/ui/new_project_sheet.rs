//! The New Project dialog.

use eframe::egui::{
    self, Align, ComboBox, Key, Label, Layout, Popup, RichText, ScrollArea, TextEdit, Ui,
};
use std::sync::atomic::Ordering;

use unity_hub::{
    new_project::build_target, online_templates::OnlineTemplate, project_template::ProjectTemplate,
};

use super::{
    format::abbreviate_home,
    theme::colors::{DELETE_RED_SOFT, HINT_TEXT, TEXT, TEXT_WEAK},
    widgets::{
        buttons::{primary_button, secondary_button},
        segmented::segmented_control,
    },
};
use crate::{
    commands::{ProjectCmd, ViewCmd},
    state::{CatalogStatus, NewProjectForm, Page, TemplateSource, UiState},
};

/// Width of the sheet (px).
const WIDTH: f32 = 480.0;
/// Room kept around the sheet on short windows (px).
const WINDOW_MARGIN: f32 = 90.0;
/// Tallest the template list gets (px).
const TEMPLATES_HEIGHT: f32 = 190.0;
/// Which templates each switch position lists, in order.
const SOURCES: [TemplateSource; 2] = [TemplateSource::Installed, TemplateSource::Online];
/// Bytes in a megabyte, for download progress.
const MEGABYTE: f64 = 1_000_000.0;
/// Width of the field labels (px).
const LABEL_WIDTH: f32 = 72.0;

/// Show the dialog if it's open.
pub fn render(ctx: &egui::Context, state: &mut UiState) {
    if state.new_project_blocked {
        render_no_editors(ctx, state);
    }
    let Some(mut form) = state.new_project.take() else {
        return;
    };
    let mut close = false;
    let frame = egui::Frame::window(&ctx.global_style()).inner_margin(egui::Margin::same(16));
    let modal = egui::Modal::new(egui::Id::unique("new_project")).frame(frame);
    let response = modal.show(ctx, |ui| {
        ui.set_width(WIDTH);
        ui.label(RichText::new("New Project").heading().color(TEXT));
        ui.add_space(10.0);
        let height = (ctx.content_rect().height() - WINDOW_MARGIN * 2.0).max(120.0);
        ScrollArea::vertical()
            .max_height(height)
            .auto_shrink([false, true])
            .show(ui, |ui| fields(ui, state, &mut form));
        ui.add_space(10.0);
        close = actions(ui, state, &form);
    });
    if !close && !response.should_close() {
        state.new_project = Some(form);
    }
}

fn render_no_editors(ctx: &egui::Context, state: &mut UiState) {
    let frame = egui::Frame::window(&ctx.global_style()).inner_margin(egui::Margin::same(16));
    let modal = egui::Modal::new(egui::Id::unique("new_project_blocked")).frame(frame);
    let mut close = false;
    let response = modal.show(ctx, |ui| {
        ui.set_width(340.0);
        ui.label(RichText::new("No Unity editors found").heading().color(TEXT));
        ui.label(
            RichText::new("Creating a project needs an installed editor. Add the folder that holds yours on the Editors page.")
                .color(TEXT_WEAK),
        );
        ui.add_space(10.0);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui.add(primary_button("Go to Editors")).clicked() {
                ViewCmd::ShowPage(Page::Editors).schedule();
                close = true;
            }
            if ui.add(secondary_button("Cancel")).clicked() {
                close = true;
            }
        });
    });
    if close || response.should_close() {
        state.new_project_blocked = false;
    }
}

/// A field's name. Pass its id to `labelled_by`, so assistive tech can tell
/// what the field is.
fn field_label(ui: &mut Ui, text: &str) -> egui::Id {
    ui.allocate_ui_with_layout(
        egui::vec2(LABEL_WIDTH, ui.spacing().interact_size.y),
        Layout::left_to_right(Align::Center),
        |ui| {
            ui.set_width(LABEL_WIDTH);
            ui.add(Label::new(RichText::new(text).color(TEXT_WEAK)).selectable(false))
                .id
        },
    )
    .inner
}

fn fields(ui: &mut Ui, state: &UiState, form: &mut NewProjectForm) {
    ui.spacing_mut().item_spacing.y = 8.0;
    ui.horizontal(|ui| {
        let label = field_label(ui, "Name");
        let name = ui
            .add(
                TextEdit::singleline(&mut form.name)
                    .hint_text("Project name")
                    .desired_width(f32::INFINITY),
            )
            .labelled_by(label);
        if std::mem::take(&mut form.focus_name) {
            name.request_focus();
        }
        if name.changed() {
            form.name_edited = true;
            form.validate();
        }
    });
    ui.horizontal(|ui| {
        let label = field_label(ui, "Location");
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui.add(secondary_button("Choose…")).clicked() {
                ProjectCmd::PickNewLocation.schedule();
            }
            let location = ui
                .add(
                    TextEdit::singleline(&mut form.location)
                        .hint_text("Folder")
                        .desired_width(f32::INFINITY),
                )
                .labelled_by(label);
            if location.changed() {
                form.validate();
            }
        });
    });
    editor_field(ui, state, form);
    platform_field(ui, state, form);
    template_field(ui, state, form);
    if state.git_available == Some(true) {
        ui.horizontal(|ui| {
            ui.add_space(LABEL_WIDTH + ui.spacing().item_spacing.x);
            ui.checkbox(&mut form.init_git, "Initialize git repository");
        });
    }

    ui.add_space(2.0);
    if let Some(problem) = &form.problem {
        ui.label(RichText::new(problem).color(DELETE_RED_SOFT));
    } else {
        let path = form.path().to_string_lossy().into_owned();
        ui.add(
            Label::new(
                RichText::new(abbreviate_home(&path))
                    .small()
                    .color(HINT_TEXT),
            )
            .truncate()
            .selectable(false),
        );
    }
}

fn editor_field(ui: &mut Ui, state: &UiState, form: &mut NewProjectForm) {
    let editors = state.editors_newest_first();
    let chosen = state
        .editor(&form.editor)
        .map_or("", |editor| editor.version.as_str())
        .to_owned();
    ui.horizontal(|ui| {
        let label = field_label(ui, "Editor");
        ComboBox::from_id_salt("new_project_editor")
            .selected_text(chosen)
            .width(ui.available_width())
            .show_ui(ui, |ui| {
                for editor in editors {
                    let selected = form.editor == editor.exe_path;
                    if ui.selectable_label(selected, &editor.version).clicked() {
                        form.set_editor(editor);
                    }
                }
            })
            .response
            .labelled_by(label);
    });
}

fn platform_field(ui: &mut Ui, state: &UiState, form: &mut NewProjectForm) {
    let platforms: Vec<&String> = state
        .editor(&form.editor)
        .into_iter()
        .flat_map(|editor| &editor.platforms)
        .filter(|platform| build_target(platform).is_some())
        .collect();
    ui.horizontal(|ui| {
        let label = field_label(ui, "Platform");
        ComboBox::from_id_salt("new_project_platform")
            .selected_text(form.platform.as_deref().unwrap_or("Default"))
            .width(ui.available_width())
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(form.platform.is_none(), "Default")
                    .clicked()
                {
                    form.platform = None;
                }
                for platform in platforms {
                    let selected = form.platform.as_ref() == Some(platform);
                    if ui.selectable_label(selected, platform.as_str()).clicked() {
                        form.platform = Some(platform.clone());
                    }
                }
            })
            .response
            .labelled_by(label);
    });
}

fn template_field(ui: &mut Ui, state: &UiState, form: &mut NewProjectForm) {
    ui.horizontal_top(|ui| {
        field_label(ui, "Template");
        ui.vertical(|ui| {
            let selected = SOURCES.iter().position(|&source| source == form.source);
            let segments =
                segmented_control(ui, "template_source", &["Installed", "Online"], selected);
            for (segment, source) in segments.iter().zip(SOURCES) {
                if segment.clicked() && form.source != source {
                    form.source = source;
                    // A choice from the other list wouldn't be visible.
                    form.template = None;
                }
            }
            ScrollArea::vertical()
                .id_salt("new_project_templates")
                .max_height(TEMPLATES_HEIGHT)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    match form.source {
                        TemplateSource::Installed => installed_list(ui, state, form),
                        TemplateSource::Online => online_list(ui, state, form),
                    }
                });
        });
    });
}

fn installed_list(ui: &mut Ui, state: &UiState, form: &mut NewProjectForm) {
    let templates = state
        .editor(&form.editor)
        .map_or(&[][..], |editor| editor.templates.as_slice());
    if template_row(
        ui,
        "Empty",
        Some("Unity's default empty project."),
        form.template.is_none(),
    ) {
        form.template = None;
    }
    for template in templates {
        if installed_row(ui, template, form) {
            form.template = Some(template.path.clone());
        }
    }
}

fn online_list(ui: &mut Ui, state: &UiState, form: &mut NewProjectForm) {
    match &state.online.status {
        CatalogStatus::Idle | CatalogStatus::Loading => {
            ui.label(RichText::new("Asking Unity for its templates…").color(TEXT_WEAK));
        }
        CatalogStatus::Failed(message) => {
            ui.label(
                RichText::new(format!("Couldn't list online templates: {message}"))
                    .color(DELETE_RED_SOFT),
            );
            if ui.add(secondary_button("Try Again")).clicked() {
                ProjectCmd::RetryCatalog.schedule();
            }
        }
        CatalogStatus::Loaded(templates) => {
            if templates.is_empty() {
                ui.label(
                    RichText::new("Unity lists no templates for this editor.").color(TEXT_WEAK),
                );
            }
            for template in templates {
                online_row(ui, state, form, template);
            }
            ui.add_space(6.0);
            let idle = state.online.download.is_none();
            if ui
                .add_enabled(idle, secondary_button("Clear Downloaded Templates"))
                .clicked()
            {
                ProjectCmd::ClearDownloads.schedule();
            }
        }
    }
}

/// A catalog entry: pick it if it's downloaded, otherwise offer to download.
fn online_row(ui: &mut Ui, state: &UiState, form: &mut NewProjectForm, template: &OnlineTemplate) {
    let online = &state.online;
    let cached = online
        .cache_dir
        .as_ref()
        .map(|dir| template.cached_path(dir));
    // Asks the disk, but only for a short list in a dialog that's open.
    let path = cached.filter(|path| path.is_file());
    let path_text = path
        .as_ref()
        .map(|path| path.to_string_lossy().into_owned());
    let selected = path_text.is_some() && form.template == path_text;
    let summary = format!(
        "{} · {} · {}",
        title_case(&template.kind),
        template.render_pipeline.replace('_', "-"),
        template.version
    );
    let downloading = online
        .download
        .as_ref()
        .filter(|download| download.package == template.package_name);

    ui.horizontal(|ui| {
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if path_text.is_none() {
                if let Some(download) = downloading {
                    if ui.add(secondary_button("Cancel Download")).clicked() {
                        ProjectCmd::CancelDownload.schedule();
                    }
                    let bytes = download.progress.load(Ordering::Relaxed) as f64;
                    ui.label(
                        RichText::new(format!("{:.1} MB", bytes / MEGABYTE))
                            .small()
                            .color(TEXT_WEAK),
                    );
                } else {
                    let idle = online.download.is_none() && online.cache_dir.is_some();
                    let tip = format!("Download {}", template.name);
                    if ui
                        .add_enabled(idle, secondary_button("Download"))
                        .on_hover_text(tip)
                        .clicked()
                    {
                        ProjectCmd::DownloadTemplate(template.package_name.clone()).schedule();
                    }
                }
            }
            ui.with_layout(Layout::top_down_justified(Align::Min), |ui| {
                if path_text.is_some() {
                    if ui
                        .selectable_label(selected, RichText::new(&template.name).color(TEXT))
                        .clicked()
                    {
                        form.template.clone_from(&path_text);
                    }
                } else {
                    ui.add(
                        Label::new(RichText::new(&template.name).color(TEXT))
                            .truncate()
                            .selectable(false),
                    );
                }
                ui.add(
                    Label::new(RichText::new(&summary).small().color(HINT_TEXT))
                        .truncate()
                        .selectable(false),
                );
            });
        });
    });
    if selected && !template.description.is_empty() {
        ui.add(
            Label::new(
                RichText::new(&template.description)
                    .small()
                    .color(TEXT_WEAK),
            )
            .wrap()
            .selectable(false),
        );
    }
    if let Some((package, message)) = &online.download_error
        && *package == template.package_name
    {
        ui.label(RichText::new(message).small().color(DELETE_RED_SOFT));
    }
    ui.add_space(4.0);
}

/// `LEARNING` as `Learning`.
fn title_case(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_string() + &chars.as_str().to_lowercase()
    })
}

fn installed_row(ui: &mut Ui, template: &ProjectTemplate, form: &NewProjectForm) -> bool {
    let selected = form.template.as_ref() == Some(&template.path);
    template_row(
        ui,
        template.label(),
        template.description.as_deref(),
        selected,
    )
}

/// A row of the template list. Returns whether it was clicked.
fn template_row(ui: &mut Ui, label: &str, description: Option<&str>, selected: bool) -> bool {
    let clicked = ui
        .with_layout(Layout::top_down_justified(Align::Min), |ui| {
            ui.selectable_label(selected, RichText::new(label).color(TEXT))
        })
        .inner
        .clicked();
    if let Some(description) = description.filter(|_| selected) {
        ui.add(
            Label::new(RichText::new(description).small().color(TEXT_WEAK))
                .wrap()
                .selectable(false),
        );
    }
    clicked
}

/// The Create and Cancel buttons. Returns whether to close without creating.
fn actions(ui: &mut Ui, state: &UiState, form: &NewProjectForm) -> bool {
    let ready = form.problem.is_none() && state.busy.is_none();
    let enter = ui.input(|i| i.key_pressed(Key::Enter)) && !Popup::is_any_open(ui.ctx());
    let mut close = false;
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        let create = ui.add_enabled(ready, primary_button("Create"));
        if create.clicked() || (enter && ready) {
            ProjectCmd::CreateNew.schedule();
        }
        if ui.add(secondary_button("Cancel")).clicked() {
            close = true;
        }
    });
    close
}
