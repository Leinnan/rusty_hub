//! Templates from Unity's online catalog, as the Unity Hub lists them.
//!
//! The catalog's API is public but undocumented, so it can change without
//! notice: everything here fails with a message rather than taking anything
//! else down. Requests go through the system's `curl`, which keeps TLS out of
//! this crate's dependencies. Replies are treated as untrusted.

use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use crate::project_template::read_manifest;

const ENDPOINT: &str = "https://live-platform-api.prd.ld.unity3d.com/graphql";
/// Where template archives may come from.
const DOWNLOAD_PREFIX: &str = "https://download.packages.unity.com/";
const QUERY: &str = "fragment TemplateEntity on Template { name packageName description type \
    renderPipeline versions { name tarball { url } } } \
    query HUB__getTemplates($limit: Int! $skip: Int! $orderBy: TemplateOrder! \
    $supportedUnityEditorVersions: [String!]!) { getTemplates(limit: $limit skip: $skip \
    orderBy: $orderBy supportedUnityEditorVersions: $supportedUnityEditorVersions) \
    { edges { node { ...TemplateEntity } } } }";
const PAGE_SIZE: u32 = 40;
/// Largest catalog reply to accept, in bytes.
const MAX_REPLY: &str = "10485760";
/// Largest template to accept, in bytes.
const MAX_DOWNLOAD: &str = "4294967296";
/// How often a running download reports its progress.
const POLL: Duration = Duration::from_millis(100);

/// A template in the catalog, at the version to use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnlineTemplate {
    /// The name to show.
    pub name: String,
    /// The package's id, which is safe to use in a file name.
    pub package_name: String,
    /// What the template is for.
    pub description: String,
    /// The catalog's category, like `CORE`.
    pub kind: String,
    /// The render pipeline, like `URP`.
    pub render_pipeline: String,
    /// The version to use, which is safe to use in a file name.
    pub version: String,
    /// Where the archive is.
    pub url: String,
}

impl OnlineTemplate {
    /// Where this template lives once downloaded to `cache_dir`.
    #[must_use]
    pub fn cached_path(&self, cache_dir: &Path) -> PathBuf {
        cache_dir.join(format!("{}-{}.tgz", self.package_name, self.version))
    }
}

/// The part of an editor's version the catalog goes by: `6000.2.8f1` is
/// `6000.2`.
#[must_use]
pub fn base_version(version: &str) -> String {
    let mut parts = version.split('.');
    match (parts.next(), parts.next()) {
        (Some(major), Some(minor)) => format!("{major}.{minor}"),
        _ => version.to_owned(),
    }
}

/// Only characters that can't climb out of a folder or mean anything to a
/// file system.
fn is_safe_file_part(text: &str) -> bool {
    !text.is_empty()
        && !text.starts_with('.')
        && text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
}

/// The version to use from a catalog entry's list, newest first: the newest
/// release, or else the newest pre-release. Entries that can't be used safely
/// are skipped.
fn pick_version(versions: &[serde_json::Value]) -> Option<(String, String)> {
    let usable: Vec<(String, String)> = versions
        .iter()
        .filter_map(|version| {
            let name = version.get("name")?.as_str()?;
            let url = version.get("tarball")?.get("url")?.as_str()?;
            (is_safe_file_part(name) && url.starts_with(DOWNLOAD_PREFIX))
                .then(|| (name.to_owned(), url.to_owned()))
        })
        .collect();
    usable
        .iter()
        .find(|(name, _)| !name.contains('-'))
        .or_else(|| usable.first())
        .cloned()
}

/// Read a catalog reply. Entries that are incomplete or unsafe are left out.
pub fn parse_catalog(reply: &str) -> Result<Vec<OnlineTemplate>, String> {
    let json: serde_json::Value =
        serde_json::from_str(reply).map_err(|_| "Unity's catalog sent something unexpected")?;
    let edges = json
        .pointer("/data/getTemplates/edges")
        .and_then(serde_json::Value::as_array)
        .ok_or("Unity's catalog sent something unexpected")?;
    Ok(edges
        .iter()
        .filter_map(|edge| {
            let node = edge.get("node")?;
            let text = |field: &str| {
                node.get(field)
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_owned()
            };
            let package_name = node.get("packageName")?.as_str()?.to_owned();
            if !is_safe_file_part(&package_name) {
                return None;
            }
            let (version, url) = pick_version(node.get("versions")?.as_array()?)?;
            Some(OnlineTemplate {
                name: node.get("name")?.as_str()?.to_owned(),
                package_name,
                description: text("description"),
                kind: text("type"),
                render_pipeline: text("renderPipeline"),
                version,
                url,
            })
        })
        .collect())
}

