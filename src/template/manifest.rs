//! `vinoa-template.toml` schema + loader. OWNER: engine-dev.
//!
//! The manifest is pure data (spec §7.8/§7.9): schema-versioned, flat, no
//! inheritance, `deny_unknown_fields` (an unknown key is rejected, which is
//! also part of the external-template trust boundary).
use crate::error::{Error, Result, EXIT_CONFIG};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// Schema version this CLI understands.
pub const SCHEMA_VERSION: u32 = 1;
/// CLI version, for the `min_cli_version` check.
pub const CLI_VERSION: &str = env!("CARGO_PKG_VERSION");
/// The only build-time variable (spec §7.3). The manifest may declare
/// extensions, but in v1 `version` is the whole set.
pub const BUILD_TIME_VARS: [&str; 1] = ["version"];

/// Capability tokens a template set can declare as implemented.
///
/// The eight `--features` values (spec §3.3) plus the non-flag toggles. `init`
/// hard-errors (exit 65) when a user asks for a capability the template set did
/// not declare — silently generating nothing is worse than failing.
pub const KNOWN_FEATURES: [&str; 12] = [
    "sqlite",
    "bstats",
    "update-check",
    "placeholderapi",
    "gui",
    "spotbugs",
    "coverage",
    "release-ci",
    "example",
    "permissions",
    "quality",
    "git",
];

/// Where the template *files* come from. Not part of the manifest file.
#[derive(Debug, Clone, Default)]
pub enum SourceRef {
    /// Embedded template set (release) / `templates/` on disk (debug).
    #[default]
    Embedded,
    /// A directory that is the root of the template tree.
    Dir(PathBuf),
}

/// The embedded built-in template tree (`templates/`).
///
/// In debug builds rust-embed reads the files from disk at run time, in
/// release builds they are baked into the binary — exactly the dev/CI
/// behaviour the spec asks for.
#[derive(rust_embed::Embed)]
#[folder = "templates/"]
struct BuiltinAssets;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TemplateManifest {
    pub schema: u32,
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub template_version: String,
    pub min_cli_version: String,
    #[serde(default)]
    pub name: String,
    /// Declared generate-time variable set (camelCase). Assertion A4 checks
    /// this against what the templates actually consume — bidirectionally.
    #[serde(default)]
    pub vars: Vec<String>,
    /// Switch name (`cap_*` / `has_*` / `is_*`) -> §7.5 condition expression.
    #[serde(default)]
    pub conditions: BTreeMap<String, String>,
    /// Entries. `[[files]]` is accepted as an alias for `[[entries]]`.
    #[serde(default, alias = "files")]
    pub entries: Vec<Entry>,
    #[serde(default)]
    pub assertions: Vec<Assertion>,
    /// What this template set actually implements (see [`KNOWN_FEATURES`]).
    #[serde(default)]
    pub features: Features,
    /// Resolved template file source (never serialized).
    #[serde(skip)]
    pub source: SourceRef,
}

