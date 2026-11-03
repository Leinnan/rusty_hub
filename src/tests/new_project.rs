//! The New Project dialog.

use std::path::Path;

use eframe::egui::{Key, Modifiers, Role};
use egui_kittest::{
    Harness,
    kittest::{NodeT, Queryable},
};
use tempfile::TempDir;
use unity_hub::{
    config::Configuration, hub::Hub, online_templates::OnlineTemplate,
    project_template::ProjectTemplate, unity_editor::UnityEditor,
};

use super::{editor, harness_with, project, settle};
use crate::{
    app::{HubApp, online::OnlineResult},
    commands::ProjectCmd,
    state::{CatalogStatus, Page, TemplateSource},
};

fn template(name: &str, description: &str) -> ProjectTemplate {
    ProjectTemplate {
        path: format!("/templates/{name}.tgz"),
        title: format!("com.unity.template.{name}"),
        display_name: Some(name.to_owned()),
        description: Some(description.to_owned()),
    }
}

fn editor_with_templates(version: &str) -> UnityEditor {
    let mut editor = editor(version, &["Android", "iOS", "macOS"]);
    editor.templates = vec![
        template(
            "3D URP",
            "Settings and assets for the Universal Render Pipeline.",
        ),
        template("Universal 2D", "An empty 2D project using URP."),
    ];
    editor
}

fn hub_with(editors: Vec<UnityEditor>) -> Hub {
    Hub::new(
        Configuration {
            unity_search_paths: Vec::new(),
            editors_configurations: editors,
        },
        vec![project("Existing", "6000.2.8f1", "", 1)],
    )
}

/// An app whose new projects go in `location`.
fn harness_in(location: &Path, hub: Hub, width: f32, height: f32) -> Harness<'static, HubApp> {
    let mut harness = harness_with(hub, width, height);
    harness
        .state_mut()
        .state_mut()
        .new_project_defaults
        .location = Some(location.to_string_lossy().into_owned());
    settle(&mut harness);
    harness
}

fn open_dialog(location: &Path) -> Harness<'static, HubApp> {
    let hub = hub_with(vec![
        editor_with_templates("2022.3.22f1"),
        editor_with_templates("6000.2.8f1"),
    ]);
    let mut harness = harness_in(location, hub, 1100.0, 700.0);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::N);
    settle(&mut harness);
    assert!(harness.state().state().new_project.is_some());
    harness
}

fn create_enabled(harness: &Harness<'static, HubApp>) -> bool {
    !harness
        .get_by_role_and_label(Role::Button, "Create")
        .accesskit_node()
        .is_disabled()
}

fn set_name(harness: &mut Harness<'static, HubApp>, name: &str) {
    let form = harness.state_mut().state_mut().new_project.as_mut();
    let form = form.expect("the dialog is open");
    form.name = name.to_owned();
    form.name_edited = true;
    form.validate();
    settle(harness);
}

#[test]
fn opens_with_a_free_name_and_the_newest_editor() {
    let tmp = TempDir::new().expect("temp dir");
    std::fs::create_dir(tmp.path().join("My Project")).expect("create folder");
    let harness = open_dialog(tmp.path());
    let state = harness.state().state();
    let form = state.new_project.as_ref().expect("open");
    assert_eq!(form.name, "My Project 2");
    assert_eq!(
        state.editor(&form.editor).expect("an editor").version,
        "6000.2.8f1"
    );
    assert!(create_enabled(&harness));
}

#[test]
fn an_editor_row_opens_the_dialog_with_that_editor() {
    let tmp = TempDir::new().expect("temp dir");
    let hub = hub_with(vec![
        editor_with_templates("2022.3.22f1"),
        editor_with_templates("6000.2.8f1"),
    ]);
    let mut harness = harness_in(tmp.path(), hub, 1100.0, 700.0);
    harness.state_mut().state_mut().page = Page::Editors;
    settle(&mut harness);
    harness
        .get_by_role_and_label(Role::Button, "New project with 2022.3.22f1")
        .click();
    settle(&mut harness);
    let state = harness.state().state();
    let form = state.new_project.as_ref().expect("open");
    assert_eq!(
        state.editor(&form.editor).expect("an editor").version,
        "2022.3.22f1"
    );
}

