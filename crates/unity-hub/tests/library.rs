//! Library behaviour against real files in temporary folders.

use std::fs;
use std::path::Path;

use tempfile::TempDir;
use unity_hub::config::Configuration;
use unity_hub::consts::UNITY_EXE_NAME;
use unity_hub::hub::{Hub, scan_projects};
use unity_hub::hub_import::read_projects;
use unity_hub::new_project::{NewProject, suggest_name};
use unity_hub::project_template::{ProjectTemplate, read_manifest};
use unity_hub::unity_editor::UnityEditor;
use unity_hub::unity_project::{UnityProject, parse_launch_args};

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

/// A fake editor at `exe`. It isn't run by these tests unless made executable.
fn fake_editor(exe: &Path) -> UnityEditor {
    UnityEditor {
        version: "6000.2.8f1".to_owned(),
        exe_path: exe.to_string_lossy().into_owned(),
        base_path: String::new(),
        platforms: Vec::new(),
        templates: Vec::new(),
    }
}

fn request(parent: &Path, template: Option<&Path>) -> NewProject {
    NewProject {
        name: "Game".to_owned(),
        parent: parent.to_path_buf(),
        template: template.map(Path::to_path_buf),
        build_target: None,
    }
}

#[test]
fn suggested_names_avoid_taken_folders() {
    let tmp = TempDir::new().expect("temp dir");
    assert_eq!(suggest_name(tmp.path()), "My Project");
    fs::create_dir(tmp.path().join("My Project")).expect("create folder");
    assert_eq!(suggest_name(tmp.path()), "My Project 2");
    fs::create_dir(tmp.path().join("My Project 2")).expect("create folder");
    assert_eq!(suggest_name(tmp.path()), "My Project 3");
}

#[test]
fn creating_over_an_existing_folder_is_refused_and_leaves_it_alone() {
    let tmp = TempDir::new().expect("temp dir");
    let existing = tmp.path().join("Game");
    fs::create_dir(&existing).expect("create folder");
    fs::write(existing.join("keep.txt"), "mine").expect("write file");

    let error = Hub::create_project(
        &fake_editor(&tmp.path().join("no-editor")),
        &request(tmp.path(), None),
    )
    .expect_err("the folder exists");
    assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
    assert_eq!(
        fs::read_to_string(existing.join("keep.txt")).expect("still there"),
        "mine"
    );
    assert!(!existing.join("Assets").exists());
}

#[test]
fn a_failed_launch_removes_only_what_it_created() {
    let tmp = TempDir::new().expect("temp dir");
    let parent = tmp.path().join("new").join("location");
    let result = Hub::create_project(
        &fake_editor(&tmp.path().join("no-editor")),
        &request(&parent, None),
    );
    assert!(result.is_err());
    assert!(!parent.join("Game").exists());
    assert!(parent.exists());
}

#[test]
fn a_bad_name_touches_nothing() {
    let tmp = TempDir::new().expect("temp dir");
    let mut bad = request(tmp.path(), None);
    bad.name = "../escape".to_owned();
    let error = Hub::create_project(&fake_editor(&tmp.path().join("no-editor")), &bad)
        .expect_err("a bad name");
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    assert_eq!(fs::read_dir(tmp.path()).expect("list").count(), 0);
}

#[cfg(unix)]
#[test]
fn creating_launches_the_editor_with_the_template_and_lists_the_project() {
    use std::os::unix::fs::PermissionsExt;

    let tmp = TempDir::new().expect("temp dir");
    let exe = tmp.path().join("fake-unity");
    let record = tmp.path().join("args.txt");
    // Written to a temporary name first: the script is complete when it runs.
    let draft = tmp.path().join("fake-unity.tmp");
    fs::write(
        &draft,
        format!(
            "#!/bin/sh\nfor a in \"$@\"; do printf '%s\\n' \"$a\"; done > '{0}.tmp'\nmv '{0}.tmp' '{0}'\n",
            record.display()
        ),
    )
    .expect("write script");
    fs::set_permissions(&draft, fs::Permissions::from_mode(0o755)).expect("make executable");
    fs::rename(&draft, &exe).expect("move script");

    let template = tmp.path().join("urp.tgz");
    let made = Hub::create_project(&fake_editor(&exe), &request(tmp.path(), Some(&template)))
        .expect("created");
    assert_eq!(made.title, "Game");
    assert_eq!(made.version, "6000.2.8f1");
    assert!(made.is_valid);
    assert!(tmp.path().join("Game").join("Assets").is_dir());

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !record.exists() && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let args = fs::read_to_string(&record).expect("the editor ran");
    let game = tmp.path().join("Game");
    assert_eq!(
        args.lines().collect::<Vec<_>>(),
        [
            "-createProject",
            &*game.to_string_lossy(),
            "-cloneFromTemplate",
            &*template.to_string_lossy(),
        ]
    );
}

