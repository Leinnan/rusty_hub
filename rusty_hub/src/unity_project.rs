use std::{path::Path, str};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UnityProject {
    pub path: String,
    pub title: String,
    pub version: String,
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
        println!("{}", key.to_string());

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
                println!("\t{}: {}", unwraped_name, project_path);
            }
        }
        projects
    }

    pub fn try_get_project_at_path(path: &str) -> Option<UnityProject> {
        let path = path.trim_matches(char::from(0)).replace("/", "\\");
        let second_path = Path::new(&path).join("ProjectSettings");
        if std::fs::metadata(&second_path).is_err() {
            return None;
        }
        let project_version_file = std::fs::read_to_string(second_path.join("ProjectVersion.txt"));
        if project_version_file.is_err() {
            return None;
        }
        let project_version_file = project_version_file.unwrap();
        let mut iter = project_version_file.split_whitespace();
        iter.next();
        let project_version = iter.next().unwrap().to_string();

        Some(UnityProject {
            path: path.to_string(),
            title: path.split("\\").last().unwrap().to_string(),
            version: project_version,
        })
    }
}
