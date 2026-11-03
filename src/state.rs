//! What the UI shows: the library plus page, selection, search and sort.

use std::cmp::Reverse;
use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64},
};

use serde::{Deserialize, Serialize};
use unity_hub::{
    hub::Hub,
    new_project::{NewProject, build_target, name_problem, suggest_name},
    online_templates::OnlineTemplate,
    unity_editor::{UnityEditor, version_key},
    unity_project::UnityProject,
};

use crate::{
    jobs::JobKind,
    settings::{NewProjectDefaults, Settings},
};

/// A page of the main window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    /// The project list.
    Projects,
    /// Search paths and installed editors.
    Editors,
}

/// The order of the project list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SortKey {
    /// Most recently modified first.
    #[default]
    Recent,
    /// Alphabetical by title.
    Name,
    /// Newest Unity version first.
    Version,
}

impl SortKey {
    /// Every order, as the sort control lists them.
    pub const ALL: [Self; 3] = [Self::Recent, Self::Name, Self::Version];

    /// The name the sort control shows.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Recent => "Recent",
            Self::Name => "Name",
            Self::Version => "Version",
        }
    }
}

/// Whether a project can be opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectStatus {
    /// The folder exists and the editor it asks for is installed.
    Ready,
    /// The editor it asks for isn't installed.
    EditorMissing,
    /// The folder is gone or isn't a project any more.
    NotFound,
}

/// Messages waiting to become toasts.
#[derive(Debug, Default)]
pub struct Notices {
    /// Confirmations, which fade.
    pub toasts: Vec<String>,
    /// Failures, which stay until dismissed.
    pub errors: Vec<String>,
}

/// Which templates the New Project dialog lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TemplateSource {
    /// Those shipped with the editor.
    #[default]
    Installed,
    /// Unity's online catalog.
    Online,
}

/// Where the online catalog has got to.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum CatalogStatus {
    /// Not asked for.
    #[default]
    Idle,
    /// On its way.
    Loading,
    /// Here.
    Loaded(Vec<OnlineTemplate>),
    /// It couldn't be read, and why.
    Failed(String),
}

/// A download on its way.
#[derive(Debug, Clone)]
pub struct TemplateDownload {
    /// The package it's for.
    pub package: String,
    /// Bytes so far.
    pub progress: Arc<AtomicU64>,
    /// Set to stop it.
    pub cancel: Arc<AtomicBool>,
}

/// What the dialog shows about online templates. Kept while the app runs, so
/// reopening the dialog doesn't ask again.
#[derive(Debug, Clone, Default)]
pub struct OnlineState {
    /// The editor base version the catalog is for.
    pub base: Option<String>,
    /// The catalog.
    pub status: CatalogStatus,
    /// The download running, if any.
    pub download: Option<TemplateDownload>,
    /// The last download's failure, by package.
    pub download_error: Option<(String, String)>,
    /// Where downloads go. Without one, nothing can be downloaded.
    pub cache_dir: Option<PathBuf>,
}

/// What the New Project dialog has filled in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewProjectForm {
    /// The project's name.
    pub name: String,
    /// The user typed the name, so it's no longer ours to change.
    pub name_edited: bool,
    /// The folder to create it in.
    pub location: String,
    /// The editor's executable path.
    pub editor: String,
    /// The template archive's path, or `None` for an empty project.
    pub template: Option<String>,
    /// Which templates are listed.
    pub source: TemplateSource,
    /// The platform to start on, or `None` for the editor's default.
    pub platform: Option<String>,
    /// Start a git repository too.
    pub init_git: bool,
    /// Why the project can't be created as it stands. Worked out when a field
    /// changes rather than every frame, since it asks the disk.
    pub problem: Option<String>,
    /// Give the name field keyboard focus next frame.
    pub focus_name: bool,
}

impl NewProjectForm {
    /// Work out `problem` again, offering another name if the current one
    /// isn't the user's and has been taken.
    pub fn validate(&mut self) {
        let location = Path::new(&self.location);
        if !self.name_edited {
            self.name = suggest_name(location);
        }
        self.problem = if let Some(problem) = name_problem(&self.name) {
            Some(problem.to_owned())
        } else if !location.is_absolute() {
            Some("Choose a location".to_owned())
        } else if location.join(&self.name).exists() {
            Some(format!("“{}” already exists in that folder", self.name))
        } else {
            None
        };
    }

    /// The folder the project would get.
    #[must_use]
    pub fn path(&self) -> PathBuf {
        Path::new(&self.location).join(&self.name)
    }