/// A gzipped tar holding `files`, as Unity packages templates.
fn write_tgz(path: &Path, files: &[(&str, &str)]) {
    let gz = flate2::write::GzEncoder::new(
        fs::File::create(path).expect("create archive"),
        flate2::Compression::fast(),
    );
    let mut tar = tar::Builder::new(gz);
    for (name, text) in files {
        let mut header = tar::Header::new_gnu();
        header.set_size(text.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        tar.append_data(&mut header, name, text.as_bytes())
            .expect("add file");
    }
    tar.into_inner()
        .expect("finish tar")
        .finish()
        .expect("finish gzip");
}

const URP_MANIFEST: &str = r#"{"displayName":"3D URP","description":"For URP."}"#;

#[test]
fn reads_a_templates_name_from_inside_its_archive() {
    let tmp = TempDir::new().expect("temp dir");
    let archive = tmp.path().join("template.tgz");
    write_tgz(
        &archive,
        &[
            ("package/ProjectData~/big.bin", "data"),
            ("package/package.json", URP_MANIFEST),
        ],
    );
    let manifest = read_manifest(&archive).expect("a manifest");
    assert_eq!(manifest.display_name.as_deref(), Some("3D URP"));
    assert_eq!(manifest.description.as_deref(), Some("For URP."));
}

#[test]
fn files_that_are_not_templates_have_no_manifest() {
    let tmp = TempDir::new().expect("temp dir");
    let garbage = tmp.path().join("garbage.tgz");
    fs::write(&garbage, "<html>not an archive</html>").expect("write file");
    assert_eq!(read_manifest(&garbage), None);

    let no_manifest = tmp.path().join("empty.tgz");
    write_tgz(&no_manifest, &[("package/other.txt", "x")]);
    assert_eq!(read_manifest(&no_manifest), None);

    let bad_json = tmp.path().join("bad.tgz");
    write_tgz(&bad_json, &[("package/package.json", "{ nope")]);
    assert_eq!(read_manifest(&bad_json), None);

    assert_eq!(read_manifest(&tmp.path().join("missing.tgz")), None);
}

/// An editor folder holding one template archive.
fn editor_with_template(tmp: &Path) -> String {
    let base = tmp.join("6000.2.8f1");
    let dir = base.join(unity_hub::consts::TEMPLATES_DIR);
    fs::create_dir_all(&dir).expect("create templates");
    write_tgz(
        &dir.join("com.unity.template.misleading-1.0.0.tgz"),
        &[("package/package.json", URP_MANIFEST)],
    );
    base.to_string_lossy().into_owned()
}

#[test]
fn templates_show_their_real_name() {
    let tmp = TempDir::new().expect("temp dir");
    let found = ProjectTemplate::find_templates(&editor_with_template(tmp.path()), &[]);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].label(), "3D URP");
    assert_eq!(found[0].description.as_deref(), Some("For URP."));
}

#[test]
fn known_templates_are_not_read_again() {
    let tmp = TempDir::new().expect("temp dir");
    let base = editor_with_template(tmp.path());
    let mut known = ProjectTemplate::find_templates(&base, &[]);
    // A name that could only have come from what was already known.
    known[0].display_name = Some("Remembered".to_owned());
    let found = ProjectTemplate::find_templates(&base, &known);
    assert_eq!(found[0].label(), "Remembered");
}