/// The options every request shares. `-q` ignores the user's `.curlrc`, and
/// only HTTPS is allowed, through redirects too.
fn curl(program: &Path) -> Command {
    let mut command = Command::new(program);
    command.args([
        "-q",
        "--silent",
        "--show-error",
        "--fail",
        "--location",
        "--proto",
        "=https",
        "--proto-redir",
        "=https",
        "--connect-timeout",
        "20",
    ]);
    hide_console(&mut command);
    command
}

/// A GUI app's child would otherwise flash a console window on Windows.
#[cfg(windows)]
pub(crate) fn hide_console(command: &mut Command) {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
pub(crate) fn hide_console(_command: &mut Command) {}

fn describe_spawn_error(error: &io::Error) -> String {
    if error.kind() == io::ErrorKind::NotFound {
        "Online templates need the curl command, which wasn't found".to_owned()
    } else {
        format!("Couldn't run curl: {error}")
    }
}

/// The first line curl wrote about a failure.
fn describe_failure(stderr: &[u8]) -> String {
    let text = String::from_utf8_lossy(stderr);
    let line = text.lines().find(|line| !line.trim().is_empty());
    line.map_or_else(
        || "The download failed".to_owned(),
        |line| line.trim().trim_start_matches("curl: ").to_owned(),
    )
}

/// List the templates for an editor's version. Blocks on the network.
pub fn fetch(editor_version: &str) -> Result<Vec<OnlineTemplate>, String> {
    fetch_with(Path::new("curl"), editor_version)
}

fn fetch_with(program: &Path, editor_version: &str) -> Result<Vec<OnlineTemplate>, String> {
    let body = serde_json::json!({
        "query": QUERY,
        "variables": {
            "limit": PAGE_SIZE,
            "skip": 0,
            "orderBy": "WEIGHTED_DESC",
            "supportedUnityEditorVersions": [base_version(editor_version)],
        },
    });
    let mut child = curl(program)
        .args(["--max-time", "30", "--max-filesize", MAX_REPLY])
        .args(["-X", "POST", "--data-binary", "@-"])
        .args(["-H", "Content-Type: application/json"])
        .args(["-H", "Accept: application/json"])
        .arg(ENDPOINT)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| describe_spawn_error(&error))?;
    if let Some(mut stdin) = child.stdin.take() {
        // A failure here shows up as curl's own failure below.
        let _ = stdin.write_all(body.to_string().as_bytes());
    }
    let output = child
        .wait_with_output()
        .map_err(|error| describe_spawn_error(&error))?;
    if !output.status.success() {
        return Err(describe_failure(&output.stderr));
    }
    parse_catalog(&String::from_utf8_lossy(&output.stdout))
}

/// Download `template` into `cache_dir` and return where it is. Only a complete
/// file that reads as a template package appears there. Blocks until done;
/// `progress` follows the bytes so far, and setting `cancel` stops it.
pub fn download(
    template: &OnlineTemplate,
    cache_dir: &Path,
    progress: &AtomicU64,
    cancel: &AtomicBool,
) -> Result<PathBuf, String> {
    download_with(Path::new("curl"), template, cache_dir, progress, cancel)
}

