//! Where to look for Unity editors, and the editors found there.

use crate::project_template::ProjectTemplate;
use crate::{consts, unity_editor::UnityEditor};
use dpc_pariter::IteratorExt;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use walkdir::{DirEntry, WalkDir};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Configuration {
    pub unity_search_paths: Vec<String>,
    pub editors_configurations: Vec<UnityEditor>,
}

impl Configuration {
    /// Find the editors under the search paths again. Walks the disk, so it
    /// belongs on a background thread.
    pub fn rebuild(&mut self) {
        let paths = self.get_unity_paths();
        let known: Arc<Vec<ProjectTemplate>> = Arc::new(
            self.editors_configurations
                .iter()
                .flat_map(|editor| editor.templates.iter().cloned())
                .collect(),
        );
        self.editors_configurations = paths
            .into_iter()
            .parallel_map(move |path| UnityEditor::with_known_templates(&path, &known))
            .flatten()
            .collect();
    }

    #[must_use]
    pub fn get_unity_paths(&self) -> Vec<String> {
        let mut paths = Vec::new();

        for path in &self.unity_search_paths {
            paths.extend(Configuration::search_for_editor(path.as_str()));
        }

        paths
    }

    fn is_unity_dir(entry: &DirEntry) -> bool {
        #[cfg(windows)]
        let uninstall_exists = entry.path().join("Uninstall.exe").exists();
        #[cfg(unix)]
        let uninstall_exists = true; // just check that on windows only
        let unity_exe_exists = entry.path().join(consts::UNITY_EXE_NAME).exists();
        uninstall_exists && unity_exe_exists
    }

    fn search_for_editor(path: &str) -> Vec<String> {
        let home = directories::BaseDirs::new().map(|dirs| dirs.home_dir().to_path_buf());
        let path = expand_home(path, home.as_deref());
        let path_exists = std::fs::metadata(&path).is_ok();
        if !path_exists {
            return Vec::new();
        }

        let hashset: HashSet<String> = WalkDir::new(&path)
            .max_depth(2)
            .into_iter()
            .filter_map(Result::ok)
            .parallel_filter(Configuration::is_unity_dir)
            .parallel_map(|entry| entry.path().to_string_lossy().into())
            .collect();

        Vec::from_iter(hashset)
    }
}

/// `path` with a leading `~` replaced by `home`.
fn expand_home(path: &str, home: Option<&Path>) -> PathBuf {
    match (path.strip_prefix('~'), home) {
        (Some(""), Some(home)) => home.to_path_buf(),
        (Some(rest), Some(home)) if rest.starts_with(['/', '\\']) => {
            home.join(rest.trim_start_matches(['/', '\\']))
        }
        _ => PathBuf::from(path),
    }
}

impl Default for Configuration {
    /// The usual install locations, with no editors found yet: call
    /// [`Self::rebuild`] to look.
    fn default() -> Self {
        Self {
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
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::expand_home;

    #[test]
    fn only_a_leading_home_marker_is_expanded() {
        let home = Some(Path::new("/home/me"));
        assert_eq!(
            expand_home("~/Unity/Hub", home),
            PathBuf::from("/home/me/Unity/Hub")
        );
        assert_eq!(expand_home("~", home), PathBuf::from("/home/me"));
        assert_eq!(expand_home("~other/x", home), PathBuf::from("~other/x"));
        assert_eq!(expand_home("/opt/~/x", home), PathBuf::from("/opt/~/x"));
        assert_eq!(expand_home("~/x", None), PathBuf::from("~/x"));
    }
}
