//! The saved library: editors, known projects, and projects the user removed.

use crate::{config::Configuration, unity_editor::UnityEditor, unity_project::UnityProject};
use dpc_pariter::IteratorExt;
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::collections::HashSet;
use std::io;
use std::path::Path;
use std::process::{Child, Command};
use walkdir::WalkDir;

/// How deep a folder scan looks for projects.
const SCAN_DEPTH: usize = 3;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Hub {
    /// Paths the user removed, so the registry doesn't bring them back.
    /// Comes first because TOML wants plain values before tables.
    #[serde(default)]
    pub hidden_projects: Vec<String>,
    pub config: Configuration,
    pub projects: Vec<UnityProject>,
}

impl Hub {
    #[must_use]
    pub fn new(config: Configuration, projects: Vec<UnityProject>) -> Self {
        Self {
            hidden_projects: Vec::new(),
            config,
            projects,
        }
    }

    /// Re-read editors and projects from disk. Walks the disk, so it belongs
    /// on a background thread.
    pub fn update_data(&mut self) {
        self.config.rebuild();
        self.update_projects_info();
    }

    pub fn update_projects_info(&mut self) {
        let hidden = &self.hidden_projects;
        let mut registry = UnityProject::get_projects_from_registry()
            .into_iter()
            .filter(|p| !self.projects.contains(p) && !hidden.contains(&p.path))
            .collect();
        self.projects.append(&mut registry);
        self.projects = self
            .projects
            .iter()
            .cloned()
            .collect::<HashSet<UnityProject>>()
            .into_iter()
            .collect();
        for project in &mut self.projects {
            project.update_info();
        }
        self.projects.sort_by_key(|p| Reverse(p.edit_time));
    }

    /// The installed editor a project asks for.
    #[must_use]
    pub fn editor_for_project(&self, project: &UnityProject) -> Option<&UnityEditor> {
        if project.version.is_empty() {
            return None;
        }
        self.config
            .editors_configurations
            .iter()
            .find(|editor| editor.version.contains(&project.version))
    }

    /// Launch `project` in `editor`.
    pub fn run_project(editor: &UnityEditor, project: &UnityProject) -> io::Result<()> {
        let child = Command::new(&editor.exe_path)
            .arg("-projectpath")
            .arg(&project.path)
            .spawn()?;
        reap_when_done(child);
        Ok(())
    }

    /// Launch `project` in the editor it asks for.
    pub fn open_project(&self, project: &UnityProject) -> io::Result<()> {
        let editor = self.editor_for_project(project).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("Unity {} is not installed", project.version),
            )
        })?;
        Self::run_project(editor, project)
    }

    /// Forget `path`, and remember not to bring it back. Returns whether it
    /// was known.
    pub fn remove_project(&mut self, path: &str) -> bool {
        let before = self.projects.len();
        self.projects.retain(|p| p.path != path);
        if !self.hidden_projects.iter().any(|hidden| hidden == path) {
            self.hidden_projects.push(path.to_owned());
        }
        self.projects.len() != before
    }

    /// Add projects the user asked for, un-hiding them. Returns how many
    /// were new.
    pub fn add_projects(&mut self, projects: Vec<UnityProject>) -> usize {
        let mut added = 0;
        for project in projects {
            self.hidden_projects
                .retain(|hidden| *hidden != project.path);
            if !self.projects.contains(&project) {
                self.projects.push(project);
                added += 1;
            }
        }
        added
    }
}

/// Wait for a launched process from a thread of its own, so it doesn't stay
/// around as a zombie after it exits.
fn reap_when_done(mut child: Child) {
    std::thread::spawn(move || {
        let _ = child.wait();
    });
}

/// Projects at or under `path` that aren't in `known`. Walks the disk, so it
/// belongs on a background thread.
#[must_use]
pub fn scan_projects(path: &Path, known: &[UnityProject]) -> Vec<UnityProject> {
    let known = known.to_vec();
    WalkDir::new(path)
        .max_depth(SCAN_DEPTH)
        .into_iter()
        .filter_map(Result::ok)
        .parallel_map(|entry| {
            UnityProject::try_get_project_at_path(&entry.path().to_string_lossy())
        })
        .flatten()
        .filter(|project| !known.contains(project))
        .collect()
}

impl Default for Hub {
    /// An empty library with the usual editor locations. Nothing is read
    /// from disk until [`Self::update_data`].
    fn default() -> Self {
        Hub::new(Configuration::default(), Vec::new())
    }
}
