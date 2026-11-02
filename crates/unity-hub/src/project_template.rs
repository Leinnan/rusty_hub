//! Project templates shipped with an editor.

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, Clone, Eq)]
pub struct ProjectTemplate {
    pub path: String,
    pub title: String,
}

impl std::hash::Hash for ProjectTemplate {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.path.hash(state);
    }
}

impl PartialEq for ProjectTemplate {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

impl ProjectTemplate {
    /// The templates of the editor installed at `path`.
    #[must_use]
    pub fn find_templates(path: &str) -> Vec<ProjectTemplate> {
        let Ok(dir) = std::fs::read_dir(Path::new(path).join(crate::consts::TEMPLATES_DIR)) else {
            return Vec::new();
        };

        dir.filter_map(Result::ok)
            .filter_map(|entry| {
                let name = entry.file_name();
                let title = name.to_str()?.strip_suffix(".tgz")?.to_owned();
                Some(Self {
                    path: entry.path().to_string_lossy().into_owned(),
                    title,
                })
            })
            .collect()
    }
}
