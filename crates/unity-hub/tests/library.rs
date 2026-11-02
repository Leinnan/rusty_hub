//! Library behaviour against real files in temporary folders.

use std::fs;
use std::path::Path;

use tempfile::TempDir;
use unity_hub::config::Configuration;
use unity_hub::consts::UNITY_EXE_NAME;
use unity_hub::hub::{Hub, scan_projects};
use unity_hub::hub_import::read_projects;
use unity_hub::unity_editor::UnityEditor;
use unity_hub::unity_project::UnityProject;

fn make_project(dir: &Path, version: &str) -> String {
    let settings = dir.join("ProjectSettings");
    fs::create_dir_all(&settings).expect("create project");
    fs::write(
        settings.join("ProjectVersion.txt"),
        format!("m_EditorVersion: {version}\nm_EditorVersionWithRevision: {version} (abc)\n"),
    )
    .expect("write version");
    dir.to_string_lossy().into_owned()
}

fn project(dir: &Path, version: &str) -> UnityProject {
    UnityProject::try_get_project_at_path(&make_project(dir, version)).expect("a project")
}

#[test]
fn detects_a_project_and_its_version() {
    let tmp = TempDir::new().expect("temp dir");
    let found = project(&tmp.path().join("My Game"), "2022.3.22f1");
    assert_eq!(found.title, "My Game");
    assert_eq!(found.version, "2022.3.22f1");
    assert!(found.is_valid);
}

#[test]
fn a_plain_folder_is_not_a_project() {
    let tmp = TempDir::new().expect("temp dir");
    assert!(UnityProject::try_get_project_at_path(&tmp.path().to_string_lossy()).is_none());
}

#[test]
fn malformed_version_files_do_not_panic() {
    let tmp = TempDir::new().expect("temp dir");
    let settings = tmp.path().join("ProjectSettings");
    fs::create_dir_all(&settings).expect("create project");
    fs::write(settings.join("ProjectVersion.txt"), "").expect("write version");
    assert_eq!(
        UnityProject::get_version_at_path(&tmp.path().to_string_lossy()),
        None
    );
}

#[test]
fn a_missing_project_is_kept_but_marked_not_valid() {
    let tmp = TempDir::new().expect("temp dir");
    let dir = tmp.path().join("Gone");
    let mut found = project(&dir, "6000.0.1f1");
    fs::remove_dir_all(&dir).expect("remove project");
    found.update_info();
    assert!(!found.is_valid);
    assert_eq!(found.version, "6000.0.1f1");
}

#[test]
fn finds_the_branch_in_the_project_or_a_parent_repo() {
    let tmp = TempDir::new().expect("temp dir");
    let git = tmp.path().join(".git");
    fs::create_dir_all(&git).expect("create repo");
    fs::write(git.join("HEAD"), "ref: refs/heads/feature/żółw\n").expect("write HEAD");
    let found = project(&tmp.path().join("games").join("Nested"), "6000.0.1f1");
    assert_eq!(found.branch, "feature/żółw");
}

#[test]
fn a_detached_head_has_no_branch() {
    let tmp = TempDir::new().expect("temp dir");
    let git = tmp.path().join(".git");
    fs::create_dir_all(&git).expect("create repo");
    fs::write(git.join("HEAD"), "0123456789abcdef\n").expect("write HEAD");
    assert_eq!(project(&tmp.path().join("Game"), "6000.0.1f1").branch, "");
}

#[test]
fn scan_finds_new_projects_only() {
    let tmp = TempDir::new().expect("temp dir");
    let known = project(&tmp.path().join("Known"), "6000.0.1f1");
    make_project(&tmp.path().join("nested").join("New"), "6000.0.1f1");
    let found = scan_projects(tmp.path(), &[known]);
    let titles: Vec<&str> = found.iter().map(|p| p.title.as_str()).collect();
    assert_eq!(titles, ["New"]);
}

#[test]
fn removed_projects_stay_hidden_until_added_again() {
    let tmp = TempDir::new().expect("temp dir");
    let found = project(&tmp.path().join("Game"), "6000.0.1f1");
    let mut hub = Hub::new(Configuration::default(), vec![found.clone()]);

    assert!(hub.remove_project(&found.path));
    assert!(hub.projects.is_empty());
    assert!(hub.hidden_projects.contains(&found.path));
    hub.update_projects_info();
    assert!(hub.projects.is_empty());

    assert_eq!(hub.add_projects(vec![found.clone()]), 1);
    assert!(hub.hidden_projects.is_empty());
    assert_eq!(hub.add_projects(vec![found]), 0, "already known");
}

