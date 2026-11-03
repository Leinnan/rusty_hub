//! Project templates shipped with an editor.

use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::Path;

/// Where a template package keeps its description.
const MANIFEST_PATH: &str = "package/package.json";
/// A template's `package.json` is small; anything larger isn't one.
const MANIFEST_LIMIT: u64 = 1024 * 1024;

#[derive(Debug, Serialize, Deserialize, Clone, Eq)]
pub struct ProjectTemplate {
    pub path: String,
    pub title: String,
    /// The name Unity shows, read from the package.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// What the template is for, read from the package.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl std::hash::Hash for ProjectTemplate {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.path.hash(state);
    }
}

impl PartialEq for ProjectTemplate {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

/// What a template package says about itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateManifest {
    /// The name Unity shows.
    pub display_name: Option<String>,
    /// What the template is for.
    pub description: Option<String>,
}

impl ProjectTemplate {
    /// The templates of the editor installed at `path`. Archives are slow to
    /// read, so a template already in `known` keeps its details: its file name
    /// includes its version, so the same path means the same content.
    #[must_use]
    pub fn find_templates(path: &str, known: &[ProjectTemplate]) -> Vec<ProjectTemplate> {
        let Ok(dir) = std::fs::read_dir(Path::new(path).join(crate::consts::TEMPLATES_DIR)) else {
            return Vec::new();
        };

        let mut templates: Vec<ProjectTemplate> = dir
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let name = entry.file_name();
                let title = name.to_str()?.strip_suffix(".tgz")?.to_owned();
                let path = entry.path().to_string_lossy().into_owned();
                if let Some(template) = known
                    .iter()
                    .find(|template| template.path == path && template.display_name.is_some())
                {
                    return Some(template.clone());
                }
                let manifest = read_manifest(&entry.path());
                Some(Self {
                    path,
                    title,
                    display_name: manifest.as_ref().and_then(|m| m.display_name.clone()),
                    description: manifest.and_then(|m| m.description),
                })
            })
            .collect();
        templates.sort_by_key(|template| template.label().to_lowercase());
        templates
    }

    /// The name to show: the package's own if it has one, because file names
    /// can mislead (`3d-cross-platform` is really "3D URP").
    #[must_use]
    pub fn label(&self) -> &str {
        self.display_name
            .as_deref()
            .unwrap_or_else(|| self.title.trim_start_matches("com.unity.template."))
    }
}

/// Read a template archive's `package.json`, without unpacking anything to
/// disk. `None` if the file isn't a readable template package.
#[must_use]
pub fn read_manifest(archive: &Path) -> Option<TemplateManifest> {
    let file = std::fs::File::open(archive).ok()?;
    let mut archive = tar::Archive::new(GzDecoder::new(file));
    let entry = archive
        .entries()
        .ok()?
        .filter_map(Result::ok)
        .find(|entry| {
            entry
                .path()
                .is_ok_and(|path| path == Path::new(MANIFEST_PATH))
        })?;

    let mut text = String::new();
    entry.take(MANIFEST_LIMIT).read_to_string(&mut text).ok()?;
    let json: serde_json::Value = serde_json::from_str(&text).ok()?;
    let field = |name: &str| json.get(name)?.as_str().map(str::to_owned);
    Some(TemplateManifest {
        display_name: field("displayName"),
        description: field("description"),
    })
}
