//! A Unity project on disk.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::Path;
use std::time::SystemTime;

/// Templates leave their package's id here until Unity replaces it.
const TEMPLATE_PRODUCT_PREFIX: &str = "com.unity.template.";

#[derive(Debug, Serialize, Deserialize, Clone, Eq)]
pub struct UnityProject {
    pub path: String,
    pub title: String,
    pub version: String,
    pub branch: String,
    pub is_valid: bool,
    /// The name set in Unity's player settings, if it says something.
    #[serde(default)]
    pub product_name: String,
    /// What the project's packages say it renders with, if that's known.
    #[serde(default)]
    pub render_pipeline: String,
    /// The user's own name for it, which never touches the folder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_title: Option<String>,
    /// More arguments to launch the editor with.
    #[serde(default)]
    pub launch_args: String,
    /// The platform to launch on, as [`crate::unity_editor::UnityEditor`] lists them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub launch_platform: Option<String>,
    // A table in TOML, so it comes after the plain values.
    pub edit_time: SystemTime,
}

impl std::hash::Hash for UnityProject {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.path.hash(state);
    }
}

impl PartialEq for UnityProject {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

impl UnityProject {
    /// Projects the editor recorded in the Windows registry.
    #[cfg(not(target_os = "windows"))]
    #[must_use]
    pub fn get_projects_from_registry() -> Vec<UnityProject> {
        Vec::new()
    }

    /// Projects the editor recorded in the Windows registry. Empty when
    /// Unity has never run on this machine.
    #[cfg(target_os = "windows")]
    #[must_use]
    pub fn get_projects_from_registry() -> Vec<UnityProject> {
        use registry::{Hive, Security};

        let Ok(key) = Hive::CurrentUser.open(
            r"SOFTWARE\Unity Technologies\Unity Editor 5.x",
            Security::Read,
        ) else {
            return Vec::new();
        };

        let mut projects = Vec::new();
        for value in key.values().filter_map(Result::ok) {
            let is_recent_project = value
                .name()
                .to_string()
                .is_ok_and(|name| name.contains("RecentlyUsedProjectPaths-"));
            if !is_recent_project {
                continue;
            }
            if let registry::value::Data::Binary(data) = &value.data()
                && let Ok(path) = std::str::from_utf8(data)
                && let Some(project) = UnityProject::try_get_project_at_path(path)
            {
                projects.push(project);
            }
        }
        projects
    }

    /// The editor version a project asks for, from its `ProjectVersion.txt`.
    #[must_use]
    pub fn get_version_at_path(path: &str) -> Option<String> {
        let text = std::fs::read_to_string(
            Path::new(path)
                .join("ProjectSettings")
                .join("ProjectVersion.txt"),
        )
        .ok()?;
        // "m_EditorVersion: 2022.3.22f1"
        text.split_whitespace().nth(1).map(str::to_owned)
    }

    /// The project at `path`, if there is one.
    #[must_use]
    pub fn try_get_project_at_path(path: &str) -> Option<UnityProject> {
        #[cfg(windows)]
        let path = path.trim_matches(char::from(0)).replace('/', "\\");
        #[cfg(not(windows))]
        let path = path.trim_matches(char::from(0)).to_string();
        let version = UnityProject::get_version_at_path(&path)?;

        let title = Path::new(&path)
            .file_name()
            .map_or_else(|| path.clone(), |name| name.to_string_lossy().into_owned());
        let mut project = UnityProject {
            path,
            title,
            branch: String::new(),
            version,
            is_valid: true,
            product_name: String::new(),
            render_pipeline: String::new(),
            custom_title: None,
            launch_args: String::new(),
            launch_platform: None,
            edit_time: SystemTime::UNIX_EPOCH,
        };

        project.update_info();

        Some(project)
    }

    /// Work out the title: the user's name for the project, or the product
    /// name, or the folder's.
    pub fn refresh_title(&mut self) {
        self.title = if let Some(title) = &self.custom_title {
            title.clone()
        } else if self.product_name.is_empty() {
            Path::new(&self.path).file_name().map_or_else(
                || self.path.clone(),
                |name| name.to_string_lossy().into_owned(),
            )
        } else {
            self.product_name.clone()
        };
    }

    /// Record that the project now asks for `version`, then re-read it. Unity
    /// fills in the rest of the file when it opens the project, so a line
    /// naming the old version's revision is dropped rather than left to lie.
    pub fn set_version(&mut self, version: &str) -> io::Result<()> {
        let settings = Path::new(&self.path).join("ProjectSettings");
        let file = settings.join("ProjectVersion.txt");
        let old = fs::read_to_string(&file)?;
        let mut text = format!("m_EditorVersion: {version}\n");
        for line in old
            .lines()
            .filter(|line| !line.starts_with("m_EditorVersion"))
        {
            text.push_str(line);
            text.push('\n');
        }
        // A rename, so a failure part way can't leave half a file behind.
        let draft = settings.join("ProjectVersion.txt.tmp");
        fs::write(&draft, text)?;
        fs::rename(&draft, &file).inspect_err(|_| {
            let _ = fs::remove_file(&draft);
        })?;
        self.update_info();
        Ok(())
    }

