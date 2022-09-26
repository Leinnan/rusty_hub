use crate::{consts, unity_editor::UnityEditor};
use dpc_pariter::IteratorExt;
use std::collections::HashSet;
use walkdir::{DirEntry, WalkDir};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Configuration {
    pub unity_search_paths: Vec<String>,
    pub editors_configurations: Vec<UnityEditor>,
}

impl Configuration {
    pub fn rebuild(&mut self) {
        let paths = self.get_unity_paths();
        println!("{}", paths.len());
        self.editors_configurations = paths
            .into_iter()
            .parallel_map(|path| UnityEditor::new(&path))
            .parallel_filter(|editor| editor.is_some())
            .parallel_map(|editor| editor.unwrap())
            .collect();
    }
    pub fn get_unity_paths(&self) -> Vec<String> {
        let mut paths = Vec::new();

        for path in &self.unity_search_paths {
            paths.extend(Configuration::search_for_editor(path.as_str()));
        }

        paths
    }

    fn is_unity_dir(entry: &DirEntry) -> bool {
        #[cfg(windows)]
        let uninstall_exists = entry.path().clone().join("Uninstall.exe").exists();
        #[cfg(unix)]
        let uninstall_exists = true; // just check that on windows only
        let unity_exe_exists = entry.path().clone().join(consts::UNITY_EXE_NAME).exists();
        println!(
            "PATH {} {:?}",
            unity_exe_exists,
            &entry.path().clone().join(consts::UNITY_EXE_NAME)
        );

        uninstall_exists && unity_exe_exists
    }

    fn search_for_editor(path: &str) -> Vec<String> {
        let path_exists = std::fs::metadata(path).is_ok();
        if !path_exists {
            println!("PATH NOT EXIST {}", &path);
            return Vec::new();
        }

        let hashset: HashSet<String> = WalkDir::new(path)
            .max_depth(2)
            .into_iter()
            .parallel_filter(|entry| entry.is_ok())
            .parallel_map(|entry| entry.unwrap())
            .parallel_filter(|entry| Configuration::is_unity_dir(&entry))
            .parallel_map(|entry| entry.path().to_string_lossy().into())
            .collect();

        Vec::from_iter(hashset)
    }
}

impl Default for Configuration {
    fn default() -> Self {
        let mut default = Self {
            #[cfg(windows)]
            unity_search_paths: vec!["C:\\Program Files\\Unity\\Hub\\Editor".to_string()],
            #[cfg(target_os = "macos")]
            unity_search_paths: vec![
                "/Applications/Unity/Hub/Editor".to_string(),
                "/Applications/Unity/".to_string(),
            ],
            #[cfg(target_os = "linux")]
            unity_search_paths: vec!["~/Unity/Hub/Editor".to_string()],
            editors_configurations: Vec::new(),
        };
        default.rebuild();

        default
    }
}
