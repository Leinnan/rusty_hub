//! Project commands.

use eframe::egui::{self, Align2, FontId, Order, Rect, Sense};
use unity_hub::{
    hub::{Hub, scan_projects},
    hub_import,
    unity_project::UnityProject,
};

use super::HubApp;
use crate::{
    commands::ProjectCmd,
    jobs::{JobKind, JobResult},
    platform,
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