    /// Read the version, branch and modified time from disk again. A project
    /// that can't be read any more is kept, marked not valid.
    pub fn update_info(&mut self) {
        let Some(version) = Self::get_version_at_path(&self.path) else {
            self.is_valid = false;
            return;
        };
        self.is_valid = true;
        self.version = version;
        let path = Path::new(&self.path);
        self.branch = path
            .ancestors()
            .find_map(|dir| read_git_branch(dir).or_else(|| read_plastic_branch(dir)))
            .unwrap_or_default();
        self.product_name = read_product_name(path).unwrap_or_default();
        self.render_pipeline = read_render_pipeline(path).unwrap_or_default().to_owned();
        self.refresh_title();

        if let Ok(modified) = std::fs::metadata(&self.path).and_then(|meta| meta.modified()) {
            self.edit_time = modified;
        }
    }
}

/// The checked-out branch of the git repository at `dir`, if it has one. In a
/// worktree or submodule `.git` is a file naming the real folder.
fn read_git_branch(dir: &Path) -> Option<String> {
    const HEAD_PREFIX: &str = "ref: refs/heads/";
    const GITDIR_PREFIX: &str = "gitdir:";

    let git = dir.join(".git");
    let git_dir = if git.is_file() {
        let text = fs::read_to_string(&git).ok()?;
        dir.join(text.trim().strip_prefix(GITDIR_PREFIX)?.trim())
    } else {
        git
    };
    let head = fs::read_to_string(git_dir.join("HEAD")).ok()?;
    head.trim().strip_prefix(HEAD_PREFIX).map(str::to_owned)
}

/// The branch of the Plastic SCM workspace at `dir`, if it is one. The selector
/// names it on a line like `br "/main/feature"`.
fn read_plastic_branch(dir: &Path) -> Option<String> {
    let text = fs::read_to_string(dir.join(".plastic").join("plastic.selector")).ok()?;
    text.lines().find_map(|line| {
        let line = line.trim();
        let rest = line
            .strip_prefix("smartbranch ")
            .or_else(|| line.strip_prefix("br "))?;
        let name = rest.trim().trim_matches('"').trim_start_matches('/');
        (!name.is_empty()).then(|| name.to_owned())
    })
}

/// The product name in the player settings, unless it's only a template's id.
/// Projects saved in Unity's binary format have nothing to read.
fn read_product_name(project: &Path) -> Option<String> {
    let bytes = fs::read(
        project
            .join("ProjectSettings")
            .join("ProjectSettings.asset"),
    )
    .ok()?;
    let text = String::from_utf8_lossy(&bytes);
    let name = text
        .lines()
        .find_map(|line| line.trim_start().strip_prefix("productName:"))?
        .trim()
        .trim_matches(['"', '\''].as_slice());
    (!name.is_empty() && !name.starts_with(TEMPLATE_PRODUCT_PREFIX)).then(|| name.to_owned())
}

/// The render pipeline a project's packages bring in. Having the package isn't
/// quite the same as using it, but Unity's own settings can't settle this
/// either: newer templates set the pipeline per quality level.
fn read_render_pipeline(project: &Path) -> Option<&'static str> {
    let manifest = fs::read_to_string(project.join("Packages").join("manifest.json")).ok()?;
    Some(
        if manifest.contains("com.unity.render-pipelines.high-definition") {
            "HDRP"
        } else if manifest.contains("com.unity.render-pipelines.universal") {
            "URP"
        } else {
            "Built-in"
        },
    )
}

/// Split launch arguments the way a shell would: on spaces, except inside
/// quotes. An unfinished quote is an error, rather than a guess.
pub fn parse_launch_args(text: &str) -> Result<Vec<String>, &'static str> {
    let mut args = Vec::new();
    let mut current: Option<String> = None;
    let mut quote = None;
    for c in text.chars() {
        if let Some(open) = quote {
            if c == open {
                quote = None;
            } else {
                current.get_or_insert_default().push(c);
            }
        } else if c == '"' || c == '\'' {
            quote = Some(c);
            current.get_or_insert_default();
        } else if c.is_whitespace() {
            args.extend(current.take());
        } else {
            current.get_or_insert_default().push(c);
        }
    }
    if quote.is_some() {
        return Err("A quote isn't closed");
    }
    args.extend(current);
    Ok(args)
}