    /// Point the form at `editor`, dropping choices it doesn't offer.
    pub fn set_editor(&mut self, editor: &UnityEditor) {
        self.editor.clone_from(&editor.exe_path);
        // A downloaded template may not suit this editor either.
        let previous = self.template.take();
        self.template = previous.filter(|path| editor.templates.iter().any(|t| t.path == *path));
        let platform = self.platform.take();
        self.platform = platform.filter(|p| editor.platforms.contains(p));
    }

    /// What to ask the library to create.
    #[must_use]
    pub fn request(&self) -> NewProject {
        NewProject {
            name: self.name.clone(),
            parent: PathBuf::from(&self.location),
            template: self.template.as_ref().map(PathBuf::from),
            build_target: self
                .platform
                .as_deref()
                .and_then(build_target)
                .map(str::to_owned),
        }
    }
}

/// What the Project Settings dialog has filled in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectSettingsForm {
    /// The project's path.
    pub project: String,
    /// The user's own name for it. Empty means none.
    pub title: String,
    /// More arguments to launch the editor with.
    pub launch_args: String,
    /// The platform to launch on, or `None` for the project's own.
    pub platform: Option<String>,
}

/// What the Change Version dialog has filled in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeVersionForm {
    /// The project's path.
    pub project: String,
    /// The editor's executable path.
    pub editor: String,
}

/// Everything the panels read.
pub struct UiState {
    /// The library.
    pub hub: Hub,
    /// The page on show.
    pub page: Page,
    /// The selected project's path.
    pub selected: Option<String>,
    /// The search box's text.
    pub search: String,
    /// The project order.
    pub sort: SortKey,
    /// Indices into `hub.projects`, filtered and in display order.
    pub visible: Vec<usize>,
    /// Left sidebar shown.
    pub sidebar_visible: bool,
    /// Details panel shown.
    pub details_visible: bool,
    /// The details panel was hidden by a narrow window, not by the user.
    pub details_auto_hidden: bool,
    /// The background job running, if any.
    pub busy: Option<JobKind>,
    /// The project waiting for a remove confirmation.
    pub confirm_remove: Option<String>,
    /// The New Project dialog, while it's open.
    pub new_project: Option<NewProjectForm>,
    /// The Project Settings dialog, while it's open.
    pub project_settings: Option<ProjectSettingsForm>,
    /// The Change Version dialog, while it's open.
    pub change_version: Option<ChangeVersionForm>,
    /// Asking whether to remove projects whose folders are gone.
    pub confirm_remove_missing: bool,
    /// Unity's own folders that exist here: what to call each, and its path.
    pub unity_folders: Vec<(String, String)>,
    /// Online templates.
    pub online: OnlineState,
    /// Whether `git` works, once that's known.
    pub git_available: Option<bool>,
    /// New Project was asked for with no editors to offer.
    pub new_project_blocked: bool,
    /// What the New Project dialog starts with.
    pub new_project_defaults: NewProjectDefaults,
    /// Give the search box keyboard focus next frame.
    pub focus_search: bool,
    /// Scroll the selected row into view next frame.
    pub scroll_to_selected: bool,
    /// The project list's scroll offset last frame (points).
    pub list_offset: f32,
    /// The project list's visible height last frame (points).
    pub list_viewport: f32,
    /// Messages for the toasts.
    pub notices: Notices,
}

impl UiState {
    /// State for `hub`, showing the projects page.
    #[must_use]
    pub fn new(hub: Hub, settings: &Settings) -> Self {
        let mut state = Self {
            hub,
            page: Page::Projects,
            selected: None,
            search: String::new(),
            sort: settings.sort,
            visible: Vec::new(),
            sidebar_visible: settings.sidebar_visible,
            details_visible: settings.details_visible,
            details_auto_hidden: false,
            busy: None,
            confirm_remove: None,
            new_project: None,
            project_settings: None,
            change_version: None,
            confirm_remove_missing: false,
            unity_folders: Vec::new(),
            online: OnlineState::default(),
            git_available: None,
            new_project_blocked: false,
            new_project_defaults: settings.new_project.clone(),
            focus_search: false,
            scroll_to_selected: false,
            list_offset: 0.0,
            list_viewport: 0.0,
            notices: Notices::default(),
        };
        state.refresh_visible();
        state
    }

    /// The settings to remember between sessions.
    #[must_use]
    pub fn settings(&self) -> Settings {
        Settings {
            sort: self.sort,
            sidebar_visible: self.sidebar_visible,
            details_visible: self.details_visible || self.details_auto_hidden,
            new_project: self.new_project_defaults.clone(),
        }
    }