#[test]
fn the_empty_state_offers_a_new_project() {
    let tmp = TempDir::new().expect("temp dir");
    let hub = Hub::new(
        Configuration {
            unity_search_paths: Vec::new(),
            editors_configurations: vec![editor_with_templates("6000.2.8f1")],
        },
        Vec::new(),
    );
    let mut harness = harness_in(tmp.path(), hub, 1100.0, 700.0);
    harness
        .get_by_role_and_label(Role::Button, "New Project…")
        .click();
    settle(&mut harness);
    assert!(harness.state().state().new_project.is_some());
}

#[test]
fn with_no_editors_it_points_to_the_editors_page() {
    let tmp = TempDir::new().expect("temp dir");
    let mut harness = harness_in(tmp.path(), hub_with(Vec::new()), 1100.0, 700.0);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::N);
    settle(&mut harness);
    assert!(harness.state().state().new_project.is_none());
    harness
        .get_by_role_and_label(Role::Button, "Go to Editors")
        .click();
    settle(&mut harness);
    assert_eq!(harness.state().state().page, Page::Editors);
    assert!(!harness.state().state().new_project_blocked);
}

#[test]
fn bad_names_disable_create_and_enter_does_nothing() {
    let tmp = TempDir::new().expect("temp dir");
    let mut harness = open_dialog(tmp.path());
    set_name(&mut harness, "../escape");
    assert!(!create_enabled(&harness));
    assert!(harness.query_by_label_contains("can't contain").is_some());

    harness.key_press(Key::Enter);
    settle(&mut harness);
    assert!(harness.state().state().new_project.is_some(), "still open");
    assert_eq!(std::fs::read_dir(tmp.path()).expect("list").count(), 0);
}

#[test]
fn a_taken_name_disables_create() {
    let tmp = TempDir::new().expect("temp dir");
    std::fs::create_dir(tmp.path().join("Taken")).expect("create folder");
    let mut harness = open_dialog(tmp.path());
    set_name(&mut harness, "Taken");
    assert!(!create_enabled(&harness));
    assert!(harness.query_by_label_contains("already exists").is_some());
}

#[test]
fn a_typed_name_survives_changing_the_editor() {
    let tmp = TempDir::new().expect("temp dir");
    let mut harness = open_dialog(tmp.path());
    set_name(&mut harness, "Keep Me");
    let older = harness.state().state().hub.config.editors_configurations[0].clone();
    let form = harness.state_mut().state_mut().new_project.as_mut();
    form.expect("open").set_editor(&older);
    settle(&mut harness);
    assert_eq!(
        harness
            .state()
            .state()
            .new_project
            .as_ref()
            .expect("open")
            .name,
        "Keep Me"
    );
}

#[test]
fn changing_editor_drops_a_template_it_does_not_have() {
    let mut other = editor("2021.3.1f1", &[]);
    other.templates = Vec::new();
    let mut form = crate::state::NewProjectForm {
        name: "Game".to_owned(),
        name_edited: true,
        location: "/work".to_owned(),
        editor: "x".to_owned(),
        template: Some("/templates/3D URP.tgz".to_owned()),
        source: crate::state::TemplateSource::Installed,
        platform: Some("iOS".to_owned()),
        init_git: false,
        problem: None,
        focus_name: false,
    };
    form.set_editor(&other);
    assert_eq!(form.template, None);
    assert_eq!(form.platform, None);
}

#[test]
fn picking_a_template_shows_its_description() {
    let tmp = TempDir::new().expect("temp dir");
    let mut harness = open_dialog(tmp.path());
    assert!(
        harness
            .query_by_label_contains("Settings and assets")
            .is_none()
    );
    harness
        .get_by_role_and_label(Role::Button, "3D URP")
        .click();
    settle(&mut harness);
    assert!(
        harness
            .query_by_label_contains("Settings and assets")
            .is_some()
    );
    assert_eq!(
        harness
            .state()
            .state()
            .new_project
            .as_ref()
            .expect("open")
            .template
            .as_deref(),
        Some("/templates/3D URP.tgz")
    );
}

