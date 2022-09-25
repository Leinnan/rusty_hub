use std::{path::Path, str};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UnityProject {
    pub path: String,
    pub title: String,
    pub version: String,
    pub branch: String,
    pub is_valid: bool,
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

    fn is_project_at_path(path: &str) -> bool {
        let one = Path::new(&path).join("ProjectSettings");
        let two = one.join("ProjectVersion.txt");

        std::fs::metadata(&one).is_ok() && std::fs::metadata(&two).is_ok()
    }

    pub fn try_get_project_at_path(path: &str) -> Option<UnityProject> {
        let path = path.trim_matches(char::from(0)).replace("/", "\\");
        if !UnityProject::is_project_at_path(&path) {
            return None;
        }
        let project_version_file = std::fs::read_to_string(
            Path::new(&path)
                .join("ProjectSettings")
                .join("ProjectVersion.txt"),
        );
        let project_version_file = project_version_file.unwrap();
        let mut iter = project_version_file.split_whitespace();
        iter.next();
        let project_version = iter.next().unwrap().to_string();

        Some(UnityProject {
            path: path.to_string(),
            title: path.split("\\").last().unwrap().to_string(),
            version: project_version,
            branch: String::new(),
            is_valid: true,
        })
    }

    pub fn update_info(&mut self) {
        const HEAD_PREFIX: &str = "ref: refs/heads/";

        let is_project = UnityProject::is_project_at_path(&self.path);
        self.is_valid = is_project;

        if !is_project {
            return;
        }

        let mut base_path = Path::new(&self.path);

        while let Some(path) = base_path.parent() {
            base_path = path;
            let head_path = Path::new(&path).join(".git").join("HEAD");
            if !head_path.exists() {
                continue;
            }
            let head_content =
                std::fs::read_to_string(&head_path).expect("Could not read HEAD file");
            if head_content.contains(HEAD_PREFIX) {
                self.branch = head_content.replace(HEAD_PREFIX, "").trim().to_string();
            }
        }
    }
}
