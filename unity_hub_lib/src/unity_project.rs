use serde::{Deserialize, Serialize};
use std::{ops::Sub, path::Path, str};

use crate::consts;

#[derive(Debug, Serialize, Deserialize, Clone, Hash, Eq)]
pub struct UnityProject {
    pub path: String,
    pub title: String,
    pub version: String,
    pub branch: String,
    pub is_valid: bool,
    pub edit_time: std::time::SystemTime,
}

impl PartialEq for UnityProject {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

impl UnityProject {
    #[cfg(not(target_os = "windows"))]
    pub fn get_projects_from_registry() -> Vec<UnityProject> {
        Vec::new()
    }
    #[cfg(target_os = "windows")]
    pub fn get_projects_from_registry() -> Vec<UnityProject> {
        use registry::{Hive, Security};
        let mut projects = Vec::new();

        let key = Hive::CurrentUser
            .open(
                r"SOFTWARE\Unity Technologies\Unity Editor 5.x",
                Security::Read,
            )
            .unwrap();

        for value in key.values() {
            if value.is_err() {
                continue;
            }
            let val = value.unwrap();
            let unwraped_name = val.name().to_string().unwrap();
            if !unwraped_name.contains("RecentlyUsedProjectPaths-") {
                continue;
            }

            if let registry::value::Data::Binary(data) = &val.data() {
                let project_path = str::from_utf8(&data).unwrap().to_string();
                if let Some(result) = UnityProject::try_get_project_at_path(&project_path) {
                    projects.push(result);
                }
            }
        }
        projects
    }

    fn is_project_at_path(path: &str) -> bool {
        let one = Path::new(&path).join("ProjectSettings");
        let two = one.join("ProjectVersion.txt");

        std::fs::metadata(&one).is_ok() && std::fs::metadata(&two).is_ok()
    }

    pub fn get_version_at_path(path: &str) -> Option<String> {
        let project_version_file = std::fs::read_to_string(
            Path::new(&path)
                .join("ProjectSettings")
                .join("ProjectVersion.txt"),
        );
        if project_version_file.is_err() {
            return None;
        }
        let binding = project_version_file.unwrap();
        let mut iter = binding.split_whitespace();
        iter.next();
        let project_version = iter.next().unwrap().to_string();

        Some(project_version)
    }

    pub fn try_get_project_at_path(path: &str) -> Option<UnityProject> {
        #[cfg(windows)]
        let path = path.trim_matches(char::from(0)).replace("/", "\\");
        #[cfg(not(windows))]
        let path = path.trim_matches(char::from(0)).to_string();
        if !UnityProject::is_project_at_path(&path) {
            return None;
        }

        let mut project = UnityProject {
            path: path.clone(),
            title: path.split(consts::SLASH).last().unwrap().to_string(),
            branch: String::new(),
            version: String::new(),
            is_valid: true,
            edit_time: std::time::SystemTime::now()
                .sub(std::time::Duration::new(60 * 60 * 24 * 365 * 30, 0)),
        };

        project.update_info();

        Some(project)
    }

    pub fn update_info(&mut self) {
        let is_project = UnityProject::is_project_at_path(&self.path);
        self.is_valid = is_project;

        if !is_project {
            return;
        }

        let mut base_path = Path::new(&self.path);
        self.version = Self::get_version_at_path(&self.path).unwrap();

        match self.try_read_from_path(base_path) {
            None => {
                while let Some(path) = base_path.parent() {
                    base_path = path;
                    let new_branch = self.try_read_from_path(base_path);
                    if new_branch.is_some() {
                        self.branch = new_branch.unwrap();
                        break;
                    }
                }
            }
            Some(value) => {
                self.branch = value;
            }
        }

        if let Ok(meta) = std::fs::metadata(&self.path) {
            if let Ok(data) = meta.modified() {
                self.edit_time = data;
            }
        }
    }

    fn try_read_from_path(&self, path: &std::path::Path) -> Option<String> {
        const HEAD_PREFIX: &str = "ref: refs/heads/";

        let head_path = Path::new(&path).join(".git").join("HEAD");
        if !head_path.exists() {
            return None;
        }
        let head_content = std::fs::read_to_string(&head_path).expect("Could not read HEAD file");
        if head_content.contains(HEAD_PREFIX) {
            Some(head_content.replace(HEAD_PREFIX, "").trim().to_string())
        } else {
            None
        }
    }
}
