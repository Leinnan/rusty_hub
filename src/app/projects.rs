//! Project commands.

use std::path::Path;

use eframe::egui::{self, Align2, FontId, Order, Rect, Sense};
use unity_hub::{
    hub::{Hub, scan_projects},
    hub_import,
    new_project::init_git,
    unity_project::UnityProject,
};

use super::HubApp;
use crate::{
    commands::ProjectCmd,
    jobs::{JobKind, JobResult},
    platform,
    settings::{self, NewProjectDefaults},
    state::{ChangeVersionForm, Page, ProjectSettingsForm},
    ui::{status_bar::plural, theme::colors},
};

impl HubApp {
    pub(super) fn apply_project(&mut self, ctx: &egui::Context, command: ProjectCmd) {
        match command {
            ProjectCmd::Open(path) => self.open_project(&path, None),
            ProjectCmd::OpenWith { project, editor } => {
                self.open_project(&project, Some(&editor));
            }
            ProjectCmd::Reveal(path) => {
                if let Err(error) = platform::reveal(&path) {
                    self.state
                        .notices
                        .errors
                        .push(format!("Couldn't show {path}: {error}"));
                }
            }
            ProjectCmd::CopyPath(path) => {
                ctx.copy_text(path);
                self.state.notices.toasts.push("Path copied".to_owned());
            }
            ProjectCmd::RequestRemove(path) => {
                if self.state.busy.is_none() {
                    self.state.confirm_remove = Some(path);
                }
            }
            ProjectCmd::ConfirmRemove(path) => self.remove_project(&path),
            ProjectCmd::RequestNew(editor) => self.open_new_project(editor.as_deref()),
            ProjectCmd::PickNewLocation => {
                if let Some(folder) = platform::dialogs::pick_folder()
                    && let Some(form) = &mut self.state.new_project
                {
                    form.location = folder.to_string_lossy().into_owned();
                    form.validate();
                }
            }
            ProjectCmd::CreateNew => self.create_project(),
            ProjectCmd::RetryCatalog => self.state.online.base = None,
            ProjectCmd::DownloadTemplate(package) => self.download_template(ctx, &package),
            ProjectCmd::CancelDownload => {
                if let Some(download) = &self.state.online.download {
                    download
                        .cancel
                        .store(true, std::sync::atomic::Ordering::Relaxed);
                }
            }
            ProjectCmd::ClearDownloads => self.clear_template_downloads(),
            ProjectCmd::RequestSettings(path) => {
                if self.state.busy.is_none()
                    && let Some(project) = self.state.project(&path)
                {
                    let form = ProjectSettingsForm {
                        title: project.custom_title.clone().unwrap_or_default(),
                        launch_args: project.launch_args.clone(),
                        platform: project.launch_platform.clone(),
                        project: path,
                    };
                    self.state.project_settings = Some(form);
                }
            }
            ProjectCmd::SaveSettings(form) => self.save_project_settings(form),
            ProjectCmd::RequestChangeVersion(path) => {
                if self.state.busy.is_none()
                    && let Some(project) = self.state.project(&path)
                    && let Some(editor) = self.state.hub.nearest_editor(&project.version)
                {
                    let editor = editor.exe_path.clone();
                    self.state.change_version = Some(ChangeVersionForm {
                        project: path,
                        editor,
                    });
                }
            }
            ProjectCmd::ChangeVersion { project, editor } => {
                self.change_project_version(&project, &editor);
            }
            ProjectCmd::RequestRemoveMissing => {
                if self.state.busy.is_none() {
                    self.state.confirm_remove_missing = true;
                }
            }
            ProjectCmd::RemoveMissing => self.remove_missing_projects(),
            ProjectCmd::ScanFolder => {
                if self.state.busy.is_none()
                    && let Some(folder) = platform::dialogs::pick_folder()
                {
                    self.start_scan(ctx, folder);
                }
            }
            ProjectCmd::AddFolder(folder) => self.start_scan(ctx, folder),
            ProjectCmd::ImportFromHub => {
                self.jobs.start(JobKind::Import, ctx, || {
                    JobResult::Found(hub_import::import_projects())
                });
            }
        }
    }

