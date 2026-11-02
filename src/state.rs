//! What the UI shows: the library plus page, selection, search and sort.

use std::cmp::Reverse;

use serde::{Deserialize, Serialize};
use unity_hub::{hub::Hub, unity_project::UnityProject};

use crate::{jobs::JobKind, settings::Settings};

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

/// The numbers in a Unity version, so 2022.3.9f1 sorts before 2022.3.22f1.
fn version_key(version: &str) -> Vec<u32> {
    version
        .split(|c: char| !c.is_ascii_digit())
        .filter_map(|part| part.parse().ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::version_key;

    #[test]
    fn versions_compare_by_number() {
        assert!(version_key("2022.3.22f1") > version_key("2022.3.9f1"));
        assert!(version_key("6000.0.1f1") > version_key("2022.3.22f1"));
        assert!(version_key("").is_empty());
    }
}