#[test]
fn a_failed_launch_keeps_the_dialog_and_says_why() {
    let tmp = TempDir::new().expect("temp dir");
    let mut harness = open_dialog(tmp.path());
    set_name(&mut harness, "Game");
    harness
        .get_by_role_and_label(Role::Button, "Create")
        .click();
    settle(&mut harness);
    let state = harness.state().state();
    assert!(state.new_project.is_some(), "nothing typed is lost");
    assert_eq!(state.hub.projects.len(), 1);
    assert!(!tmp.path().join("Game").exists());
    assert!(
        harness
            .query_by_label_contains("Couldn't create Game")
            .is_some()
    );
}

#[test]
fn escape_closes_the_dialog() {
    let tmp = TempDir::new().expect("temp dir");
    let mut harness = open_dialog(tmp.path());
    harness.key_press(Key::Escape);
    settle(&mut harness);
    assert!(harness.state().state().new_project.is_none());
}

#[test]
fn the_git_option_only_appears_when_git_works() {
    let tmp = TempDir::new().expect("temp dir");
    let mut harness = open_dialog(tmp.path());
    let option = "Initialize git repository";
    assert!(harness.query_by_label(option).is_none(), "not known yet");
    harness
        .state_mut()
        .deliver_online(OnlineResult::GitChecked(false));
    settle(&mut harness);
    assert!(harness.query_by_label(option).is_none());
    harness
        .state_mut()
        .deliver_online(OnlineResult::GitChecked(true));
    settle(&mut harness);
    assert!(harness.query_by_label(option).is_some());
}

#[cfg(unix)]
#[test]
fn creating_lists_selects_and_remembers() {
    use std::os::unix::fs::PermissionsExt;

    let tmp = TempDir::new().expect("temp dir");
    let exe = tmp.path().join("fake-unity");
    std::fs::write(&exe, "#!/bin/sh\nexit 0\n").expect("write script");
    std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o755)).expect("permissions");
    let location = tmp.path().join("projects");

    let mut installed = editor_with_templates("6000.2.8f1");
    installed.exe_path = exe.to_string_lossy().into_owned();
    let mut harness = harness_in(&location, hub_with(vec![installed]), 1100.0, 700.0);
    harness.state_mut().state_mut().search = "nothing matches this".to_owned();
    ProjectCmd::RequestNew(None).schedule();
    settle(&mut harness);
    set_name(&mut harness, "Fresh Game");
    harness
        .get_by_role_and_label(Role::Button, "3D URP")
        .click();
    settle(&mut harness);
    harness
        .get_by_role_and_label(Role::Button, "Create")
        .click();
    settle(&mut harness);

    let state = harness.state().state();
    let path = location.join("Fresh Game").to_string_lossy().into_owned();
    assert!(state.new_project.is_none());
    assert_eq!(state.selected.as_deref(), Some(path.as_str()));
    assert!(state.search.is_empty(), "the new row must be in sight");
    assert_eq!(state.hub.projects.len(), 2);
    let defaults = &state.new_project_defaults;
    assert_eq!(defaults.template.as_deref(), Some("3D URP"));
    assert_eq!(
        defaults.location.as_deref(),
        Some(&*location.to_string_lossy())
    );
    assert!(
        harness
            .query_by_label_contains("Creating Fresh Game")
            .is_some()
    );
}

fn online_template(name: &str, package: &str, kind: &str) -> OnlineTemplate {
    OnlineTemplate {
        name: name.to_owned(),
        package_name: package.to_owned(),
        description: format!("All about {name}."),
        kind: kind.to_owned(),
        render_pipeline: "URP".to_owned(),
        version: "1.0.0".to_owned(),
        url: format!("https://download.packages.unity.com/{package}.tgz"),
    }
}

/// The dialog on its Online list, with no catalog yet.
fn open_online(cache: &Path) -> Harness<'static, HubApp> {
    let mut harness = open_dialog(cache);
    harness.state_mut().state_mut().online.cache_dir = Some(cache.to_path_buf());
    harness
        .get_by_role_and_label(Role::Button, "Online")
        .click();
    settle(&mut harness);
    harness
}

