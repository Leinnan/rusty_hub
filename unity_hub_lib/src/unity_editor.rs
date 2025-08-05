use crate::consts;
use crate::project_template::ProjectTemplate;
use serde::{Deserialize, Serialize};
use std::borrow::Borrow;
use std::collections::HashMap;
use std::hash::Hash;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, Clone, Eq, Hash)]
pub struct UnityEditor {
    pub version: String,
    pub exe_path: String,
    pub base_path: String,
    pub platforms: Vec<String>,
    pub templates: Vec<ProjectTemplate>,
}

impl PartialEq for UnityEditor {
    fn eq(&self, other: &Self) -> bool {
        self.exe_path == other.exe_path
    }
}

impl UnityEditor {
    pub fn new(path: &str) -> Option<Self> {
        let base_path = Path::new(path);
        let exe_path = base_path.join(consts::UNITY_EXE_NAME);
        let meta = std::fs::metadata(&exe_path);
        if !meta.is_ok_and(|meta| meta.is_file()) {
            return None;
        }

        let mut version: Option<String> = None;

        #[cfg(windows)]
        {
            use exe::pe::VecPE;
            use exe::VSVersionInfo;
            let image = VecPE::from_disk_file(&exe_path).unwrap();
            let vs_version_check = VSVersionInfo::parse(&image);

            if let Some(string_file_info) = vs_version_check.unwrap().string_file_info {
                let hashmap_result = string_file_info.children[0].string_map();

                let hash_map = match hashmap_result {
                    Ok(r) => r,
                    Err(_) => HashMap::new(),
                };

                if let Some(result_version) = hash_map.get("ProductVersion") {
                    version = Some(result_version.clone());
                    if let Some(short) = result_version.clone().split("_").take(1).next() {
                        version = Some(short.to_string());
                    }
                }
            }
        }
        if version.is_none() {
            let folder = base_path
                .to_str()
                .expect("Fail")
                .split(consts::SLASH)
                .last()
                .unwrap();
            version = Some(folder.to_string());
        }

        if version.is_none() {
            return None;
        }
        Some(Self {
            version: version.unwrap().clone(),
            exe_path: exe_path.into_os_string().into_string().unwrap(),
            base_path: String::from(path),
            platforms: UnityEditor::get_platforms(&path),
            templates: ProjectTemplate::find_templates(&path),
        })
    }

    fn get_platforms(unity_folder: &str) -> Vec<String> {
        let platform_names = HashMap::from([
            ("androidplayer", "Android"),
            ("windowsstandalonesupport", "Windows"),
            ("linuxstandalonesupport", "Linux"),
            ("LinuxStandalone", "Linux"),
            ("OSXStandalone", "OSX"),
            ("webglsupport", "WebGL"),
            ("metrosupport", "UWP"),
            ("iossupport", "iOS"),
        ]);

        let mut platforms = Vec::new();
        let base_path = Path::new(unity_folder).join("Data").join("PlaybackEngines");

        if std::fs::metadata(&base_path).is_err() {
            return platforms;
        }
        let dir = std::fs::read_dir(base_path);

        if dir.is_err() {
            return platforms;
        }

        for path in dir.unwrap() {
            if path.is_err() {
                continue;
            }
            let f = path.unwrap();
            if let Ok(result_string) = f.file_name().into_string() {
                if let Some(value) =
                    platform_names.get(&result_string.clone().to_lowercase().borrow())
                {
                    platforms.push(value.to_string());
                } else {
                    platforms.push(result_string);
                }
            }
        }

        platforms
    }
}
