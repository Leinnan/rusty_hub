//! Project settings, changing version, removing missing projects, and the
//! editors page's additions.

use eframe::egui::{Key, Role};
use egui_kittest::{
    Harness,
    kittest::{NodeT, Queryable},
};
use tempfile::TempDir;
use unity_hub::{config::Configuration, hub::Hub, unity_project::UnityProject};

use super::{editor, harness, harness_with, project, sample_hub, settle};
use crate::{
    app::HubApp,
    commands::{EditorCmd, ProjectCmd, ViewCmd},
    state::{Page, ProjectSettingsForm},
};

const MUSHROOM: &str = "/work/games/Mushroom Masters";

fn settings_form(harness: &Harness<'static, HubApp>) -> ProjectSettingsForm {
    harness
        .state()
        .state()
        .project_settings
        .clone()
        .expect("open")
}

#[test]
fn settings_are_kept_and_change_the_title() {
    let mut harness = harness();
    settle(&mut harness);
    ProjectCmd::RequestSettings(MUSHROOM.to_owned()).schedule();
    settle(&mut harness);
    assert_eq!(settings_form(&harness).launch_args, "");

    let form = harness.state_mut().state_mut().project_settings.as_mut();
    let form = form.expect("open");
    form.title = "  Fungi  ".to_owned();
    form.launch_args = "-force-metal".to_owned();
    form.platform = Some("Android".to_owned());
    settle(&mut harness);
    harness.get_by_role_and_label(Role::Button, "Save").click();
    settle(&mut harness);

    let state = harness.state().state();
    assert!(state.project_settings.is_none());
    let saved = state.project(MUSHROOM).expect("a project");
    assert_eq!(saved.title, "Fungi");
    assert_eq!(saved.custom_title.as_deref(), Some("Fungi"));
    assert_eq!(saved.launch_args, "-force-metal");
    assert_eq!(saved.launch_platform.as_deref(), Some("Android"));

    // Clearing the name goes back to the folder's.
    ProjectCmd::SaveSettings(ProjectSettingsForm {
        project: MUSHROOM.to_owned(),
        title: String::new(),
        launch_args: String::new(),
        platform: None,
    })
    .schedule();
    settle(&mut harness);
    assert_eq!(
        harness
            .state()
            .state()
            .project(MUSHROOM)
            .expect("a project")
            .title,
        "Mushroom Masters"
    );
}

#[test]
fn an_unfinished_quote_stops_the_settings_being_saved() {
    let mut harness = harness();
    settle(&mut harness);
    ProjectCmd::RequestSettings(MUSHROOM.to_owned()).schedule();
    settle(&mut harness);
    let form = harness.state_mut().state_mut().project_settings.as_mut();
    form.expect("open").launch_args = "\"oops".to_owned();
    settle(&mut harness);
    assert!(harness.query_by_label_contains("isn't closed").is_some());
    assert!(
        harness
            .get_by_role_and_label(Role::Button, "Save")
            .accesskit_node()
            .is_disabled()
    );
}

#[test]
fn shortcuts_wait_while_settings_are_open() {
    let mut harness = harness();
    settle(&mut harness);
    ProjectCmd::RequestSettings(MUSHROOM.to_owned()).schedule();
    settle(&mut harness);
    harness.key_press(Key::ArrowDown);
    settle(&mut harness);
    assert_eq!(harness.state().state().selected, None);
    harness.key_press(Key::Escape);
    settle(&mut harness);
    assert!(harness.state().state().project_settings.is_none());
}

fn real_project(dir: &std::path::Path, version: &str) -> UnityProject {
    let settings = dir.join("ProjectSettings");
    std::fs::create_dir_all(&settings).expect("create");
    std::fs::write(
        settings.join("ProjectVersion.txt"),
        format!("m_EditorVersion: {version}\n"),
    )
    .expect("write");
    UnityProject::try_get_project_at_path(&dir.to_string_lossy()).expect("a project")
}

fn hub_with_two_editors(projects: Vec<UnityProject>) -> Hub {
    Hub::new(
        Configuration {
            unity_search_paths: Vec::new(),
            editors_configurations: vec![
                editor("2022.3.22f1", &["iOS"]),
                editor("6000.2.8f1", &["Android"]),
            ],
        },
        projects,
    )
}

#[test]
fn change_version_starts_on_the_nearest_newer_editor_and_warns() {
    let game = project("Game", "2021.3.1f1", "", 1);
    let path = game.path.clone();
    let mut harness = harness_with(hub_with_two_editors(vec![game]), 1100.0, 700.0);
    settle(&mut harness);
    ProjectCmd::RequestChangeVersion(path).schedule();
    settle(&mut harness);
    let state = harness.state().state();
    let form = state.change_version.as_ref().expect("open");
    assert_eq!(
        state.editor(&form.editor).expect("an editor").version,
        "2022.3.22f1"
    );
    assert!(
        harness
            .query_by_label_contains("Commit or back up")
            .is_some()
    );
    harness.snapshot("change_version");
}