fn deliver_catalog(harness: &mut Harness<'static, HubApp>, templates: Vec<OnlineTemplate>) {
    harness.state_mut().deliver_online(OnlineResult::Catalog {
        base: "6000.2".to_owned(),
        result: Ok(templates),
    });
    settle(harness);
}

#[test]
fn nothing_is_asked_online_until_the_online_list_is_opened() {
    let tmp = TempDir::new().expect("temp dir");
    let harness = open_dialog(tmp.path());
    let online = &harness.state().state().online;
    assert_eq!(online.status, CatalogStatus::Idle);
    assert_eq!(online.base, None);
}

#[test]
fn opening_the_online_list_asks_for_the_editors_version() {
    let tmp = TempDir::new().expect("temp dir");
    let harness = open_online(tmp.path());
    let online = &harness.state().state().online;
    assert_eq!(online.status, CatalogStatus::Loading);
    assert_eq!(online.base.as_deref(), Some("6000.2"));
    assert!(harness.query_by_label_contains("Asking Unity").is_some());
}

#[test]
fn a_late_reply_for_another_editor_is_ignored() {
    let tmp = TempDir::new().expect("temp dir");
    let mut harness = open_online(tmp.path());
    harness.state_mut().deliver_online(OnlineResult::Catalog {
        base: "2022.3".to_owned(),
        result: Ok(vec![online_template("Old", "old", "CORE")]),
    });
    settle(&mut harness);
    assert_eq!(
        harness.state().state().online.status,
        CatalogStatus::Loading
    );
}

#[test]
fn a_failed_catalog_says_why_and_can_be_retried() {
    let tmp = TempDir::new().expect("temp dir");
    let mut harness = open_online(tmp.path());
    harness.state_mut().deliver_online(OnlineResult::Catalog {
        base: "6000.2".to_owned(),
        result: Err("no network".to_owned()),
    });
    settle(&mut harness);
    assert!(harness.query_by_label_contains("no network").is_some());
    harness
        .get_by_role_and_label(Role::Button, "Try Again")
        .click();
    settle(&mut harness);
    assert_eq!(
        harness.state().state().online.status,
        CatalogStatus::Loading
    );

    // The installed list still works.
    harness
        .get_by_role_and_label(Role::Button, "Installed")
        .click();
    settle(&mut harness);
    assert!(
        harness
            .query_by_role_and_label(Role::Button, "3D URP")
            .is_some()
    );
}

#[test]
fn downloading_shows_progress_then_selects_the_template() {
    let tmp = TempDir::new().expect("temp dir");
    let mut harness = open_online(tmp.path());
    deliver_catalog(
        &mut harness,
        vec![online_template("Fancy", "com.fancy", "CORE")],
    );
    assert!(
        harness
            .query_by_label_contains("Core · URP · 1.0.0")
            .is_some()
    );
    harness
        .get_by_role_and_label(Role::Button, "Download")
        .click();
    settle(&mut harness);
    let download = harness
        .state()
        .state()
        .online
        .download
        .clone()
        .expect("started");
    assert_eq!(download.package, "com.fancy");
    download
        .progress
        .store(12_300_000, std::sync::atomic::Ordering::Relaxed);
    settle(&mut harness);
    assert!(harness.query_by_label("12.3 MB").is_some());

    let path = tmp.path().join("com.fancy-1.0.0.tgz");
    std::fs::write(&path, "x").expect("write file");
    harness
        .state_mut()
        .deliver_online(OnlineResult::Downloaded {
            package: "com.fancy".to_owned(),
            result: Ok(path.clone()),
        });
    settle(&mut harness);
    let state = harness.state().state();
    assert!(state.online.download.is_none());
    assert_eq!(
        state
            .new_project
            .as_ref()
            .expect("open")
            .template
            .as_deref(),
        Some(&*path.to_string_lossy())
    );
    assert!(harness.query_by_label_contains("All about Fancy").is_some());
    assert!(
        harness
            .query_by_role_and_label(Role::Button, "Download")
            .is_none()
    );
}