fn download_with(
    program: &Path,
    template: &OnlineTemplate,
    cache_dir: &Path,
    progress: &AtomicU64,
    cancel: &AtomicBool,
) -> Result<PathBuf, String> {
    // Checked again here: nothing else stops a bad entry being written out.
    if !template.url.starts_with(DOWNLOAD_PREFIX)
        || !is_safe_file_part(&template.package_name)
        || !is_safe_file_part(&template.version)
    {
        return Err("That template's address can't be trusted".to_owned());
    }
    fs::create_dir_all(cache_dir)
        .map_err(|error| format!("Couldn't create {}: {error}", cache_dir.display()))?;
    let path = template.cached_path(cache_dir);
    let part = path.with_extension("tgz.part");

    let result = run_download(program, template, &part, progress, cancel).and_then(|()| {
        if read_manifest(&part).is_none() {
            return Err("The download isn't a Unity template".to_owned());
        }
        fs::rename(&part, &path).map_err(|error| format!("Couldn't save the template: {error}"))
    });
    if result.is_err() {
        let _ = fs::remove_file(&part);
    }
    result.map(|()| path)
}

fn run_download(
    program: &Path,
    template: &OnlineTemplate,
    part: &Path,
    progress: &AtomicU64,
    cancel: &AtomicBool,
) -> Result<(), String> {
    let mut child = curl(program)
        .args(["--max-filesize", MAX_DOWNLOAD])
        // Give up on a stalled connection rather than on a slow large file.
        .args(["--speed-limit", "1024", "--speed-time", "60"])
        .arg("--output")
        .arg(part)
        .arg(&template.url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| describe_spawn_error(&error))?;
    loop {
        if cancel.load(Ordering::Relaxed) {
            let _ = child.kill();
            let _ = child.wait();
            return Err("Cancelled".to_owned());
        }
        if let Ok(meta) = fs::metadata(part) {
            progress.store(meta.len(), Ordering::Relaxed);
        }
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return Ok(()),
            Ok(Some(_)) => {
                let mut stderr = Vec::new();
                if let Some(mut pipe) = child.stderr.take() {
                    let _ = pipe.read_to_end(&mut stderr);
                }
                return Err(describe_failure(&stderr));
            }
            Ok(None) => std::thread::sleep(POLL),
            Err(error) => return Err(format!("Couldn't follow the download: {error}")),
        }
    }
}

