//! A Unity project on disk.

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::SystemTime;

#[derive(Debug, Serialize, Deserialize, Clone, Eq)]
pub struct UnityProject {
    pub path: String,
    pub title: String,
    pub version: String,
    pub branch: String,
    pub is_valid: bool,
    pub edit_time: SystemTime,
}

impl std::hash::Hash for UnityProject {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.path.hash(state);
    }
}

impl PartialEq for UnityProject {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

impl UnityProject {
    /// Projects the editor recorded in the Windows registry.
    #[cfg(not(target_os = "windows"))]
    #[must_use]
    pub fn get_projects_from_registry() -> Vec<UnityProject> {
        Vec::new()
    }

    /// Projects the editor recorded in the Windows registry. Empty when
    /// Unity has never run on this machine.
    #[cfg(target_os = "windows")]
    #[must_use]
    pub fn get_projects_from_registry() -> Vec<UnityProject> {
        use registry::{Hive, Security};

        let Ok(key) = Hive::CurrentUser.open(
            r"SOFTWARE\Unity Technologies\Unity Editor 5.x",
            Security::Read,
        ) else {
            return Vec::new();
        };

        let mut projects = Vec::new();
        for value in key.values().filter_map(Result::ok) {
            let is_recent_project = value
                .name()
                .to_string()
                .is_ok_and(|name| name.contains("RecentlyUsedProjectPaths-"));
            if !is_recent_project {
                continue;
            }
            if let registry::value::Data::Binary(data) = &value.data()
                && let Ok(path) = std::str::from_utf8(data)
                && let Some(project) = UnityProject::try_get_project_at_path(path)
            {
                projects.push(project);
            }
        }
        projects
    }

    /// The editor version a project asks for, from its `ProjectVersion.txt`.
    #[must_use]
    pub fn get_version_at_path(path: &str) -> Option<String> {
        let text = std::fs::read_to_string(
            Path::new(path)
                .join("ProjectSettings")
                .join("ProjectVersion.txt"),
        )
        .ok()?;
        // "m_EditorVersion: 2022.3.22f1"
        text.split_whitespace().nth(1).map(str::to_owned)
    }

    /// The project at `path`, if there is one.
    #[must_use]
    pub fn try_get_project_at_path(path: &str) -> Option<UnityProject> {
        #[cfg(windows)]
        let path = path.trim_matches(char::from(0)).replace('/', "\\");
        #[cfg(not(windows))]
        let path = path.trim_matches(char::from(0)).to_string();
        let version = UnityProject::get_version_at_path(&path)?;

        let title = Path::new(&path)
            .file_name()
            .map_or_else(|| path.clone(), |name| name.to_string_lossy().into_owned());
        let mut project = UnityProject {
            path,
            title,
            branch: String::new(),
            version,
            is_valid: true,
            edit_time: SystemTime::UNIX_EPOCH,
        };

        project.update_info();

        Some(project)
    }

    /// Read the version, branch and modified time from disk again. A project
    /// that can't be read any more is kept, marked not valid.
    pub fn update_info(&mut self) {
        let Some(version) = Self::get_version_at_path(&self.path) else {
            self.is_valid = false;
            return;
        };
        self.is_valid = true;
        self.version = version;
        self.branch = Path::new(&self.path)
            .ancestors()
            .find_map(read_branch)
            .unwrap_or_default();

        if let Ok(modified) = std::fs::metadata(&self.path).and_then(|meta| meta.modified()) {
            self.edit_time = modified;
        }
    }
}

/// The checked-out branch of the git repository at `dir`, if it has one.
fn read_branch(dir: &Path) -> Option<String> {
    const HEAD_PREFIX: &str = "ref: refs/heads/";

    let head = std::fs::read_to_string(dir.join(".git").join("HEAD")).ok()?;
    head.trim().strip_prefix(HEAD_PREFIX).map(str::to_owned)
}
