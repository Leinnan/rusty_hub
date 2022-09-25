use std::{path::PathBuf, process::Command};

use walkdir::WalkDir;

use crate::{config::Configuration, unity_editor::UnityEditor, unity_project::UnityProject};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Hub {
    pub config: Configuration,
    pub projects: Vec<UnityProject>,
}

impl Hub {
    pub fn new(config: Configuration, projects: Vec<UnityProject>) -> Self {
        Self { config, projects }
    }

    pub fn update_info(&mut self) {
        self.config.rebuild();
        for project in self.projects.iter_mut() {
            project.update_info();
        }
    }

    pub fn run_project_nr(&self, nr: usize) {
        let project = self.projects[nr].clone();

        if let Some(editor) = self.editor_for_project(&project) {
            Hub::run_project(&editor, &project);
        }
    }

    pub fn editor_for_project(&self, project: &UnityProject) -> Option<UnityEditor> {
        let editor_option = self
            .config
            .editors_configurations
            .clone()
            .into_iter()
            .find(|editor| editor.version.contains(&project.version));

        editor_option
    }

    pub fn run_project(editor: &UnityEditor, project: &UnityProject) {
        println!("{} -projectpath {}", editor.exe_path, project.path);
        Command::new(&editor.exe_path)
            .arg("-projectpath")
            .arg(&project.path)
            .spawn()
            .expect("Failed to run project");
    }

    pub fn search_for_projects_at_path(&mut self, path: &PathBuf) -> usize {
        let path_exists = std::fs::metadata(path).is_ok();
        let mut result = 0;
        if !path_exists {
            return result;
        }
        for entry in WalkDir::new(path)
            .max_depth(3)
            .into_iter()
            .filter_entry(|_| true)
        {
            let projects = self.projects.clone();
            if entry.is_err() {
                continue;
            }

            let entry_unwraped = entry.unwrap();
            let path_string = entry_unwraped.path().as_os_str().to_str();
            if let Some(project) = UnityProject::try_get_project_at_path(&path_string.unwrap()) {
                if !projects.contains(&project) {
                    self.projects.push(project);
                    result = result + 1;
                }
            }
        }
        result
    }
}
impl Default for Hub {
    fn default() -> Self {
        Hub::new(
            Configuration::default(),
            UnityProject::get_projects_from_registry(),
        )
    }
}