/// Delete downloaded templates, and nothing else that may be in `cache_dir`.
/// Returns how many were deleted.
pub fn clear_downloads(cache_dir: &Path) -> io::Result<usize> {
    let Ok(entries) = fs::read_dir(cache_dir) else {
        return Ok(0);
    };
    let mut removed = 0;
    for entry in entries.filter_map(Result::ok) {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.ends_with(".tgz") || name.ends_with(".tgz.part") {
            fs::remove_file(entry.path())?;
            removed += 1;
        }
    }
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::{
        OnlineTemplate, base_version, clear_downloads, download_with, fetch_with, parse_catalog,
    };
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicBool, AtomicU64};
    use tempfile::TempDir;

    fn entry(package: &str, versions: &serde_json::Value) -> serde_json::Value {
        serde_json::json!({"node": {
            "name": "Universal 3D",
            "packageName": package,
            "description": "For URP.",
            "type": "CORE",
            "renderPipeline": "URP",
            "versions": versions,
        }})
    }

    fn version(name: &str, url: &str) -> serde_json::Value {
        serde_json::json!({"name": name, "tarball": {"url": url}})
    }

    fn url(file: &str) -> String {
        format!("https://download.packages.unity.com/{file}")
    }

    fn catalog(entries: &[serde_json::Value]) -> String {
        serde_json::json!({"data": {"getTemplates": {"edges": entries}}}).to_string()
    }

    #[test]
    fn the_catalog_goes_by_major_and_minor() {
        assert_eq!(base_version("6000.2.8f1"), "6000.2");
        assert_eq!(base_version("2022.3.22f1"), "2022.3");
        assert_eq!(base_version("custom"), "custom");
    }

    #[test]
    fn reads_a_catalog_and_prefers_a_release_version() {
        let reply = catalog(&[entry(
            "com.unity.template.urp",
            &serde_json::json!([
                version("2.0.0-pre.1", &url("pre.tgz")),
                version("1.0.1", &url("release.tgz")),
                version("1.0.0", &url("old.tgz")),
            ]),
        )]);
        let found = parse_catalog(&reply).expect("a catalog");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "Universal 3D");
        assert_eq!(found[0].version, "1.0.1");
        assert_eq!(found[0].url, url("release.tgz"));
        assert_eq!(found[0].render_pipeline, "URP");
    }

    #[test]
    fn a_template_with_only_prereleases_uses_the_newest() {
        let reply = catalog(&[entry(
            "com.unity.template.urp",
            &serde_json::json!([version("2.0.0-pre.2", &url("a.tgz"))]),
        )]);
        assert_eq!(
            parse_catalog(&reply).expect("a catalog")[0].version,
            "2.0.0-pre.2"
        );
    }

    #[test]
    fn unsafe_entries_are_left_out() {
        let good = serde_json::json!([version("1.0.0", &url("a.tgz"))]);
        let reply = catalog(&[
            entry("../../escape", &good),
            entry("back\\slash", &good),
            entry(".hidden", &good),
            entry(
                "elsewhere",
                &serde_json::json!([version("1.0.0", "https://evil.example/a.tgz")]),
            ),
            entry(
                "plain-http",
                &serde_json::json!([version("1.0.0", "http://download.packages.unity.com/a")]),
            ),
            entry(
                "lookalike",
                &serde_json::json!([version(
                    "1.0.0",
                    "https://download.packages.unity.com.evil.example/a"
                )]),
            ),
            entry(
                "bad-version",
                &serde_json::json!([version("../../x", &url("a.tgz"))]),
            ),
            entry("no-versions", &serde_json::json!([])),
        ]);
        assert_eq!(parse_catalog(&reply).expect("a catalog"), Vec::new());
    }

    #[test]
    fn unexpected_replies_are_errors_not_panics() {
        for reply in ["", "<html>", "{}", r#"{"errors":[{"message":"no"}]}"#] {
            assert!(parse_catalog(reply).is_err(), "{reply}");
        }
    }

    fn template() -> OnlineTemplate {
        OnlineTemplate {
            name: "Universal 3D".to_owned(),
            package_name: "com.unity.template.urp".to_owned(),
            description: String::new(),
            kind: "CORE".to_owned(),
            render_pipeline: "URP".to_owned(),
            version: "1.0.0".to_owned(),
            url: url("urp.tgz"),
        }
    }

    /// A stand-in for curl, running `script` with the arguments it was given.
    #[cfg(unix)]
    fn fake_curl(dir: &Path, script: &str) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;

        let draft = dir.join("curl.tmp");
        fs::write(&draft, format!("#!/bin/sh\n{script}\n")).expect("write script");
        fs::set_permissions(&draft, fs::Permissions::from_mode(0o755)).expect("permissions");
        let path = dir.join("curl");
        fs::rename(&draft, &path).expect("move script");
        path
    }

    /// A script that copies `source` to the path after `--output`.
    #[cfg(unix)]
    fn copying_curl(dir: &Path, source: &Path) -> PathBuf {
        let script = format!(
            "while [ $# -gt 0 ]; do\n  if [ \"$1\" = --output ]; then cp '{}' \"$2\"; fi\n  shift\ndone",
            source.display()
        );
        fake_curl(dir, &script)
    }

    fn template_archive(path: &Path) {
        let gz = flate2::write::GzEncoder::new(
            fs::File::create(path).expect("create archive"),
            flate2::Compression::fast(),
        );
        let mut tar = tar::Builder::new(gz);
        let text = br#"{"displayName":"Universal 3D"}"#;
        let mut header = tar::Header::new_gnu();
        header.set_size(text.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        tar.append_data(&mut header, "package/package.json", &text[..])
            .expect("add file");
        tar.into_inner()
            .expect("finish tar")
            .finish()
            .expect("finish gzip");
    }

    fn run(program: &Path, template: &OnlineTemplate, cache: &Path) -> Result<PathBuf, String> {
        download_with(
            program,
            template,
            cache,
            &AtomicU64::new(0),
            &AtomicBool::new(false),
        )
    }

    #[cfg(unix)]
    #[test]
    fn a_good_download_lands_in_the_cache() {
        let tmp = TempDir::new().expect("temp dir");
        let source = tmp.path().join("source.tgz");
        template_archive(&source);
        let program = copying_curl(tmp.path(), &source);
        let cache = tmp.path().join("cache");

        let path = run(&program, &template(), &cache).expect("downloaded");
        assert_eq!(path, cache.join("com.unity.template.urp-1.0.0.tgz"));
        assert!(path.is_file());
        assert_eq!(fs::read_dir(&cache).expect("list").count(), 1, "no .part");
    }

    #[cfg(unix)]
    #[test]
    fn a_download_that_is_not_a_template_is_discarded() {
        let tmp = TempDir::new().expect("temp dir");
        let source = tmp.path().join("source.tgz");
        fs::write(&source, "<html>sign in</html>").expect("write file");
        let program = copying_curl(tmp.path(), &source);
        let cache = tmp.path().join("cache");

        let error = run(&program, &template(), &cache).expect_err("not a template");
        assert!(error.contains("isn't a Unity template"), "{error}");
        assert_eq!(fs::read_dir(&cache).expect("list").count(), 0);
    }

    #[cfg(unix)]
    #[test]
    fn a_failed_download_reports_curls_message() {
        let tmp = TempDir::new().expect("temp dir");
        let program = fake_curl(tmp.path(), "echo 'curl: (22) error: 404' >&2\nexit 22");
        let error = run(&program, &template(), &tmp.path().join("cache")).expect_err("a failure");
        assert_eq!(error, "(22) error: 404");
    }

    #[cfg(unix)]
    #[test]
    fn cancelling_stops_the_download_and_leaves_nothing() {
        let tmp = TempDir::new().expect("temp dir");
        let program = fake_curl(tmp.path(), "sleep 30");
        let cache = tmp.path().join("cache");
        let started = std::time::Instant::now();
        let error = download_with(
            &program,
            &template(),
            &cache,
            &AtomicU64::new(0),
            &AtomicBool::new(true),
        )
        .expect_err("cancelled");
        assert_eq!(error, "Cancelled");
        assert!(started.elapsed().as_secs() < 10);
        assert_eq!(fs::read_dir(&cache).expect("list").count(), 0);
    }

    #[test]
    fn untrusted_entries_are_refused_even_if_they_reach_download() {
        let tmp = TempDir::new().expect("temp dir");
        let cache = tmp.path().join("cache");
        let mut escaping = template();
        escaping.package_name = "../../escape".to_owned();
        let mut elsewhere = template();
        elsewhere.url = "https://evil.example/a.tgz".to_owned();
        for bad in [escaping, elsewhere] {
            assert!(run(Path::new("/nonexistent/curl"), &bad, &cache).is_err());
        }
        assert!(!cache.exists(), "nothing was started");
    }

    #[test]
    fn missing_curl_says_so() {
        let error = fetch_with(Path::new("/nonexistent/curl"), "6000.2.8f1").expect_err("no curl");
        assert!(error.contains("curl"), "{error}");
    }

    #[cfg(unix)]
    #[test]
    fn fetch_sends_the_editors_base_version_and_reads_the_reply() {
        let tmp = TempDir::new().expect("temp dir");
        let body = tmp.path().join("body.json");
        let reply = tmp.path().join("reply.json");
        fs::write(
            &reply,
            catalog(&[entry(
                "com.unity.template.urp",
                &serde_json::json!([version("1.0.0", &url("a.tgz"))]),
            )]),
        )
        .expect("write reply");
        let program = fake_curl(
            tmp.path(),
            &format!("cat > '{}'\ncat '{}'", body.display(), reply.display()),
        );

        let found = fetch_with(&program, "6000.2.8f1").expect("a catalog");
        assert_eq!(found.len(), 1);
        let sent: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&body).expect("the request")).expect("JSON");
        assert_eq!(
            sent["variables"]["supportedUnityEditorVersions"],
            serde_json::json!(["6000.2"])
        );
    }

    #[test]
    fn clearing_removes_only_templates() {
        let tmp = TempDir::new().expect("temp dir");
        fs::write(tmp.path().join("a.tgz"), "x").expect("write file");
        fs::write(tmp.path().join("b.tgz.part"), "x").expect("write file");
        fs::write(tmp.path().join("notes.txt"), "keep").expect("write file");
        assert_eq!(clear_downloads(tmp.path()).expect("cleared"), 2);
        assert!(tmp.path().join("notes.txt").exists());
        assert_eq!(fs::read_dir(tmp.path()).expect("list").count(), 1);
        assert_eq!(clear_downloads(&tmp.path().join("missing")).expect("ok"), 0);
    }
}
