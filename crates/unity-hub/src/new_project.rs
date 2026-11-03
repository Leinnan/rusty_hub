//! Creating a project. Unity does the real work: this prepares the folder and
//! launches the editor with the right arguments.

use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::hub::{Hub, reap_when_done};
use crate::online_templates::hide_console;
use crate::unity_editor::UnityEditor;
use crate::unity_project::UnityProject;

/// The name offered for a new project.
const DEFAULT_NAME: &str = "My Project";
/// How many numbered names to try before giving up on suggesting one.
const MAX_SUGGESTIONS: u32 = 1000;
/// Characters a folder name can't have on at least one desktop system.
const FORBIDDEN: &[char] = &['<', '>', ':', '"', '/', '\\', '|', '?', '*'];

/// Unity's usual `.gitignore`.
const GITIGNORE: &str = include_str!("unity.gitignore");

/// What to create.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewProject {
    /// The project's name, which is also its folder's.
    pub name: String,
    /// The folder to create it in.
    pub parent: PathBuf,
    /// The template archive to start from, or `None` for Unity's empty project.
    pub template: Option<PathBuf>,
    /// The `-buildTarget` to start on, or `None` for the editor's default.
    pub build_target: Option<String>,
}

impl NewProject {
    /// The folder the project will live in.
    #[must_use]
    pub fn path(&self) -> PathBuf {
        self.parent.join(&self.name)
    }
}

/// Why `name` can't be used as a project's folder name, if it can't.
#[must_use]
pub fn name_problem(name: &str) -> Option<&'static str> {
    if name.is_empty() {
        return Some("Enter a name");
    }
    if name == "." || name == ".." {
        return Some("The name can't be only dots");
    }
    if name
        .chars()
        .any(|c| FORBIDDEN.contains(&c) || c.is_control())
    {
        return Some("The name can't contain < > : \" / \\ | ? *");
    }
    if name.starts_with(' ') || name.ends_with(' ') || name.ends_with('.') {
        return Some("The name can't start with a space, or end with a space or dot");
    }
    let stem = name.split('.').next().unwrap_or(name).to_uppercase();
    let reserved = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ["COM", "LPT"].iter().any(|prefix| {
            stem.strip_prefix(prefix)
                .is_some_and(|rest| matches!(rest.as_bytes(), [b'1'..=b'9']))
        });
    if reserved {
        return Some("Windows reserves that name");
    }
    None
}

/// Whether the `git` command works. Run it away from the UI thread: it can be
/// slow, and on a Mac without developer tools it asks to install them.
#[must_use]
pub fn git_available() -> bool {
    git_works(Path::new("git"))
}

fn git_works(program: &Path) -> bool {
    let mut command = Command::new(program);
    hide_console(&mut command);
    command
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

/// Start a git repository in a new project, with Unity's `.gitignore`. Nothing
/// is committed: at this point Unity hasn't generated anything to commit.
pub fn init_git(path: &Path) -> Result<(), String> {
    init_git_with(Path::new("git"), path)
}

fn init_git_with(program: &Path, path: &Path) -> Result<(), String> {
    let mut command = Command::new(program);
    hide_console(&mut command);
    let output = command
        .args(["init", "--quiet"])
        .current_dir(path)
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        let text = String::from_utf8_lossy(&output.stderr);
        return Err(text.trim().to_owned());
    }
    fs::write(path.join(".gitignore"), GITIGNORE).map_err(|error| error.to_string())
}

/// A name that isn't taken in `parent`.
#[must_use]
pub fn suggest_name(parent: &Path) -> String {
    (2..MAX_SUGGESTIONS)
        .map(|number| format!("{DEFAULT_NAME} {number}"))
        .find(|name| !parent.join(name).exists())
        .filter(|_| parent.join(DEFAULT_NAME).exists())
        .unwrap_or_else(|| DEFAULT_NAME.to_owned())
}

/// The editor's command line for creating `request`.
#[must_use]
pub fn build_args(request: &NewProject) -> Vec<OsString> {
    let mut args: Vec<OsString> = Vec::new();
    if let Some(template) = &request.template {
        args.push("-createProject".into());
        args.push(request.path().into());
        args.push("-cloneFromTemplate".into());
        args.push(template.into());
    } else {
        args.push("-projectPath".into());
        args.push(request.path().into());
    }
    if let Some(target) = &request.build_target {
        args.push("-buildTarget".into());
        args.push(target.into());
    }
    args
}

/// The `-buildTarget` for a platform name [`UnityEditor`] lists.
#[must_use]
pub fn build_target(platform: &str) -> Option<&'static str> {
    Some(match platform {
        "Android" => "android",
        "iOS" => "ios",
        "Linux" => "linux64",
        "macOS" => "osxuniversal",
        "UWP" => "WindowsStoreApps",
        "WebGL" => "webgl",
        "Windows" => "win64",
        _ => return None,
    })
}

