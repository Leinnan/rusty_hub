use std::process::Command;

use crate::{config::Configuration, unity_project::UnityProject, unity_editor::UnityEditor};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Hub {
    pub config: Configuration,
    pub projects: Vec<UnityProject>,
}

impl Hub {
    pub fn new(config: Configuration, projects: Vec<UnityProject>) -> Self {
        Self { config, projects }
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
}
impl Default for Hub {
    fn default() -> Self {
        Hub::new(
            Configuration::default(),
            UnityProject::get_projects_from_registry(),
        )
    }
}