#[test]
fn an_unreadable_template_falls_back_to_its_file_name() {
    let tmp = TempDir::new().expect("temp dir");
    let base = tmp.path().join("6000.2.8f1");
    let dir = base.join(unity_hub::consts::TEMPLATES_DIR);
    fs::create_dir_all(&dir).expect("create templates");
    fs::write(dir.join("com.unity.template.broken-1.0.0.tgz"), "nope").expect("write file");
    let found = ProjectTemplate::find_templates(&base.to_string_lossy(), &[]);
    assert_eq!(found[0].label(), "broken-1.0.0");
}

#[test]
fn loads_templates_saved_before_they_had_details() {
    let text = r#"
path = "/x/com.unity.template.urp-1.0.0.tgz"
title = "com.unity.template.urp-1.0.0"
"#;
    let template: ProjectTemplate = toml::from_str(text).expect("old template parses");
    assert_eq!(template.display_name, None);
    assert_eq!(template.label(), "urp-1.0.0");
}

#[test]
fn launch_args_split_like_a_shell() {
    assert_eq!(parse_launch_args(""), Ok(Vec::new()));
    assert_eq!(
        parse_launch_args(r#" -force-metal  -logFile "my log.txt" -x 'a b'"#),
        Ok(vec![
            "-force-metal".to_owned(),
            "-logFile".to_owned(),
            "my log.txt".to_owned(),
            "-x".to_owned(),
            "a b".to_owned(),
        ])
    );
    assert_eq!(
        parse_launch_args(r#"-a """#),
        Ok(vec!["-a".to_owned(), String::new()])
    );
    assert!(parse_launch_args(r#"-a "unfinished"#).is_err());
}

#[test]
fn changing_version_replaces_only_what_it_should() {
    let tmp = TempDir::new().expect("temp dir");
    let dir = tmp.path().join("Game");
    let mut found = project(&dir, "2022.3.22f1");
    let settings = dir.join("ProjectSettings");
    let file = settings.join("ProjectVersion.txt");
    let mut old = fs::read_to_string(&file).expect("read");
    old.push_str("somethingElse: kept\n");
    fs::write(&file, old).expect("write");

    found.set_version("6000.2.8f1").expect("changed");
    assert_eq!(found.version, "6000.2.8f1");
    assert_eq!(
        fs::read_to_string(&file).expect("read"),
        "m_EditorVersion: 6000.2.8f1\nsomethingElse: kept\n"
    );
    assert_eq!(
        fs::read_dir(&settings).expect("list").count(),
        1,
        "no draft left"
    );
}

#[test]
fn changing_the_version_of_a_missing_project_is_an_error() {
    let tmp = TempDir::new().expect("temp dir");
    let dir = tmp.path().join("Gone");
    let mut found = project(&dir, "6000.0.1f1");
    fs::remove_dir_all(&dir).expect("remove");
    assert!(found.set_version("6000.2.8f1").is_err());
    assert_eq!(found.version, "6000.0.1f1");
    assert!(!dir.exists(), "nothing was created");
}

fn write_product_name(dir: &Path, name: &str) {
    fs::write(
        dir.join("ProjectSettings").join("ProjectSettings.asset"),
        format!("%YAML 1.1\nPlayerSettings:\n  companyName: Me\n  productName: {name}\n"),
    )
    .expect("write settings");
}

#[test]
fn the_title_prefers_the_users_name_then_the_product_name_then_the_folder() {
    let tmp = TempDir::new().expect("temp dir");
    let dir = tmp.path().join("folder-name");
    let mut found = project(&dir, "6000.0.1f1");
    assert_eq!(found.title, "folder-name");

    write_product_name(&dir, "Mushroom Masters");
    found.update_info();
    assert_eq!(found.title, "Mushroom Masters");

    found.custom_title = Some("Mine".to_owned());
    found.update_info();
    assert_eq!(found.title, "Mine");
    assert_eq!(found.product_name, "Mushroom Masters");
}

#[test]
fn a_templates_leftover_product_name_is_ignored() {
    let tmp = TempDir::new().expect("temp dir");
    let dir = tmp.path().join("Real Name");
    let mut found = project(&dir, "6000.0.1f1");
    write_product_name(&dir, "com.unity.template.hdrp-blank");
    found.update_info();
    assert_eq!(found.title, "Real Name");
    write_product_name(&dir, "'Quoted: Name'");
    found.update_info();
    assert_eq!(found.title, "Quoted: Name");
}

#[test]
fn binary_settings_do_not_break_a_project() {
    let tmp = TempDir::new().expect("temp dir");
    let dir = tmp.path().join("Binary");
    let mut found = project(&dir, "6000.0.1f1");
    fs::write(
        dir.join("ProjectSettings").join("ProjectSettings.asset"),
        [0_u8, 159, 146, 150, 0, 255],
    )
    .expect("write settings");
    found.update_info();
    assert_eq!(found.title, "Binary");
    assert!(found.is_valid);
}

#[test]
fn the_render_pipeline_comes_from_the_packages() {
    let tmp = TempDir::new().expect("temp dir");
    let dir = tmp.path().join("Game");
    let mut found = project(&dir, "6000.0.1f1");
    assert_eq!(found.render_pipeline, "", "no packages file");
    let packages = dir.join("Packages");
    fs::create_dir(&packages).expect("create");
    for (manifest, expected) in [
        (r#"{"dependencies":{"com.unity.ugui":"2.0.0"}}"#, "Built-in"),
        (
            r#"{"dependencies":{"com.unity.render-pipelines.universal":"17"}}"#,
            "URP",
        ),
        (
            r#"{"dependencies":{"com.unity.render-pipelines.high-definition":"17"}}"#,
            "HDRP",
        ),
    ] {
        fs::write(packages.join("manifest.json"), manifest).expect("write");
        found.update_info();
        assert_eq!(found.render_pipeline, expected);
    }
}

#[test]
fn finds_a_worktrees_branch_through_its_git_file() {
    let tmp = TempDir::new().expect("temp dir");
    let real = tmp
        .path()
        .join("main-repo")
        .join(".git")
        .join("worktrees")
        .join("w");
    fs::create_dir_all(&real).expect("create repo");
    fs::write(real.join("HEAD"), "ref: refs/heads/side\n").expect("write HEAD");
    let dir = tmp.path().join("Worktree");
    fs::create_dir_all(&dir).expect("create");
    fs::write(dir.join(".git"), format!("gitdir: {}\n", real.display())).expect("write");
    assert_eq!(project(&dir, "6000.0.1f1").branch, "side");
}

#[test]
fn finds_a_plastic_workspaces_branch() {
    let tmp = TempDir::new().expect("temp dir");
    let dir = tmp.path().join("Game");
    fs::create_dir_all(dir.join(".plastic")).expect("create");
    fs::write(
        dir.join(".plastic").join("plastic.selector"),
        "rep \"game@cloud\"\n  path \"/\"\n    smartbranch \"/main/feature\"\n",
    )
    .expect("write");
    assert_eq!(project(&dir, "6000.0.1f1").branch, "main/feature");
}

fn editor_at(version: &str) -> UnityEditor {
    let mut editor = fake_editor(Path::new(&format!("/editors/{version}")));
    version.clone_into(&mut editor.version);
    editor
}

#[test]
fn moving_a_project_prefers_the_nearest_newer_editor() {
    let hub = Hub::new(
        Configuration {
            unity_search_paths: Vec::new(),
            editors_configurations: vec![
                editor_at("2021.3.1f1"),
                editor_at("6000.2.8f1"),
                editor_at("2022.3.22f1"),
            ],
        },
        Vec::new(),
    );
    let nearest = |version| hub.nearest_editor(version).map(|e| e.version.as_str());
    assert_eq!(nearest("2022.3.9f1"), Some("2022.3.22f1"));
    assert_eq!(nearest("2022.3.22f1"), Some("2022.3.22f1"));
    assert_eq!(nearest("6000.5.0f1"), Some("6000.2.8f1"), "nothing newer");
}

#[test]
fn counts_projects_per_editor_and_finds_missing_ones() {
    let tmp = TempDir::new().expect("temp dir");
    let gone = tmp.path().join("Gone");
    let mut missing = project(&gone, "6000.2.8f1");
    fs::remove_dir_all(&gone).expect("remove");
    missing.update_info();
    let present = project(&tmp.path().join("Here"), "6000.2.8f1");
    let other = project(&tmp.path().join("Other"), "2022.3.22f1");
    let editor = editor_at("6000.2.8f1");
    let hub = Hub::new(
        Configuration {
            unity_search_paths: Vec::new(),
            editors_configurations: vec![editor.clone()],
        },
        vec![missing.clone(), present, other],
    );
    assert_eq!(hub.project_count(&editor), 2);
    assert_eq!(hub.missing_projects(), [missing.path]);
}

#[cfg(unix)]
#[test]
fn launch_settings_reach_the_editor() {
    use std::os::unix::fs::PermissionsExt;

    let tmp = TempDir::new().expect("temp dir");
    let record = tmp.path().join("args.txt");
    let draft = tmp.path().join("fake-unity.tmp");
    fs::write(
        &draft,
        format!(
            "#!/bin/sh\nfor a in \"$@\"; do printf '%s\\n' \"$a\"; done > '{0}.tmp'\nmv '{0}.tmp' '{0}'\n",
            record.display()
        ),
    )
    .expect("write script");
    fs::set_permissions(&draft, fs::Permissions::from_mode(0o755)).expect("permissions");
    let exe = tmp.path().join("fake-unity");
    fs::rename(&draft, &exe).expect("move script");

    let mut found = project(&tmp.path().join("Game"), "6000.2.8f1");
    found.launch_args = r#"-force-metal -logFile "a b.txt""#.to_owned();
    found.launch_platform = Some("Android".to_owned());
    Hub::run_project(&fake_editor(&exe), &found).expect("launched");

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !record.exists() && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let args = fs::read_to_string(&record).expect("the editor ran");
    assert_eq!(
        args.lines().collect::<Vec<_>>(),
        [
            "-projectpath",
            found.path.as_str(),
            "-buildTarget",
            "android",
            "-force-metal",
            "-logFile",
            "a b.txt",
        ]
    );
}

#[test]
fn bad_launch_args_stop_the_launch() {
    let tmp = TempDir::new().expect("temp dir");
    let mut found = project(&tmp.path().join("Game"), "6000.2.8f1");
    found.launch_args = "\"unfinished".to_owned();
    let error = Hub::run_project(&fake_editor(&tmp.path().join("no-editor")), &found)
        .expect_err("bad arguments");
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
}

#[test]
fn a_config_with_every_new_field_round_trips() {
    let tmp = TempDir::new().expect("temp dir");
    let mut kept = project(&tmp.path().join("Kept"), "6000.0.1f1");
    kept.custom_title = Some("Mine".to_owned());
    kept.launch_args = "-a".to_owned();
    kept.launch_platform = Some("iOS".to_owned());
    let mut editor = editor_at("6000.0.1f1");
    editor.templates.push(ProjectTemplate {
        path: "/t.tgz".to_owned(),
        title: "t".to_owned(),
        display_name: None,
        description: Some("d".to_owned()),
    });
    let mut hub = Hub::new(
        Configuration {
            unity_search_paths: Vec::new(),
            editors_configurations: vec![editor],
        },
        vec![kept.clone()],
    );
    hub.preferred_editor = Some("/editors/6000.0.1f1".to_owned());

    let text = toml::to_string(&hub).expect("config serializes");
    let back: Hub = toml::from_str(&text).expect("config parses");
    assert_eq!(back.preferred_editor, hub.preferred_editor);
    assert_eq!(back.projects[0].custom_title.as_deref(), Some("Mine"));
    assert_eq!(back.projects[0].launch_args, "-a");
    assert_eq!(back.projects[0].launch_platform.as_deref(), Some("iOS"));
    assert_eq!(
        back.config.editors_configurations[0].templates[0].display_name,
        None
    );
}