#[test]
fn a_failed_download_says_why_on_its_row() {
    let tmp = TempDir::new().expect("temp dir");
    let mut harness = open_online(tmp.path());
    deliver_catalog(
        &mut harness,
        vec![online_template("Fancy", "com.fancy", "CORE")],
    );
    harness
        .get_by_role_and_label(Role::Button, "Download")
        .click();
    settle(&mut harness);
    harness
        .state_mut()
        .deliver_online(OnlineResult::Downloaded {
            package: "com.fancy".to_owned(),
            result: Err("The download isn't a Unity template".to_owned()),
        });
    settle(&mut harness);
    assert!(
        harness
            .query_by_label_contains("isn't a Unity template")
            .is_some()
    );
    assert_eq!(
        harness
            .state()
            .state()
            .new_project
            .as_ref()
            .expect("open")
            .template,
        None
    );
}

#[test]
fn closing_the_dialog_cancels_the_download() {
    let tmp = TempDir::new().expect("temp dir");
    let mut harness = open_online(tmp.path());
    deliver_catalog(
        &mut harness,
        vec![online_template("Fancy", "com.fancy", "CORE")],
    );
    harness
        .get_by_role_and_label(Role::Button, "Download")
        .click();
    settle(&mut harness);
    let download = harness
        .state()
        .state()
        .online
        .download
        .clone()
        .expect("started");
    harness.key_press(Key::Escape);
    settle(&mut harness);
    assert!(harness.state().state().new_project.is_none());
    assert!(download.cancel.load(std::sync::atomic::Ordering::Relaxed));
}

#[test]
fn switching_lists_never_leaves_a_hidden_choice() {
    let tmp = TempDir::new().expect("temp dir");
    let mut harness = open_dialog(tmp.path());
    harness
        .get_by_role_and_label(Role::Button, "3D URP")
        .click();
    settle(&mut harness);
    harness
        .get_by_role_and_label(Role::Button, "Online")
        .click();
    settle(&mut harness);
    let form = harness.state().state().new_project.clone().expect("open");
    assert_eq!(form.source, TemplateSource::Online);
    assert_eq!(form.template, None);
}

#[test]
fn clearing_downloads_forgets_the_selected_one() {
    let tmp = TempDir::new().expect("temp dir");
    let mut harness = open_online(tmp.path());
    deliver_catalog(
        &mut harness,
        vec![online_template("Fancy", "com.fancy", "CORE")],
    );
    let path = tmp.path().join("com.fancy-1.0.0.tgz");
    std::fs::write(&path, "x").expect("write file");
    harness
        .state_mut()
        .state_mut()
        .new_project
        .as_mut()
        .expect("open")
        .template = Some(path.to_string_lossy().into_owned());
    settle(&mut harness);
    harness
        .get_by_role_and_label(Role::Button, "Clear Downloaded Templates")
        .click();
    settle(&mut harness);
    assert!(!path.exists());
    assert_eq!(
        harness
            .state()
            .state()
            .new_project
            .as_ref()
            .expect("open")
            .template,
        None
    );
}

#[test]
fn online_dialog_snapshot() {
    let cache = Path::new("/nonexistent/template-cache");
    let mut harness = open_online(cache);
    deliver_catalog(
        &mut harness,
        vec![
            online_template("Universal 3D", "com.unity.template.urp", "CORE"),
            online_template("FPS Microgame", "com.unity.template.fps", "LEARNING"),
        ],
    );
    harness.snapshot("new_project_online");
}

/// Snapshots need the same text everywhere, so the location is made up.
fn snapshot_harness(width: f32, height: f32) -> Harness<'static, HubApp> {
    let hub = hub_with(vec![editor_with_templates("6000.2.8f1")]);
    let mut harness = harness_in(Path::new("/work/games"), hub, width, height);
    ProjectCmd::RequestNew(None).schedule();
    settle(&mut harness);
    harness
        .get_by_role_and_label(Role::Button, "3D URP")
        .click();
    settle(&mut harness);
    harness
}

#[test]
fn dialog_snapshot() {
    snapshot_harness(1100.0, 700.0).snapshot("new_project");
}

#[test]
fn dialog_fits_the_smallest_window() {
    snapshot_harness(760.0, 480.0).snapshot("new_project_narrow");
}