impl Hub {
    /// Create `request` and launch `editor` on it. An existing folder is never
    /// reused. If the launch fails, what this created is removed again, apart
    /// from `request.parent`.
    ///
    /// Unity keeps working after this returns, and can still fail on its own.
    pub fn create_project(editor: &UnityEditor, request: &NewProject) -> io::Result<UnityProject> {
        if let Some(problem) = name_problem(&request.name) {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, problem));
        }
        fs::create_dir_all(&request.parent)?;
        let path = request.path();
        fs::create_dir(&path)?;

        let result = prepare(&path, &editor.version).and_then(|()| {
            let child = Command::new(&editor.exe_path)
                .args(build_args(request))
                .spawn()?;
            reap_when_done(child);
            UnityProject::try_get_project_at_path(&path.to_string_lossy())
                .ok_or_else(|| io::Error::other("the new project can't be read"))
        });
        if result.is_err() {
            clean_up(&path);
        }
        result
    }
}

/// Lay out what makes a folder a project, so it can be listed and opened while
/// Unity is still filling it in.
fn prepare(path: &Path, version: &str) -> io::Result<()> {
    fs::create_dir(path.join("Assets"))?;
    let settings = path.join("ProjectSettings");
    fs::create_dir(&settings)?;
    fs::write(
        settings.join("ProjectVersion.txt"),
        format!("m_EditorVersion: {version}\n"),
    )
}

/// Undo [`prepare`] and the folder's creation. These calls only remove what
/// is empty or named here, so a folder something else wrote to stays.
fn clean_up(path: &Path) {
    let _ = fs::remove_file(path.join("ProjectSettings").join("ProjectVersion.txt"));
    let _ = fs::remove_dir(path.join("ProjectSettings"));
    let _ = fs::remove_dir(path.join("Assets"));
    let _ = fs::remove_dir(path);
}

#[cfg(test)]
mod tests {
    use super::{GITIGNORE, NewProject, build_args, git_works, init_git_with, name_problem};
    use std::ffi::OsString;
    use std::path::{Path, PathBuf};

    #[test]
    fn a_missing_git_is_noticed() {
        assert!(!git_works(Path::new("/nonexistent/git")));
        let tmp = tempfile::TempDir::new().expect("temp dir");
        assert!(init_git_with(Path::new("/nonexistent/git"), tmp.path()).is_err());
        assert!(!tmp.path().join(".gitignore").exists());
    }

    #[test]
    fn git_setup_makes_a_repository_with_nothing_committed() {
        if !git_works(Path::new("git")) {
            return;
        }
        let tmp = tempfile::TempDir::new().expect("temp dir");
        init_git_with(Path::new("git"), tmp.path()).expect("git init");
        assert!(tmp.path().join(".git").is_dir());
        assert_eq!(
            std::fs::read_to_string(tmp.path().join(".gitignore")).expect("written"),
            GITIGNORE
        );
        assert!(GITIGNORE.contains("/[Ll]ibrary/"));
    }

    #[test]
    fn usable_names_pass() {
        for name in [
            "Game",
            "My Project 2",
            "żółw",
            "v1.2 demo",
            "COM",
            "Console",
        ] {
            assert_eq!(name_problem(name), None, "{name}");
        }
    }

    #[test]
    fn names_that_would_break_a_path_are_refused() {
        for name in [
            "",
            ".",
            "..",
            "a/b",
            "a\\b",
            "../up",
            "a:b",
            "what?",
            "tab\there",
            " lead",
            "trail ",
            "trail.",
            "CON",
            "con.txt",
            "COM1",
            "lpt9",
        ] {
            assert!(name_problem(name).is_some(), "{name:?}");
        }
    }

    fn request(template: Option<&str>, target: Option<&str>) -> NewProject {
        NewProject {
            name: "Game".to_owned(),
            parent: PathBuf::from("/work"),
            template: template.map(PathBuf::from),
            build_target: target.map(str::to_owned),
        }
    }

    fn args(request: &NewProject) -> Vec<OsString> {
        build_args(request)
    }

    #[test]
    fn a_template_is_cloned_by_unity() {
        let expected: Vec<OsString> = [
            "-createProject",
            "/work/Game",
            "-cloneFromTemplate",
            "/t/urp.tgz",
            "-buildTarget",
            "android",
        ]
        .map(OsString::from)
        .to_vec();
        assert_eq!(
            args(&request(Some("/t/urp.tgz"), Some("android"))),
            expected
        );
    }

    #[test]
    fn with_no_template_unity_fills_the_folder() {
        let expected: Vec<OsString> = ["-projectPath", "/work/Game"].map(OsString::from).to_vec();
        assert_eq!(args(&request(None, None)), expected);
    }
}