    fn open_new_project(&mut self, editor: Option<&str>) {
        if self.state.busy.is_some() {
            return;
        }
        let location = self
            .state
            .new_project_defaults
            .location
            .clone()
            .unwrap_or_else(settings::default_project_location);
        match self.state.new_project_form(editor, location) {
            Some(form) => self.state.new_project = Some(form),
            None => self.state.new_project_blocked = true,
        }
    }

    /// Create what the dialog describes. The dialog stays open on failure, so
    /// nothing typed is lost.
    fn create_project(&mut self) {
        let state = &mut self.state;
        if state.busy.is_some() {
            return;
        }
        let Some(form) = &mut state.new_project else {
            return;
        };
        form.validate();
        if form.problem.is_some() {
            return;
        }
        let form = form.clone();
        let Some(editor) = state
            .hub
            .config
            .editors_configurations
            .iter()
            .find(|editor| editor.exe_path == form.editor)
        else {
            return;
        };
        let template = form.template.as_ref().and_then(|path| {
            let template = editor.templates.iter().find(|t| t.path == *path)?;
            Some(template.label().to_owned())
        });
        match Hub::create_project(editor, &form.request()) {
            Ok(project) => {
                let (title, path) = (project.title.clone(), project.path.clone());
                if form.init_git
                    && let Err(error) = init_git(Path::new(&path))
                {
                    // The project itself is fine, so it stays.
                    state
                        .notices
                        .errors
                        .push(format!("{title} has no git repository: {error}"));
                }
                state.hub.add_projects(vec![project]);
                state.new_project_defaults = NewProjectDefaults {
                    location: Some(form.location),
                    editor: Some(form.editor),
                    template,
                    init_git: form.init_git,
                };
                state.new_project = None;
                state.page = Page::Projects;
                // Keep the new row from being filtered out of sight.
                state.search.clear();
                state.refresh_visible();
                state.selected = Some(path);
                state.scroll_to_selected = true;
                state
                    .notices
                    .toasts
                    .push(format!("Creating {title} in Unity…"));
                self.save_config();
            }
            Err(error) => state
                .notices
                .errors
                .push(format!("Couldn't create {}: {error}", form.name)),
        }
    }

