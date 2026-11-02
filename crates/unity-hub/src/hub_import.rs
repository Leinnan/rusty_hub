//! Projects the official Unity Hub knows about, read from its project list.
//!
//! Only `projects-v1.json` is read, and it is never written. The same folder
//! holds the Hub's account tokens, so nothing else in it is touched.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use directories::BaseDirs;
use serde::Deserialize;

use crate::unity_project::UnityProject;

#[derive(Deserialize)]
struct ProjectsFile {
    data: BTreeMap<String, Entry>,
}

#[derive(Deserialize)]
struct Entry {
    path: Option<String>,
}

/// Where the Unity Hub keeps its project list on this machine.
#[must_use]
pub fn projects_file() -> Option<PathBuf> {
    let dirs = BaseDirs::new()?;
    Some(dirs.config_dir().join("UnityHub").join("projects-v1.json"))
}

/// The projects the Unity Hub lists on this machine that still exist.
#[must_use]
pub fn import_projects() -> Vec<UnityProject> {
    projects_file().map_or_else(Vec::new, |file| read_projects(&file))
}

/// The projects listed in `file` that still exist. A missing or malformed
/// file has none.
#[must_use]
pub fn read_projects(file: &Path) -> Vec<UnityProject> {
    let Ok(text) = std::fs::read_to_string(file) else {
        return Vec::new();
    };
    let Ok(list) = serde_json::from_str::<ProjectsFile>(&text) else {
        return Vec::new();
    };
    list.data
        .into_iter()
        .filter_map(|(key, entry)| {
            UnityProject::try_get_project_at_path(&entry.path.unwrap_or(key))
        })
        .collect()
}