/// `[features] implemented = [...]` — capabilities this template set really
/// generates. Absent/empty means "nothing optional implemented".
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Features {
    #[serde(default)]
    pub implemented: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    /// Target path inside the generated project. Rendered; only the four
    /// path-safe variables are allowed (spec §7.4).
    #[serde(alias = "target")]
    pub path: String,
    /// Source path inside the template tree. Literal (no `{{ }}`); for
    /// `foreach` entries the segment `_p_` is replaced by the platform id.
    /// Defaults to `path` when `path` has no placeholders.
    #[serde(default)]
    pub template: Option<String>,
    #[serde(default)]
    pub render: Render,
    /// §7.5 condition over switches / raw variables.
    #[serde(default)]
    pub when: Option<String>,
    /// v1 understands exactly one loop source: `"platforms"`.
    #[serde(default)]
    pub foreach: Option<String>,
    /// Extra loop-scope variable names (declarative, spec §7.8 `paths`).
    #[serde(default)]
    pub paths: Vec<String>,
    /// Octal permission, e.g. `"755"`.
    #[serde(default)]
    pub mode: Option<String>,
    /// Build-time variables expanded by Gradle `processResources` (§7.3).
    #[serde(default)]
    pub build_time_vars: Vec<String>,
    /// Force text/binary classification. `None` = infer.
    #[serde(default)]
    pub text: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Render {
    #[default]
    Template,
    Copy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Assertion {
    /// Target path (rendered, so it may use variables).
    #[serde(alias = "path")]
    pub target: String,
    /// Substrings that must appear in the rendered target. These strings are
    /// themselves rendered first (spec §7.8), so `{{pluginName}}` works.
    pub contains: Vec<String>,
    /// Optional §7.5 condition: the assertion only applies when it holds. Needed
    /// for targets that are themselves conditional (e.g. `release.yml` under
    /// `cap_release_ci`).
    #[serde(default)]
    pub when: Option<String>,
}

fn config_error(code: &'static str, msg: impl Into<String>) -> Error {
    Error::new(code, EXIT_CONFIG, msg)
}

/// Load the built-in template set (`templates/`, embedded).
pub fn load_builtin() -> Result<TemplateManifest> {
    load_from_str_with_source(
        include_str!("../../templates/vinoa-template.toml"),
        SourceRef::Embedded,
    )
}

/// Load a manifest from a file; its parent directory becomes the template root.
pub fn load_from(path: &Path) -> Result<TemplateManifest> {
    let raw = std::fs::read_to_string(path).map_err(|e| {
        config_error(
            "template.not_found",
            format!("模板清单不可读: {} ({e})", path.display()),
        )
    })?;
    let root = path.parent().unwrap_or_else(|| Path::new(".")).to_path_buf();
    load_from_str_with_source(&raw, SourceRef::Dir(root))
}

/// Parse a manifest that has no on-disk template root (uses the embedded tree).
pub fn load_from_str(toml: &str) -> Result<TemplateManifest> {
    load_from_str_with_source(toml, SourceRef::Embedded)
}

pub fn load_from_str_with_source(toml: &str, source: SourceRef) -> Result<TemplateManifest> {
    let parsed: TemplateManifest = toml::from_str(toml).map_err(|e| {
        let msg = e.to_string();
        // toml's "unknown field" is the unknown-key case (§7.8).
        if msg.contains("unknown field") {
            config_error("template.manifest_unknown_key", msg)
        } else {
            config_error("template.manifest_invalid", msg)
        }
    })?;
    let manifest = TemplateManifest { source, ..parsed };
    manifest.validate()?;
    Ok(manifest)
}

impl TemplateManifest {
    /// Capabilities this template set declares as implemented. `init` compares
    /// the user's request against this list and hard-errors on a mismatch
    /// (exit 65) instead of silently generating nothing.
    pub fn implemented_features(&self) -> &[String] {
        &self.features.implemented
    }

    /// Structural validation independent of `ProjectSpec`. Returns warnings.
    pub fn validate(&self) -> Result<Vec<String>> {
        let mut warnings = Vec::new();
        if self.schema != SCHEMA_VERSION {
            return Err(config_error(
                "template.manifest_invalid",
                format!(
                    "清单 schema={}，本 CLI 只认识 schema={SCHEMA_VERSION}",
                    self.schema
                ),
            ));
        }
        if self.min_cli_version.trim().is_empty() {
            return Err(config_error(
                "template.manifest_invalid",
                "清单缺少 min_cli_version",
            ));
        }
        if version_gt(&self.min_cli_version, CLI_VERSION) {
            return Err(config_error(
                "template.version_too_old",
                format!(
                    "模板集要求 CLI >= {}，当前 CLI {CLI_VERSION}",
                    self.min_cli_version
                ),
            )
            .with_hint("升级 vinoa 或使用与当前 CLI 匹配的模板集"));
        }
        if self.id.trim().is_empty() {
            warnings.push("清单未声明 id".to_string());
        }
        if self.template_version.trim().is_empty() {
            warnings.push("清单未声明 template_version".to_string());
        }
        if self.entries.is_empty() {
            return Err(config_error("template.manifest_invalid", "清单没有任何 [[entries]]"));
        }
        if self.vars.iter().any(|v| v.trim().is_empty()) {
            return Err(config_error("template.manifest_invalid", "vars 含空变量名"));
        }
        let mut seen_vars = BTreeSet::new();
        for v in &self.vars {
            if !seen_vars.insert(v.as_str()) {
                return Err(config_error(
                    "template.manifest_invalid",
                    format!("vars 重复声明变量 `{v}`"),
                ));
            }
        }
        let mut seen_features = BTreeSet::new();
        for f in &self.features.implemented {
            if !KNOWN_FEATURES.contains(&f.as_str()) {
                return Err(config_error(
                    "template.manifest_invalid",
                    format!(
                        "[features] implemented 里的 `{f}` 不是已知能力（可用: {}）",
                        KNOWN_FEATURES.join(", ")
                    ),
                ));
            }
            if !seen_features.insert(f.as_str()) {
                return Err(config_error(
                    "template.manifest_invalid",
                    format!("[features] implemented 重复声明 `{f}`"),
                ));
            }
        }
        for (name, expr) in &self.conditions {
            if !is_switch_name(name) {
                return Err(config_error(
                    "template.manifest_invalid",
                    format!("[conditions] 的键 `{name}` 必须以 cap_ / has_ / is_ 开头（§7.5）"),
                ));
            }
            if expr.trim().is_empty() {
                return Err(config_error(
                    "template.manifest_invalid",
                    format!("[conditions] `{name}` 的表达式为空"),
                ));
            }
        }
        let mut seen_targets = BTreeSet::new();
        for (i, entry) in self.entries.iter().enumerate() {
            let at = format!("[[entries]][{i}] (path={})", entry.path);
            if entry.path.trim().is_empty() {
                return Err(config_error("template.bad_target", format!("{at}: path 为空")));
            }
            if let Some(src) = &entry.template {
                if src.contains("{{") || src.contains("}}") || src.contains("{%") {
                    return Err(config_error(
                        "template.bad_target",
                        format!("{at}: template 源路径不能含 `{{{{ }}}}`（源路径是字面量；平台分叉用 `_p_`）"),
                    ));
                }
                validate_source_rel(src, &at)?;
            } else if entry.path.contains("{{") {
                return Err(config_error(
                    "template.bad_target",
                    format!(
                        "{at}: 目标路径含占位符时必须显式给出 `template = \"...\"`（源路径是字面量）"
                    ),
                ));
            }
            match entry.foreach.as_deref() {
                None => {}
                Some("platforms") => {
                    if entry.template.is_none() {
                        return Err(config_error(
                            "template.bad_target",
                            format!("{at}: foreach = \"platforms\" 的条目必须给出 `template`（用 `_p_` 选择平台源文件）"),
                        ));
                    }
                }
                Some(other) => {
                    return Err(config_error(
                        "template.manifest_invalid",
                        format!("{at}: 不支持 foreach = \"{other}\"，v1 只支持 \"platforms\""),
                    ));
                }
            }
            if entry.mode.is_some() && parse_mode(entry.mode.as_deref().unwrap()).is_none() {
                return Err(config_error(
                    "template.manifest_invalid",
                    format!("{at}: mode 必须是八进制权限，如 \"755\""),
                ));
            }
            if entry.render == Render::Copy && !entry.build_time_vars.is_empty() {
                return Err(config_error(
                    "template.manifest_invalid",
                    format!("{at}: copy 条目不能声明 build_time_vars"),
                ));
            }
            for var in &entry.build_time_vars {
                if !BUILD_TIME_VARS.contains(&var.as_str()) {
                    warnings.push(format!(
                        "{at}: build_time_vars 里的 `{var}` 不是本 CLI 已知的 build-time 变量"
                    ));
                }
            }
            if entry.template.is_none() && !seen_targets.insert(entry.path.clone()) {
                return Err(config_error(
                    "template.bad_target",
                    format!("{at}: 目标路径重复"),
                ));
            }
        }
        for (i, a) in self.assertions.iter().enumerate() {
            if a.target.trim().is_empty() {
                return Err(config_error(
                    "template.manifest_invalid",
                    format!("[[assertions]][{i}]: target 为空"),
                ));
            }
            if a.contains.is_empty() {
                return Err(config_error(
                    "template.manifest_invalid",
                    format!("[[assertions]][{i}] (target={}): contains 为空", a.target),
                ));
            }
        }
        Ok(warnings)
    }
}

fn validate_source_rel(src: &str, at: &str) -> Result<()> {
    if src.starts_with('/') || src.starts_with('\\') || src.contains(':') {
        return Err(config_error(
            "template.bad_target",
            format!("{at}: template 源路径必须是相对路径（{src}）"),
        ));
    }
    for seg in src.split('/') {
        if seg.is_empty() || seg == "." || seg == ".." {
            return Err(config_error(
                "template.bad_target",
                format!("{at}: template 源路径含非法路径段（{src}）"),
            ));
        }
    }
    Ok(())
}

/// `"755"` / `"0755"` -> `0o755`.
pub fn parse_mode(mode: &str) -> Option<u32> {
    let m = mode.trim();
    let m = m.strip_prefix("0o").unwrap_or(m);
    if m.is_empty() || m.len() > 4 || !m.chars().all(|c| ('0'..='7').contains(&c)) {
        return None;
    }
    u32::from_str_radix(m, 8).ok()
}

pub fn is_switch_name(name: &str) -> bool {
    let n = name.trim();
    n.starts_with("cap_") || n.starts_with("has_") || n.starts_with("is_")
}

/// Read a template file from the manifest's source tree.
pub fn read_source(source: &SourceRef, rel: &str) -> Result<Vec<u8>> {
    match source {
        SourceRef::Embedded => BuiltinAssets::get(rel)
            .map(|f| f.data.into_owned())
            .ok_or_else(|| {
                config_error(
                    "template.not_found",
                    format!("模板文件不存在（内置模板集）: {rel}"),
                )
            }),
        SourceRef::Dir(root) => {
            let joined = root.join(rel);
            // No symlink / `..` escape out of the template root (§7.9).
            if let (Ok(croot), Ok(cjoined)) = (root.canonicalize(), joined.canonicalize()) {
                if !cjoined.starts_with(&croot) {
                    return Err(config_error(
                        "template.bad_target",
                        format!("模板源路径逃出模板根: {rel}"),
                    ));
                }
            }
            std::fs::read(&joined).map_err(|e| {
                config_error(
                    "template.not_found",
                    format!("模板文件不可读: {} ({e})", joined.display()),
                )
            })
        }
    }
}

/// `a > b` for dotted numeric versions (`1.20.6`, `26.2`, `0.1.0`).
pub fn version_gt(a: &str, b: &str) -> bool {
    let pa = version_parts(a);
    let pb = version_parts(b);
    let n = pa.len().max(pb.len());
    for i in 0..n {
        let x = pa.get(i).copied().unwrap_or(0);
        let y = pb.get(i).copied().unwrap_or(0);
        if x != y {
            return x > y;
        }
    }
    false
}

fn version_parts(v: &str) -> Vec<u64> {
    v.split(['.', '-', '+'])
        .map_while(|s| s.parse::<u64>().ok())
        .collect()
}