    /// Launch a project, in `editor_path` if given and otherwise the editor
    /// it asks for. Failures become an error toast.
    fn open_project(&mut self, path: &str, editor_path: Option<&str>) {
        let state = &mut self.state;
        let Some(project) = state.hub.projects.iter().find(|p| p.path == path) else {
            return;
        };
        let title = project.title.clone();
        let result = if project.is_valid {
            match editor_path {
                Some(exe) => state
                    .hub
                    .config
                    .editors_configurations
                    .iter()
                    .find(|editor| editor.exe_path == exe)
                    .map_or(Err(std::io::ErrorKind::NotFound.into()), |editor| {
                        Hub::run_project(editor, project)
                    }),
                None => state.hub.open_project(project),
            }
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "the project's folder is gone",
            ))
        };
        match result {
            Ok(()) => state.notices.toasts.push(format!("Opening {title}")),
            Err(error) => state
                .notices
                .errors
                .push(format!("Couldn't open {title}: {error}")),
        }
    }

    fn save_project_settings(&mut self, form: ProjectSettingsForm) {
        if self.state.busy.is_some() {
            return;
        }
        let Some(project) = self
            .state
            .hub
            .projects
            .iter_mut()
            .find(|project| project.path == form.project)
        else {
            return;
        };
        let title = form.title.trim();
        project.custom_title = (!title.is_empty()).then(|| title.to_owned());
        project.launch_args = form.launch_args;
        project.launch_platform = form.platform;
        project.refresh_title();
        self.state.refresh_visible();
        self.save_config();
    }

    /// Point a project at an editor's version and open it there. If the editor
    /// can't be launched, the project goes back to the version it asked for.
    fn change_project_version(&mut self, path: &str, exe: &str) {
        let state = &mut self.state;
        if state.busy.is_some() {
            return;
        }
        let Some(editor) = state
            .hub
            .config
            .editors_configurations
            .iter()
            .find(|editor| editor.exe_path == exe)
        else {
            return;
        };
        let Some(project) = state.hub.projects.iter_mut().find(|p| p.path == path) else {
            return;
        };
        let title = project.title.clone();
        let previous = project.version.clone();
        let result = project.set_version(&editor.version).and_then(|()| {
            Hub::run_project(editor, project).inspect_err(|_| {
                let _ = project.set_version(&previous);
            })
        });
        match result {
            Ok(()) => state
                .notices
                .toasts
                .push(format!("Opening {title} in Unity {}", editor.version)),
            Err(error) => state
                .notices
                .errors
                .push(format!("Couldn't change {title}'s version: {error}")),
        }
        state.refresh_visible();
        self.save_config();
    }

    fn remove_missing_projects(&mut self) {
        if self.state.busy.is_some() {
            return;
        }
        let missing = self.state.hub.missing_projects();
        for path in &missing {
            self.state.hub.remove_project(path);
        }
        self.state.refresh_visible();
        self.save_config();
        self.state
            .notices
            .toasts
            .push(format!("Removed {}", plural(missing.len(), "project")));
    }

    fn remove_project(&mut self, path: &str) {
        let title = self.state.project(path).map(|p| p.title.clone());
        if self.state.busy.is_some() || !self.state.hub.remove_project(path) {
            return;
        }
        self.state.refresh_visible();
        self.save_config();
        if let Some(title) = title {
            self.state
                .notices
                .toasts
                .push(format!("Removed {title} from the list"));
        }
    }

    fn start_scan(&mut self, ctx: &egui::Context, folder: std::path::PathBuf) {
        let known = self.state.hub.projects.clone();
        self.jobs.start(JobKind::Scan, ctx, move || {
            JobResult::Found(scan_projects(&folder, &known))
        });
    }

    pub(super) fn add_found_projects(&mut self, kind: JobKind, projects: Vec<UnityProject>) {
        let added = self.state.hub.add_projects(projects);
        self.state.refresh_visible();
        self.save_config();
        let message = match (added, kind) {
            (0, JobKind::Import) => "No new projects in the Unity Hub".to_owned(),
            (0, _) => "No new projects found".to_owned(),
            (added, JobKind::Import) => format!("Imported {}", plural(added, "project")),
            (added, _) => format!("Added {}", plural(added, "project")),
        };
        self.state.notices.toasts.push(message);
    }

    /// Scan folders dropped on the window.
    pub(super) fn handle_dropped_folders(&mut self, ctx: &egui::Context) {
        let dropped: Vec<_> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .map(|file| file.path().to_path_buf())
                .filter(|path| !path.as_os_str().is_empty())
                .collect()
        });
        for path in dropped {
            if path.is_dir() {
                ProjectCmd::AddFolder(path).schedule();
            } else {
                self.state
                    .notices
                    .errors
                    .push(format!("{} isn't a folder", path.display()));
            }
        }
    }

    /// Dim the page while something is dragged over the window.
    pub(super) fn show_drop_overlay(ctx: &egui::Context, page: Rect) {
        if ctx.input(|i| i.raw.hovered_files.is_empty()) {
            return;
        }
        egui::Area::new(egui::Id::unique("drop_overlay"))
            .order(Order::Foreground)
            .fixed_pos(page.min)
            .show(ctx, |ui| {
                ui.allocate_rect(page, Sense::hover());
                ui.painter().rect_filled(page, 0.0, colors::DROP_OVERLAY);
                ui.painter().text(
                    page.center(),
                    Align2::CENTER_CENTER,
                    "Drop to add projects",
                    FontId::proportional(17.0),
                    colors::TEXT,
                );
            });
    }
}
