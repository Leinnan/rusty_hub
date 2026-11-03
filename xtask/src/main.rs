//! Project tasks that need more than a recipe: `cargo xtask <task>`.
//!
//! Everything goes to `target/package`, which recipes pack into bundles. None
//! of it is committed: `assets/icon/icon.svg` and `Cargo.toml` are the sources.

use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use app_icon::Tree;
use icns::{IconFamily, IconType, PixelFormat};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// The package whose dependencies the licence list covers.
const APP_PACKAGE: &str = "rusty_hub";
/// Sizes written to `icon/png/{size}.png`, for Linux.
const PNG_SIZES: [u32; 7] = [16, 32, 48, 64, 128, 256, 512];
/// Every slot of a modern `.icns`, `@2x` variants included.
const ICNS_SLOTS: [(IconType, u32); 10] = [
    (IconType::RGBA32_16x16, 16),
    (IconType::RGBA32_16x16_2x, 32),
    (IconType::RGBA32_32x32, 32),
    (IconType::RGBA32_32x32_2x, 64),
    (IconType::RGBA32_128x128, 128),
    (IconType::RGBA32_128x128_2x, 256),
    (IconType::RGBA32_256x256, 256),
    (IconType::RGBA32_256x256_2x, 512),
    (IconType::RGBA32_512x512, 512),
    (IconType::RGBA32_512x512_2x, 1024),
];

fn main() -> ExitCode {
    let task = std::env::args().nth(1);
    let result = match task.as_deref() {
        Some("icons") => icons(),
        Some("licenses") => licenses(),
        _ => {
            eprintln!("usage: cargo xtask <icons|licenses>");
            return ExitCode::FAILURE;
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("xtask: {error}");
            ExitCode::FAILURE
        }
    }
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask lives inside the workspace")
        .to_path_buf()
}

fn package_dir() -> PathBuf {
    workspace_root().join("target").join("package")
}

/// Render the icon for every bundle into `target/package/icon`.
fn icons() -> Result<()> {
    let tree = app_icon::load(&workspace_root().join("assets/icon/icon.svg"))?;
    let dir = package_dir().join("icon");
    write_icons(&tree, &dir)?;
    println!("wrote {}", dir.display());
    Ok(())
}

fn write_icons(tree: &Tree, dir: &Path) -> Result<()> {
    let png_dir = dir.join("png");
    fs::create_dir_all(&png_dir)?;
    for size in PNG_SIZES {
        fs::write(
            png_dir.join(format!("{size}.png")),
            app_icon::render_png(tree, size)?,
        )?;
    }

    let mut family = IconFamily::new();
    for (slot, size) in ICNS_SLOTS {
        let pixels = app_icon::render_rgba(tree, size)?;
        let image = icns::Image::from_data(PixelFormat::RGBA, size, size, pixels)?;
        family.add_icon_with_type(&image, slot)?;
    }
    family.write(BufWriter::new(fs::File::create(
        dir.join("rusty-hub.icns"),
    )?))?;

    fs::write(dir.join("rusty-hub.ico"), app_icon::render_ico(tree)?)?;
    Ok(())
}

/// List the app's third-party crates in `target/package/licenses.txt`.
fn licenses() -> Result<()> {
    let root = workspace_root();
    let output = std::process::Command::new(env!("CARGO"))
        .args(["metadata", "--format-version", "1"])
        .current_dir(&root)
        .output()?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned().into());
    }
    let metadata: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let lines = license_lines(&metadata)?;

    let dir = package_dir();
    fs::create_dir_all(&dir)?;
    let path = dir.join("licenses.txt");
    let text = format!(
        "Rusty Hub is built on these open-source crates.\n\n{}\n",
        lines.join("\n")
    );
    fs::write(&path, text)?;
    println!("wrote {} ({} crates)", path.display(), lines.len());
    Ok(())
}

