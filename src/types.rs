//! Shared vocabulary. CONTRACT FILE — changes here must be announced to all teammates.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

pub const PLATFORMS: [&str; 7] = [
    "paper", "bukkit", "velocity", "bungeecord", "folia", "sponge", "minestom",
];
/// Selecting `paper` implies `bukkit` (spec §8).
pub const PAPER_IMPLIES: &str = "bukkit";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MetadataFormat {
    PluginYml,
    PaperPluginYml,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ArtifactLanguage {
    Zh,
    En,
    Both,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UiLanguage {
    Zh,
    En,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureSet {
    pub sqlite: bool,
    pub bstats: bool,
    pub update_check: bool,
    pub placeholderapi: bool,
    pub gui: bool,
    pub spotbugs: bool,
    pub coverage: bool,
    pub release_ci: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QualitySet {
    pub checkstyle: bool,
    pub unit_tests: bool,
    pub ci: bool,
}

impl Default for QualitySet {
    fn default() -> Self {
        Self { checkstyle: true, unit_tests: true, ci: true }
    }
}

/// Fully resolved answers: what the wizard collected or the flags supplied.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSpec {
    pub target_dir: PathBuf,
    pub project_name: String,
    pub plugin_name: String,
    pub package_name: String,
    pub author: Vec<String>,
    pub description: Option<String>,
    pub mc_version: String,
    pub platforms: Vec<String>,
    pub metadata: MetadataFormat,
    pub language: ArtifactLanguage,
    pub ui_language: UiLanguage,
    pub license: String,
    pub features: FeatureSet,
    pub quality: QualitySet,
    /// `--no-example`: generate the sample command/listener/service at all.
    pub example: bool,
    /// `--no-permissions`: emit permission declarations + constants class.
    pub permissions: bool,
    /// Reserved for a future `--website`; templates must handle `None`.
    pub website: Option<String>,
    pub git: bool,
    pub download_jdk: bool,
    pub bstats_id: Option<String>,
}

/// Per-platform capabilities derived from the matrix (spec §6/§7).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capabilities {
    pub plugin_yml_api_version: Option<String>,
    pub libraries_enabled: bool,
    pub run_task: bool,
    pub paperweight: bool,
    pub reobf: bool,
    pub legacy_namespace: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatformPlan {
    pub id: String,
    pub gradle_path: String,
    pub api_coordinate: String,
    pub java_target: u8,
    pub metadata: MetadataFormat,
    pub capability: Capabilities,
    pub experimental: bool,
}

/// Matrix lookup result — everything the renderer needs about versions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resolved {
    pub mc_version: String,
    pub gradle_version: String,
    pub core_java_target: u8,
    pub platforms: BTreeMap<String, PlatformPlan>,
    pub thirdparty: BTreeMap<String, String>,
    /// checkstyle / checkstyle_legacy / junit / junit_legacy / spotbugs / spotbugs_legacy
    #[serde(default)]
    pub quality: BTreeMap<String, String>,
    /// shadow / run_paper / run_velocity / paperweight (version strings)
    #[serde(default)]
    pub gradle_plugins: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMeta {
    pub path: String,
    pub bytes: usize,
    pub sha256: String,
}

#[derive(Debug, Clone)]
pub struct PlannedFile {
    pub path: String,
    pub content: Vec<u8>,
    pub executable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlannedAction {
    GitInit { branch: String, message: String },
    Verify,
}

/// The plan is a first-class value: `--dry-run` prints it, execution only executes it.
#[derive(Debug, Clone)]
pub struct Plan {
    pub root: PathBuf,
    pub files: Vec<PlannedFile>,
    pub actions: Vec<PlannedAction>,
    /// Conditional files that were intentionally not generated (informational).
    pub skipped: Vec<String>,
    /// Things the user should act on — not routine conditional skips.
    pub warnings: Vec<String>,
}

/// `paper` implies `bukkit`; de-duplicate and keep the canonical platform order.
pub fn normalize_platforms(requested: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for p in requested {
        let p = p.trim().to_ascii_lowercase();
        if p.is_empty() {
            continue;
        }
        if !out.contains(&p) {
            out.push(p.clone());
        }
        if p == "paper" && !out.contains(&PAPER_IMPLIES.to_string()) {
            out.push(PAPER_IMPLIES.to_string());
        }
    }
    out.sort_by_key(|p| PLATFORMS.iter().position(|x| x == p).unwrap_or(usize::MAX));
    out
}

/// `My Plugin!` -> `my-plugin`
pub fn slug(name: &str) -> String {
    let mut out = String::new();
    let mut pending_dash = false;
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            if pending_dash && !out.is_empty() {
                out.push('-');
            }
            pending_dash = false;
            out.push(ch.to_ascii_lowercase());
        } else {
            pending_dash = true;
        }
    }
    out
}

/// `my-plugin` -> `MyPlugin`
pub fn to_pascal(name: &str) -> String {
    name.split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(|s| {
            let mut chars = s.chars();
            match chars.next() {
                Some(first) => {
                    first.to_ascii_uppercase().to_string() + &chars.as_str().to_ascii_lowercase()
                }
                None => String::new(),
            }
        })
        .collect()
}

impl Plan {
    pub fn metas(&self) -> Vec<FileMeta> {
        self.files
            .iter()
            .map(|f| FileMeta {
                path: f.path.clone(),
                bytes: f.content.len(),
                sha256: crate::template::sha256_hex(&f.content),
            })
            .collect()
    }
}