#[test]
fn duplicate_paths_collapse_on_refresh() {
    let tmp = TempDir::new().expect("temp dir");
    let first = project(&tmp.path().join("Game"), "6000.0.1f1");
    let mut second = first.clone();
    second.branch = "stale".to_owned();
    let mut hub = Hub::new(Configuration::default(), vec![first, second]);
    hub.update_projects_info();
    assert_eq!(hub.projects.len(), 1);
}

#[test]
fn opening_without_the_editor_is_an_error() {
    let tmp = TempDir::new().expect("temp dir");
    let found = project(&tmp.path().join("Game"), "6000.0.1f1");
    let hub = Hub::new(Configuration::default(), vec![found.clone()]);
    assert!(hub.editor_for_project(&found).is_none());
    assert!(hub.open_project(&found).is_err());
}

#[test]
fn finds_an_editor_with_its_folder_as_the_version() {
    let tmp = TempDir::new().expect("temp dir");
    let base = tmp.path().join("6000.2.8f1");
    let exe = base.join(UNITY_EXE_NAME);
    fs::create_dir_all(exe.parent().expect("a parent")).expect("create editor");
    fs::write(&exe, "").expect("write exe");
    for dir in unity_hub::consts::PLAYBACK_ENGINE_DIRS {
        fs::create_dir_all(base.join(dir).join("iOSSupport")).expect("create module");
        // Stray files aren't modules.
        fs::write(base.join(dir).join(".DS_Store"), "").expect("write file");
    }

    let editor = UnityEditor::new(&base.to_string_lossy()).expect("an editor");
    #[cfg(not(windows))]
    assert_eq!(editor.version, "6000.2.8f1");
    assert_eq!(editor.platforms, ["iOS"]);
}

#[test]
fn imports_listed_projects_that_still_exist() {
    let tmp = TempDir::new().expect("temp dir");
    let real = make_project(&tmp.path().join("Real"), "6000.0.1f1");
    let stale = tmp.path().join("Stale").to_string_lossy().into_owned();
    let file = tmp.path().join("projects-v1.json");
    let list = serde_json::json!({
        "schema_version": "v1",
        "data": {
            real.clone(): {"title": "Real", "path": real, "version": "6000.0.1f1"},
            stale.clone(): {"title": "Stale", "path": stale, "version": "6000.0.1f1"},
        }
    });
    fs::write(&file, list.to_string()).expect("write list");

    let found = read_projects(&file);
    let titles: Vec<&str> = found.iter().map(|p| p.title.as_str()).collect();
    assert_eq!(titles, ["Real"]);
}

#[test]
fn a_missing_or_malformed_hub_list_imports_nothing() {
    let tmp = TempDir::new().expect("temp dir");
    let file = tmp.path().join("projects-v1.json");
    assert!(read_projects(&file).is_empty());
    fs::write(&file, "{ not json").expect("write list");
    assert!(read_projects(&file).is_empty());
    fs::write(&file, r#"{"schema_version":"v2","items":[]}"#).expect("write list");
    assert!(read_projects(&file).is_empty());
}

#[test]
fn loads_a_config_saved_before_hidden_projects_existed() {
    let text = r#"
[config]
unity_search_paths = ["/Applications/Unity/Hub/Editor"]

[[config.editors_configurations]]
version = "2022.3.22f1"
exe_path = "/Applications/Unity/Hub/Editor/2022.3.22f1/Unity.app/Contents/MacOS/Unity"
base_path = "/Applications/Unity/Hub/Editor/2022.3.22f1"
platforms = []
templates = []

[[projects]]
path = "/tmp/Game"
title = "Game"
version = "2022.3.22f1"
branch = ""
is_valid = true

[projects.edit_time]
secs_since_epoch = 1724609811
nanos_since_epoch = 795729979
"#;
    let hub: Hub = toml::from_str(text).expect("old config parses");
    assert!(hub.hidden_projects.is_empty());
    assert_eq!(hub.projects[0].title, "Game");
    assert_eq!(hub.config.editors_configurations[0].version, "2022.3.22f1");
}

#[test]
fn config_round_trips_with_hidden_projects() {
    let tmp = TempDir::new().expect("temp dir");
    let kept = project(&tmp.path().join("Kept"), "6000.0.1f1");
    let mut hub = Hub::new(Configuration::default(), vec![kept]);
    hub.hidden_projects.push("/tmp/Removed".to_owned());

    let text = toml::to_string(&hub).expect("config serializes");
    let back: Hub = toml::from_str(&text).expect("config parses");
    assert_eq!(back.hidden_projects, ["/tmp/Removed"]);
    assert_eq!(back.projects, hub.projects);
}
