use crate::{config::Configuration, unity_editor::UnityEditor, unity_project::UnityProject};
use dpc_pariter::IteratorExt;
use std::{path::PathBuf, process::Command};
use walkdir::WalkDir;
use std::collections::HashSet;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Hub {
    pub config: Configuration,
    pub projects: Vec<UnityProject>,
}

impl Hub {
    pub fn new(config: Configuration, projects: Vec<UnityProject>) -> Self {
        Self { config, projects }
    }

    pub fn update_data(&mut self) {
        self.config.rebuild();
        self.update_projects_info();
    }

    pub fn update_projects_info(&mut self) {
        let mut registry = UnityProject::get_projects_from_registry()
            .into_iter()
            .filter(|p| !self.projects.contains(p))
            .collect();
        self.projects.append(&mut registry);
        self.projects = self.projects.iter().cloned().collect::<HashSet<UnityProject>>().into_iter().collect();
        self.projects.iter_mut().for_each(|project| {
            project.update_info();
        });
        self.projects.sort_by(|a, b| b.edit_time.cmp(&a.edit_time));
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
        if !path_exists {
            return 0;
        }
        let projects = self.projects.clone();
        let new_projects: Vec<UnityProject> = WalkDir::new(path)
            .max_depth(3)
            .into_iter()
            .parallel_filter(|entry| entry.is_ok())
            .parallel_map(|entry| {
                UnityProject::try_get_project_at_path(
                    &entry.unwrap().path().as_os_str().to_str().unwrap(),
                )
            })
            .parallel_filter(|project| project.is_some())
            .parallel_map(|project| project.unwrap())
            .parallel_filter(move |p| !projects.contains(p))
            .collect();

        let len = new_projects.len();
        self.projects.extend(new_projects);

        len
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