#[test]
fn keeping_the_same_version_offers_nothing_to_do() {
    let tmp = TempDir::new().expect("temp dir");
    let game = real_project(&tmp.path().join("Game"), "6000.2.8f1");
    let path = game.path.clone();
    let mut harness = harness_with(hub_with_two_editors(vec![game]), 1100.0, 700.0);
    settle(&mut harness);
    ProjectCmd::RequestChangeVersion(path).schedule();
    settle(&mut harness);
    assert!(
        harness
            .get_by_role_and_label(Role::Button, "Change and Open")
            .accesskit_node()
            .is_disabled()
    );
}

#[test]
fn a_failed_launch_puts_the_version_back() {
    let tmp = TempDir::new().expect("temp dir");
    let game = real_project(&tmp.path().join("Game"), "2021.3.1f1");
    let path = game.path.clone();
    let mut harness = harness_with(hub_with_two_editors(vec![game]), 1100.0, 700.0);
    settle(&mut harness);
    ProjectCmd::RequestChangeVersion(path.clone()).schedule();
    settle(&mut harness);
    harness
        .get_by_role_and_label(Role::Button, "Change and Open")
        .click();
    settle(&mut harness);
    // Fixture editors can't launch, which is reported.
    assert!(harness.query_by_label_contains("Couldn't change").is_some());
    let state = harness.state().state();
    assert!(state.change_version.is_none());
    assert_eq!(
        state.project(&path).expect("a project").version,
        "2021.3.1f1"
    );
}

#[test]
fn removing_missing_projects_asks_then_removes_only_those() {
    let tmp = TempDir::new().expect("temp dir");
    let kept = real_project(&tmp.path().join("Kept"), "6000.2.8f1");
    let mut gone = project("Gone", "6000.2.8f1", "", 3);
    gone.is_valid = false;
    let hub = hub_with_two_editors(vec![kept.clone(), gone.clone()]);
    let mut harness = harness_with(hub, 1100.0, 700.0);
    settle(&mut harness);
    ProjectCmd::RequestRemoveMissing.schedule();
    settle(&mut harness);
    assert!(
        harness
            .query_by_label_contains("whose folders are gone")
            .is_some()
    );
    assert_eq!(harness.state().state().hub.projects.len(), 2, "not yet");

    harness
        .get_by_role_and_label(Role::Button, "Remove")
        .click();
    settle(&mut harness);
    let state = harness.state().state();
    assert_eq!(state.hub.projects, [kept]);
    assert!(state.hub.hidden_projects.contains(&gone.path));
    assert!(tmp.path().join("Kept").is_dir());
    assert!(!state.confirm_remove_missing);
}

#[test]
fn the_details_panel_shows_what_was_set() {
    let mut hub = sample_hub();
    let first = &mut hub.projects[0];
    first.render_pipeline = "URP".to_owned();
    first.launch_args = "-force-metal".to_owned();
    first.launch_platform = Some("iOS".to_owned());
    first.title = "Fungi".to_owned();
    let mut harness = harness_with(hub, 1100.0, 700.0);
    settle(&mut harness);
    ViewCmd::Select(MUSHROOM.to_owned()).schedule();
    settle(&mut harness);
    assert!(harness.query_by_label("URP").is_some());
    assert!(harness.query_by_label("-force-metal").is_some());
    assert!(
        harness.query_by_label("Mushroom Masters").is_some(),
        "the folder"
    );
    harness.snapshot("project_details_full");
}

#[test]
fn the_preferred_editor_comes_first_in_new_projects() {
    let tmp = TempDir::new().expect("temp dir");
    let hub = hub_with_two_editors(Vec::new());
    let mut harness = harness_with(hub, 1100.0, 700.0);
    harness
        .state_mut()
        .state_mut()
        .new_project_defaults
        .location = Some(tmp.path().to_string_lossy().into_owned());
    settle(&mut harness);
    EditorCmd::SetPreferred(Some("/nonexistent/2022.3.22f1/Unity".to_owned())).schedule();
    settle(&mut harness);
    harness.state_mut().state_mut().page = Page::Editors;
    settle(&mut harness);
    assert!(harness.query_by_label("Preferred").is_some());

    ProjectCmd::RequestNew(None).schedule();
    settle(&mut harness);
    let state = harness.state().state();
    let form = state.new_project.as_ref().expect("open");
    assert_eq!(
        state.editor(&form.editor).expect("an editor").version,
        "2022.3.22f1"
    );
}

#[test]
fn a_failed_editor_launch_is_reported() {
    let mut harness = harness();
    settle(&mut harness);
    harness.state_mut().state_mut().page = Page::Editors;
    settle(&mut harness);
    harness
        .get_by_role_and_label(Role::Button, "Open Unity 6000.2.8f1")
        .click();
    settle(&mut harness);
    assert!(
        harness
            .query_by_label_contains("Couldn't open Unity")
            .is_some()
    );
}

#[test]
fn the_editors_page_counts_projects_and_lists_unity_folders() {
    let mut harness = harness();
    harness.state_mut().state_mut().page = Page::Editors;
    harness.state_mut().state_mut().unity_folders = vec![(
        "Editor Logs".to_owned(),
        "/home/me/Library/Logs/Unity".to_owned(),
    )];
    settle(&mut harness);
    assert!(harness.query_by_label("2 projects").is_some());
    assert!(harness.query_by_label("Editor Logs").is_some());
    harness.snapshot("editors_full");
}