    /// Work out the visible rows again. Call when the projects, search or
    /// sort change. A selected project that is no longer listed is deselected.
    pub fn refresh_visible(&mut self) {
        let terms: Vec<String> = self
            .search
            .split_whitespace()
            .map(str::to_lowercase)
            .collect();
        let projects = &self.hub.projects;
        let mut visible: Vec<usize> = (0..projects.len())
            .filter(|&ix| matches_search(&projects[ix], &terms))
            .collect();
        match self.sort {
            SortKey::Recent => visible.sort_by_key(|&ix| Reverse(projects[ix].edit_time)),
            SortKey::Name => visible.sort_by_key(|&ix| projects[ix].title.to_lowercase()),
            SortKey::Version => {
                visible.sort_by_key(|&ix| Reverse(version_key(&projects[ix].version)));
            }
        }
        self.visible = visible;
        let listed = |path: &String| {
            self.visible
                .iter()
                .any(|&ix| self.hub.projects[ix].path == *path)
        };
        if !self.selected.as_ref().is_some_and(listed) {
            self.selected = None;
        }
    }

    /// The project at `path`.
    #[must_use]
    pub fn project(&self, path: &str) -> Option<&UnityProject> {
        self.hub.projects.iter().find(|p| p.path == path)
    }

    /// The selected project.
    #[must_use]
    pub fn selected_project(&self) -> Option<&UnityProject> {
        self.project(self.selected.as_deref()?)
    }

    /// Installed editors, newest first.
    #[must_use]
    pub fn editors_newest_first(&self) -> Vec<&UnityEditor> {
        let mut editors: Vec<&UnityEditor> =
            self.hub.config.editors_configurations.iter().collect();
        editors.sort_by_key(|editor| Reverse(version_key(&editor.version)));
        editors
    }

    /// The editor at `exe_path`.
    #[must_use]
    pub fn editor(&self, exe_path: &str) -> Option<&UnityEditor> {
        self.hub
            .config
            .editors_configurations
            .iter()
            .find(|editor| editor.exe_path == exe_path)
    }

    /// Whether a dialog is open, so keyboard shortcuts should wait.
    #[must_use]
    pub fn sheet_open(&self) -> bool {
        self.confirm_remove.is_some()
            || self.new_project.is_some()
            || self.new_project_blocked
            || self.project_settings.is_some()
            || self.change_version.is_some()
            || self.confirm_remove_missing
    }

    /// A New Project form for `editor`, or else the preferred editor, or else
    /// the last used, or else the newest. `None` if no editors are installed.
    #[must_use]
    pub fn new_project_form(
        &self,
        editor: Option<&str>,
        location: String,
    ) -> Option<NewProjectForm> {
        let defaults = &self.new_project_defaults;
        let named = |exe: Option<&str>| exe.and_then(|exe| self.editor(exe));
        let editor = named(editor)
            .or_else(|| named(self.hub.preferred_editor.as_deref()))
            .or_else(|| named(defaults.editor.as_deref()))
            .or_else(|| self.editors_newest_first().first().copied())?;
        let template = defaults.template.as_ref().and_then(|name| {
            let template = editor.templates.iter().find(|t| t.label() == name)?;
            Some(template.path.clone())
        });
        let mut form = NewProjectForm {
            name: String::new(),
            name_edited: false,
            location,
            editor: editor.exe_path.clone(),
            template,
            source: TemplateSource::default(),
            platform: None,
            init_git: defaults.init_git,
            problem: None,
            focus_name: true,
        };
        form.validate();
        Some(form)
    }

    /// Whether `project` can be opened.
    #[must_use]
    pub fn status(&self, project: &UnityProject) -> ProjectStatus {
        if !project.is_valid {
            ProjectStatus::NotFound
        } else if self.hub.editor_for_project(project).is_none() {
            ProjectStatus::EditorMissing
        } else {
            ProjectStatus::Ready
        }
    }

    /// Move the selection `step` rows through the visible list. With nothing
    /// selected, going down starts at the top and going up at the bottom.
    pub fn select_step(&mut self, step: isize) {
        let Some(last) = self.visible.len().checked_sub(1) else {
            return;
        };
        let current = self.selected.as_deref().and_then(|path| {
            self.visible
                .iter()
                .position(|&ix| self.hub.projects[ix].path == path)
        });
        let next = match current {
            Some(row) => row.saturating_add_signed(step).min(last),
            None if step > 0 => 0,
            None => last,
        };
        self.selected = Some(self.hub.projects[self.visible[next]].path.clone());
        self.scroll_to_selected = true;
    }
}

fn matches_search(project: &UnityProject, terms: &[String]) -> bool {
    terms.iter().all(|term| {
        [
            &project.title,
            &project.path,
            &project.branch,
            &project.version,
        ]
        .iter()
        .any(|field| field.to_lowercase().contains(term))
    })
}