/// "name version: license" for each third-party crate the app reaches through
/// normal dependencies, on any platform. Build and dev dependencies don't ship.
fn license_lines(metadata: &serde_json::Value) -> Result<Vec<String>> {
    let text =
        |value: &serde_json::Value, key: &str| value[key].as_str().unwrap_or_default().to_owned();
    let packages: HashMap<String, &serde_json::Value> = metadata["packages"]
        .as_array()
        .ok_or("metadata: no packages")?
        .iter()
        .map(|package| (text(package, "id"), package))
        .collect();
    let members: BTreeSet<String> = metadata["workspace_members"]
        .as_array()
        .ok_or("metadata: no workspace members")?
        .iter()
        .filter_map(|id| id.as_str().map(str::to_owned))
        .collect();
    let nodes: HashMap<String, &serde_json::Value> = metadata["resolve"]["nodes"]
        .as_array()
        .ok_or("metadata: no resolve graph")?
        .iter()
        .map(|node| (text(node, "id"), node))
        .collect();
    let app = members
        .iter()
        .find(|id| packages.get(*id).is_some_and(|p| p["name"] == APP_PACKAGE))
        .ok_or("metadata: the app's package wasn't found")?
        .clone();

    let mut seen = BTreeSet::new();
    let mut stack = vec![app];
    while let Some(id) = stack.pop() {
        if !seen.insert(id.clone()) {
            continue;
        }
        let Some(node) = nodes.get(&id) else { continue };
        for dep in node["deps"].as_array().into_iter().flatten() {
            let normal = dep["dep_kinds"]
                .as_array()
                .into_iter()
                .flatten()
                .any(|kind| kind["kind"].is_null());
            if normal {
                stack.push(text(dep, "pkg"));
            }
        }
    }

    let mut lines: Vec<String> = seen
        .iter()
        .filter(|id| !members.contains(*id))
        .filter_map(|id| packages.get(id))
        .map(|package| {
            let license = package["license"].as_str().unwrap_or("see crate");
            format!(
                "{} {}: {license}",
                text(package, "name"),
                text(package, "version")
            )
        })
        .collect();
    lines.sort();
    lines.dedup();
    Ok(lines)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::BufReader;
    use std::path::Path;

    use icns::IconFamily;

    use super::{ICNS_SLOTS, PNG_SIZES, license_lines, write_icons};

    #[test]
    fn writes_every_icon_a_bundle_needs() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let svg = Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets/icon/icon.svg");
        let tree = app_icon::load(&svg).expect("the icon source parses");
        write_icons(&tree, tmp.path()).expect("written");

        for size in PNG_SIZES {
            assert!(tmp.path().join(format!("png/{size}.png")).is_file());
        }
        assert!(tmp.path().join("rusty-hub.ico").is_file());
        let icns = fs::File::open(tmp.path().join("rusty-hub.icns")).expect("an icns file");
        let family = IconFamily::read(BufReader::new(icns)).expect("a valid icns file");
        assert_eq!(family.available_icons().len(), ICNS_SLOTS.len());
    }

    #[test]
    fn only_third_party_crates_the_app_ships_are_listed() {
        let metadata = serde_json::json!({
            "packages": [
                {"id": "app", "name": "rusty_hub", "version": "1.0.0", "license": "MIT"},
                {"id": "own", "name": "unity-hub", "version": "1.0.0", "license": "MIT"},
                {"id": "direct", "name": "direct", "version": "1.2.0", "license": "Apache-2.0"},
                {"id": "nested", "name": "nested", "version": "0.1.0", "license": null},
                {"id": "dev", "name": "dev-only", "version": "1.0.0", "license": "MIT"},
                {"id": "build", "name": "build-only", "version": "1.0.0", "license": "MIT"},
            ],
            "workspace_members": ["app", "own"],
            "resolve": {"nodes": [
                {"id": "app", "deps": [
                    {"pkg": "own", "dep_kinds": [{"kind": null}]},
                    {"pkg": "direct", "dep_kinds": [{"kind": null}]},
                    {"pkg": "dev", "dep_kinds": [{"kind": "dev"}]},
                    {"pkg": "build", "dep_kinds": [{"kind": "build"}]},
                ]},
                {"id": "own", "deps": [
                    {"pkg": "nested", "dep_kinds": [{"kind": null}]},
                ]},
                {"id": "direct", "deps": []},
                {"id": "nested", "deps": []},
                {"id": "dev", "deps": []},
                {"id": "build", "deps": []},
            ]},
        });
        assert_eq!(
            license_lines(&metadata).expect("listed"),
            ["direct 1.2.0: Apache-2.0", "nested 0.1.0: see crate"]
        );
    }

    #[test]
    fn unexpected_metadata_is_an_error() {
        assert!(license_lines(&serde_json::json!({})).is_err());
    }
}
