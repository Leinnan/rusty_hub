//! App commands and the queue that widgets and shortcuts push them onto.
//!
//! Widgets only read state and schedule commands: `ViewCmd::Refresh.schedule()`.
//! The app applies them once per frame. The queue is thread-local: egui runs
//! on the UI thread, and it keeps parallel UI tests from draining each other's
//! commands. Background work hands results back through [`crate::jobs`].

use std::{cell::RefCell, collections::VecDeque, path::PathBuf};

use crate::state::{Page, ProjectSettingsForm, SortKey};

/// Every action the user can trigger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Acting on projects.
    Project(ProjectCmd),
    /// Acting on editors and their search paths.
    Editor(EditorCmd),
    /// Navigation and layout.
    View(ViewCmd),
}

/// Acting on projects; projects are named by their path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectCmd {
    /// Open in the editor the project asks for.
    Open(String),
    /// Open in a chosen editor, named by its executable path.
    OpenWith {
        /// The project's path.
        project: String,
        /// The editor's executable path.
        editor: String,
    },
    /// Show the folder in the file manager.
    Reveal(String),
    /// Copy the path to the clipboard.
    CopyPath(String),
    /// Ask whether to remove it from the list.
    RequestRemove(String),
    /// Remove it from the list.
    ConfirmRemove(String),
    /// Open the New Project dialog, with an editor chosen if one is named by
    /// its executable path.
    RequestNew(Option<String>),
    /// Pick the folder a new project goes in.
    PickNewLocation,
    /// Create the project the New Project dialog describes.
    CreateNew,
    /// Ask for the online catalog again.
    RetryCatalog,
    /// Download an online template, named by its package.
    DownloadTemplate(String),
    /// Stop the download.
    CancelDownload,
    /// Delete downloaded templates.
    ClearDownloads,
    /// Open the Project Settings dialog.
    RequestSettings(String),
    /// Keep what the Project Settings dialog holds.
    SaveSettings(ProjectSettingsForm),
    /// Open the Change Version dialog.
    RequestChangeVersion(String),
    /// Move a project to a chosen editor's version and open it there.
    ChangeVersion {
        /// The project's path.
        project: String,
        /// The editor's executable path.
        editor: String,
    },
    /// Ask whether to remove projects whose folders are gone.
    RequestRemoveMissing,
    /// Remove projects whose folders are gone.
    RemoveMissing,
    /// Pick a folder to scan.
    ScanFolder,
    /// Add the project at, or the projects under, a folder.
    AddFolder(PathBuf),
    /// Add the projects the Unity Hub lists.
    ImportFromHub,
}

/// Acting on editors and their search paths.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorCmd {
    /// Pick a folder to search for editors.
    AddSearchPath,
    /// Stop searching a folder.
    RemoveSearchPath(String),
    /// Show a folder in the file manager.
    Reveal(String),
    /// Launch an editor with no project, named by its executable path.
    Run(String),
    /// Offer an editor first for new projects, or stop preferring one.
    SetPreferred(Option<String>),
}

/// Navigation and layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewCmd {
    /// Switch page.
    ShowPage(Page),
    /// Select a project.
    Select(String),
    /// Change the project order.
    SetSort(SortKey),
    /// Show or hide the left sidebar.
    ToggleSidebar,
    /// Show or hide the project details.
    ToggleDetails,
    /// Read editors and projects from disk again.
    Refresh,
}

macro_rules! command_group {
    ($($group:ident => $variant:ident),* $(,)?) => {$(
        impl From<$group> for Command {
            fn from(command: $group) -> Self {
                Self::$variant(command)
            }
        }

        impl $group {
            /// Queue this command for the next frame.
            pub fn schedule(self) {
                Command::from(self).schedule();
            }
        }
    )*};
}

command_group! {
    ProjectCmd => Project,
    EditorCmd => Editor,
    ViewCmd => View,
}

thread_local! {
    static QUEUE: RefCell<VecDeque<Command>> = const { RefCell::new(VecDeque::new()) };
}

impl Command {
    /// Queue this command for the next frame.
    pub fn schedule(self) {
        QUEUE.with_borrow_mut(|queue| queue.push_back(self));
    }

    /// Take every queued command, oldest first.
    pub fn drain() -> Vec<Self> {
        QUEUE.with_borrow_mut(|queue| queue.drain(..).collect())
    }

    /// Whether a command is waiting for the next frame.
    pub fn pending() -> bool {
        QUEUE.with_borrow(|queue| !queue.is_empty())
    }
}
