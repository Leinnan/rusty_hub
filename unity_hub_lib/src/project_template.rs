use dpc_pariter::IteratorExt;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, Clone, Eq, Hash)]
pub struct ProjectTemplate {
    pub path: String,
    pub title: String,
}

impl PartialEq for ProjectTemplate {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

impl ProjectTemplate {
    pub fn find_templates(path: &str) -> Vec<ProjectTemplate> {
        let dir = std::fs::read_dir(Path::new(&path).join(crate::consts::TEMPLATES_DIR));

        if dir.is_err() {
            return Vec::new();
        }

        dir.unwrap()
            .into_iter()
            .parallel_filter(|path| path.is_ok())
            .parallel_map(|path| path.unwrap())
            .parallel_filter(|path| path.file_name().into_string().unwrap().contains(".tgz"))
            .parallel_map(|path| Self {
                path: path.path().to_str().unwrap().to_string(),
                title: path.file_name().into_string().unwrap().replace(".tgz", ""),
            })
            .collect()
    }
}
