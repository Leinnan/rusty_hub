use walkdir::{DirEntry, WalkDir};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Configuration {
    pub unity_search_paths: Vec<String>,
}

impl Configuration {
    pub fn get_unity_paths(&self) -> Vec<String> {
        let mut paths = Vec::new();

        for path in &self.unity_search_paths {
            paths.extend(Configuration::search_for_editor(path.as_str()));
        }

        paths
    }

    fn is_unity_dir(entry: &DirEntry) -> bool {
        let uninstall_exists = entry.path().clone().join("Uninstall.exe").exists();
        let unity_exe_exists = entry.path().clone().join("Unity.exe").exists();
        // println!(
        //     "{}: Unity.exe {}, Uninstall.exe {}", entry.path().display(),
        //     unity_exe_exists, uninstall_exists
        // );
        uninstall_exists && unity_exe_exists
    }

    fn search_for_editor(path: &str) -> Vec<String> {
        let path_exists = std::fs::metadata(path).is_ok();
        if !path_exists {
            return Vec::new();
        }
        let mut result_paths: Vec<String> = Vec::new();

        for entry in WalkDir::new(path)
            .max_depth(5)
            .into_iter()
            .filter_entry(|_| true)
        {
            if entry.is_ok() {
                let entry_unwraped = entry.unwrap();
                let success = Configuration::is_unity_dir(&entry_unwraped);
                if success {
                    result_paths.push(entry_unwraped.path().to_string_lossy().into());
                }
            }
        }
        println!("Result paths are:");
        for path in &result_paths {
            println!("- {}", path);
        }
        result_paths
    }
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            unity_search_paths: vec!["C:\\Program Files\\Unity\\Hub".to_string()],
        }
    }
}
