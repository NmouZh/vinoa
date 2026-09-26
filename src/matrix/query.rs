//! Matrix type + queries (spec §6). OWNER: matrix-dev.
//!
//! Data model: **per-platform whitelists only** — every `(mc_version, platform)`
//! pair that exists is written out explicitly in `data/version-matrix.toml`.
//! Nothing here derives availability from a version range, and no coordinate is
//! ever assembled by string interpolation: `1.8.9 → spigot-api:1.8.8-R0.1-SNAPSHOT`
//! is the proof that interpolation produces a 404 (§6.3.3, LEG §1.1).
use std::cmp::Ordering;
use std::collections::BTreeMap;

use serde::Deserialize;

use super::builtin::BUILTIN_MATRIX_TOML;
use crate::error::{self, Error, Result};
use crate::types::{Capabilities, MetadataFormat, PlatformPlan, Resolved, PAPER_IMPLIES, PLATFORMS};

/// Fixed platform priority order (§7.1 "去重后按固定优先级排序"; §11.5 shows
/// `"platforms": ["paper","bukkit"]`). Identical to [`crate::types::PLATFORMS`].
pub const PLATFORM_ORDER: [&str; 7] = PLATFORMS;

/// Proxies never participate in MC-version filtering (§6.3.5).
pub const PROXY_PLATFORMS: [&str; 2] = ["velocity", "bungeecord"];

/// Recognised `schema` value.
pub const SCHEMA: u32 = 1;

/// `plugin.yml`'s `api-version` exists from the 1.13 API ([LEG §5.1]); the most
/// permissive legal value is written for every version that supports the key.
const API_VERSION_FLOOR: &str = "1.13";
const API_VERSION_VALUE: &str = "1.13";
/// `libraries:` arrived with the 1.16.5 API (§7.5 `cap_libraries`, [LEG §5.1]).
const LIBRARIES_FLOOR: &str = "1.16.5";
/// `paperweight-userdev`'s oldest dev bundle is 1.17.1 ([LEG §4.2]).
pub const PAPERWEIGHT_FLOOR: &str = "1.17.1";
/// From 26.1 Paper no longer supports reobfuscated plugins (§7.1, [VM §5.4]).
pub const REOBF_UNTIL_EXCLUSIVE: &str = "26.1";

// ---------------------------------------------------------------- data model

fn default_true() -> bool {
    true
}

/// `[java]` row: the toolchain fact (`min`) plus the advisor-only recommended value.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JavaRequirement {
    pub min: u8,
    pub recommended: u8,
    /// `false` = outside the support window (data-driven, never hard-coded).
    #[serde(default = "default_true")]
    pub selectable: bool,
}

/// A coordinate that also carries its own Java floor (`bungeecord`, `sponge`).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VersionedCoordinate {
    pub api: String,
    pub java: u8,
}

/// Which module consumes a third-party library (drives the Java-floor check).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ThirdPartyScope {
    #[default]
    Core,
    Platform,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThirdPartyLib {
    pub coordinate: String,
    /// Class-file major version was read from the published jar (major 52 = Java 8).
    pub java_min: u8,
    #[serde(default)]
    pub scope: ThirdPartyScope,
    /// Repository ids from `[repositories]` that serve this coordinate
    /// (`me.clip:placeholderapi` is only on `extendedclip`).
    #[serde(default)]
    pub repos: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GradlePlugin {
    pub version: String,
    /// Minimum Gradle version, from the plugin's `.module`
    /// `org.gradle.plugin.api-version`. `None` = the plugin declares no floor
    /// (e.g. `com.github.spotbugs`, built by Gradle 8.14.5 without one).
    #[serde(default)]
    pub gradle_min: Option<String>,
    /// JVM floor from `.module` `org.gradle.jvm.version`.
    pub java: u8,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GradleData {
    pub version: String,
    pub run_on_jvm: String,
    pub java25_toolchain_min: String,
    pub shadow: GradlePlugin,
    pub run_paper: GradlePlugin,
    pub run_velocity: GradlePlugin,
    pub paperweight: GradlePlugin,
    /// `com.github.spotbugs` Gradle **plugin** (distinct from `[quality].spotbugs`,
    /// which is the analysis tool version).
    pub spotbugs_plugin: GradlePlugin,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct PlatformData {
    pub api: String,
    /// Repository id from `[repositories]` serving this platform's coordinates.
    #[serde(default)]
    pub repo: Option<String>,

    /// Explicit whitelist of MC versions this platform publishes for (§6.1).
    #[serde(default)]
    pub versions: Vec<String>,
    /// MC version → exact coordinate. Literal strings, never interpolated.
    #[serde(default)]
    pub coordinate_map: BTreeMap<String, String>,
    /// Known-good build pins; take precedence over `coordinate_map`.
    #[serde(default)]
    pub pinned: BTreeMap<String, String>,
    /// Version → coordinate for proxy-style platforms (`bungeecord`, `sponge`).
    #[serde(default)]
    pub coordinates: BTreeMap<String, VersionedCoordinate>,
    /// MC version → SpongeAPI line (spec §6.1 `api_versions`).
    #[serde(default)]
    pub api_versions: BTreeMap<String, String>,
    #[serde(default)]
    pub legacy_api: Option<String>,
    #[serde(default)]
    pub legacy_namespace_until: Option<String>,
    #[serde(default)]
    pub legacy_version_until: Option<String>,
    #[serde(default)]
    pub min_version: Option<String>,
    /// Fixed Java target (proxies / minestom).
    #[serde(default)]
    pub java: Option<u8>,
    /// Java target used up to (and including) `java_legacy_until` (`bukkit`, §6.3.4).
    #[serde(default)]
    pub java_legacy: Option<u8>,
    #[serde(default)]
    pub java_legacy_until: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub protocol_min: Option<String>,
    #[serde(default)]
    pub protocol_max: BTreeMap<String, String>,
    #[serde(default)]
    pub default_api: Option<String>,
    #[serde(default)]
    pub default: Option<String>,
    /// B-level platform (§12.1: `folia` / `sponge` / `minestom`).
    #[serde(default)]
    pub experimental: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatrixData {
    pub schema: u32,
    pub generated_at: String,
    /// Repository id → URL. Data-only for now: `Resolved` deliberately does not
    /// expose it (the generated build keeps its repository literals in templates).
    #[serde(default)]
    pub repositories: BTreeMap<String, String>,
    pub java: BTreeMap<String, JavaRequirement>,
    pub quality: BTreeMap<String, String>,
    pub gradle: GradleData,
    pub thirdparty: BTreeMap<String, ThirdPartyLib>,
    pub platform: BTreeMap<String, PlatformData>,
}

// ------------------------------------------------------------ version keys

/// MC versions are compared **numerically**, never lexically (`1.9 < 1.10 < 1.21.11 < 26.1`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct McKey(u32, u32, u32);

/// Parse `X.Y[.Z]` into a sortable key. `None` for anything else.
pub fn mc_key(v: &str) -> Option<McKey> {
    let mut it = v.split('.');
    let major = it.next()?.parse().ok()?;
    let minor = it.next().unwrap_or("0").parse().ok()?;
    let patch = it.next().unwrap_or("0").parse().ok()?;
    if it.next().is_some() {
        return None;
    }
    Some(McKey(major, minor, patch))
}

/// Numeric key of a Maven artifact version (`5.2.0-SNAPSHOT` → `5.2.0`).
fn artifact_key(v: &str) -> Option<McKey> {
    mc_key(v.split('-').next().unwrap_or(v))
}

/// Total-order sort key: `major.minor.patch` of the release part, then the
/// pre-release suffix, then the raw string as a tie-break (so the ordering is a
/// pure function of the input and `sort_by` can never see an inconsistent pair —
/// Fill returns ids like `1.21.11-rc3` alongside `1.21.11`).
fn sort_key(v: &str) -> (u32, u32, u32, String) {
    match v.split_once('-') {
        Some((core, suffix)) => match mc_key(core) {
            Some(McKey(a, b, c)) => (a, b, c, suffix.to_string()),
            None => (u32::MAX, u32::MAX, u32::MAX, v.to_string()),
        },
        None => match mc_key(v) {
            Some(McKey(a, b, c)) => (a, b, c, String::new()),
            None => (u32::MAX, u32::MAX, u32::MAX, v.to_string()),
        },
    }
}

/// Ordering for two MC version strings (`1.9 < 1.10 < 1.21.11 < 26.1`; a release
/// sorts before its own `-rc`/`-pre` ids). Always a total order.
pub fn cmp_mc(a: &str, b: &str) -> Ordering {
    sort_key(a).cmp(&sort_key(b))
}

/// Sort helper: ascending MC order (`1.8.9, 1.9, …, 1.21.11, 26.1, …, 26.3`).
pub fn sort_versions(v: &mut [String]) {
    v.sort_by(|a, b| cmp_mc(a, b));
}

fn is_proxy(platform: &str) -> bool {
    PROXY_PLATFORMS.contains(&platform)
}

// -------------------------------------------------------------------- Matrix

/// Parsed, validated version matrix.
#[derive(Debug, Clone)]
pub struct Matrix {
    data: MatrixData,
}

impl Matrix {
    /// The matrix compiled into the binary (`data/version-matrix.toml`).
    pub fn builtin() -> Result<Self> {
        Self::load_from_str(BUILTIN_MATRIX_TOML)
    }

    /// Parse + validate a matrix document. A malformed built-in matrix is a
    /// config error (§11.2 exit 78).
    pub fn load_from_str(toml_src: &str) -> Result<Self> {
        let data: MatrixData = toml::from_str(toml_src)
            .map_err(|e| error::config(format!("版本矩阵 TOML 解析失败: {e}")))?;
        validate(&data).map_err(|e| error::config(format!("版本矩阵校验失败: {e}")))?;
        Ok(Self { data })
    }

    /// `generated_at` of the data file (`--json` reports the matrix source).
    pub fn generated_at(&self) -> &str {
        &self.data.generated_at
    }

    pub fn schema(&self) -> u32 {
        self.data.schema
    }

    /// Gradle wrapper version (spec §6.1 `[gradle].version`, §8.3 `libs.versions.toml`).
    pub fn gradle_version(&self) -> &str {
        &self.data.gradle.version
    }

    pub fn gradle(&self) -> &GradleData {
        &self.data.gradle
    }

    /// Quality-tool versions, canonical keys (§7.1 `qualityToolVersions`).
    pub fn quality(&self) -> &BTreeMap<String, String> {
        &self.data.quality
    }

    /// Gradle plugin versions only (canonical keys: `shadow` / `run_paper` /
    /// `run_velocity` / `paperweight`); floors stay in the data file.
    pub fn gradle_plugins(&self) -> BTreeMap<String, String> {
        let g = &self.data.gradle;
        [
            ("shadow", &g.shadow),
            ("run_paper", &g.run_paper),
            ("run_velocity", &g.run_velocity),
            ("paperweight", &g.paperweight),
            ("spotbugs_plugin", &g.spotbugs_plugin),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.version.clone()))
        .collect()
    }

    pub fn gradle_plugin(&self, key: &str) -> Option<&GradlePlugin> {
        match key {
            "shadow" => Some(&self.data.gradle.shadow),
            "run_paper" => Some(&self.data.gradle.run_paper),
            "run_velocity" => Some(&self.data.gradle.run_velocity),
            "paperweight" => Some(&self.data.gradle.paperweight),
            "spotbugs_plugin" => Some(&self.data.gradle.spotbugs_plugin),
            _ => None,
        }
    }

    /// Third-party libraries keyed canonically (`placeholderapi`, `bstats`, `sqlite_jdbc`).
    pub fn thirdparty(&self) -> &BTreeMap<String, ThirdPartyLib> {
        &self.data.thirdparty
    }

    /// Repository id → URL (`central`, `papermc`, `extendedclip`, `spongepowered`).
    pub fn repositories(&self) -> &BTreeMap<String, String> {
        &self.data.repositories
    }

    /// Repository URL serving this platform's coordinates.
    pub fn platform_repo(&self, platform: &str) -> Option<&str> {
        let id = self.data.platform.get(platform)?.repo.as_deref()?;
        self.data.repositories.get(id).map(String::as_str)
    }

    /// Repository URLs that serve a third-party coordinate.
    pub fn thirdparty_repos(&self, key: &str) -> Vec<&str> {
        self.data
            .thirdparty
            .get(key)
            .map(|lib| {
                lib.repos
                    .iter()
                    .filter_map(|id| self.data.repositories.get(id).map(String::as_str))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// `java_min` / `java_recommended` for an MC version (`recommended` is advisor-only, §6.3.2).
    pub fn java_requirement(&self, mc: &str) -> Option<(u8, u8)> {
        self.data.java.get(mc).map(|j| (j.min, j.recommended))
    }

    pub fn is_selectable(&self, mc: &str) -> bool {
        self.data.java.get(mc).is_some_and(|j| j.selectable)
    }

    /// Every MC version present in `[java]`, ascending (`versions --matrix`).
    pub fn all_java_versions(&self) -> Vec<String> {
        let mut out: Vec<String> = self.data.java.keys().cloned().collect();
        sort_versions(&mut out);
        out
    }

    /// Platform ids present in the data file, ascending.
    pub fn platform_ids(&self) -> Vec<String> {
        self.data.platform.keys().cloned().collect()
    }

    /// Normalise a user-supplied platform list: validate names, expand
    /// `paper → bukkit`, dedupe, then sort by the fixed priority order (§7.1).
    pub fn normalize_platforms(&self, requested: &[String]) -> Result<Vec<String>> {
        if requested.is_empty() {
            return Err(error::usage(
                "至少需要一个目标平台（--platform paper|bukkit|velocity|bungeecord|folia|sponge|minestom）",
            ));
        }
        let mut set: Vec<String> = Vec::new();
        for raw in requested {
            let name = raw.trim();
            if !PLATFORM_ORDER.contains(&name) {
                return Err(error::usage(format!(
                    "未知平台: {name}（可用: {}）",
                    PLATFORM_ORDER.join(", ")
                )));
            }
            if !set.iter().any(|s| s == name) {
                set.push(name.to_string());
            }
            if name == "paper" && !set.iter().any(|s| s == PAPER_IMPLIES) {
                set.push(PAPER_IMPLIES.to_string());
            }
        }
        set.sort_by_key(|p| PLATFORM_ORDER.iter().position(|x| x == p).unwrap_or(usize::MAX));
        Ok(set)
    }

    /// MC versions that are selectable **and** have at least one server-side
    /// platform (§1 support window; proxies alone do not make a version selectable).
    pub fn supported_versions(&self) -> Vec<String> {
        let mut out: Vec<String> = self
            .data
            .java
            .iter()
            .filter(|(mc, j)| j.selectable && !self.gated_platforms(mc).is_empty())
            .map(|(mc, _)| mc.clone())
            .collect();
        sort_versions(&mut out);
        out
    }

    /// Server-side platforms available for this MC version (proxies excluded).
    /// This is the list shown by the "该版本可用平台" half of the error (§11.3).
    pub fn gated_platforms(&self, mc: &str) -> Vec<String> {
        PLATFORM_ORDER
            .iter()
            .filter(|p| !is_proxy(p))
            .filter(|p| self.platform_plan(mc, p).is_some())
            .map(|p| (*p).to_string())
            .collect()
    }

    /// Everything the wizard may offer for this MC version: the whitelisted
    /// server platforms plus both proxies, which never filter on MC version
    /// (§6.3.5), in fixed priority order.
    pub fn available_platforms(&self, mc: &str) -> Vec<String> {
        PLATFORM_ORDER
            .iter()
            .filter(|p| self.platform_plan(mc, p).is_some())
            .map(|p| (*p).to_string())
            .collect()
    }

    /// The whitelist a platform publishes for, in ascending MC order.
    pub fn platform_versions(&self, platform: &str) -> Vec<String> {
        let Some(p) = self.data.platform.get(platform) else {
            return Vec::new();
        };
        let mut out: Vec<String> = match p.model.as_deref() {
            Some("protocol") => Vec::new(),
            Some("major-grain") => p.coordinates.keys().cloned().collect(),
            _ => p.versions.clone(),
        };
        sort_versions(&mut out);
        out
    }

    /// Hit/miss lookup for one `(mc, platform)` pair (§6.3.1). `None` = miss.
    pub fn platform_plan(&self, mc: &str, platform: &str) -> Option<PlatformPlan> {
        let jr = self.data.java.get(mc)?;
        if !jr.selectable {
            return None;
        }
        let p = self.data.platform.get(platform)?;
        if !PLATFORM_ORDER.contains(&platform) {
            return None;
        }
        let (api_coordinate, java_target) = match p.model.as_deref() {
            // Proxy: protocol range, no MC-version filtering (§6.3.5).
            Some("protocol") => (p.default_api.clone()?, p.java?),
            Some("major-grain") => {
                let chosen = p.default.as_ref()?;
                let c = p.coordinates.get(chosen)?;
                (c.api.clone(), c.java)
            }
            _ => {
                if !p.versions.iter().any(|v| v == mc) {
                    return None;
                }
                match platform {
                    "bukkit" => {
                        let java = match (&p.java_legacy, &p.java_legacy_until) {
                            (Some(legacy), Some(until))
                                if cmp_mc(mc, until) != Ordering::Greater =>
                            {
                                *legacy
                            }
                            _ => jr.min,
                        };
                        (p.coordinate_map.get(mc)?.clone(), java)
                    }
                    "sponge" => {
                        let c = p.coordinates.get(mc)?;
                        (c.api.clone(), c.java)
                    }
                    "minestom" => (p.coordinate_map.get(mc)?.clone(), p.java?),
                    // paper / folia: pinned wins, then the literal coordinate map.
                    _ => {
                        let coord = p
                            .pinned
                            .get(mc)
                            .or_else(|| p.coordinate_map.get(mc))?
                            .clone();
                        (coord, jr.min)
                    }
                }
            }
        };
        Some(PlatformPlan {
            id: platform.to_string(),
            gradle_path: format!("platforms:{platform}"),
            api_coordinate,
            java_target,
            metadata: MetadataFormat::PluginYml,
            capability: capabilities(platform, mc, p),
            experimental: p.experimental,
        })
    }

    /// Full resolution for `(mc, platforms)`; errors with a two-way list on miss.
    pub fn resolve(&self, mc: &str, platforms: &[String]) -> Result<Resolved> {
        let jr = self.ensure_targetable(mc)?;
        let normalized = self.normalize_platforms(platforms)?;
        let mut plans: BTreeMap<String, PlatformPlan> = BTreeMap::new();
        let mut missing: Vec<String> = Vec::new();
        for platform in &normalized {
            match self.platform_plan(mc, platform) {
                Some(plan) => {
                    plans.insert(platform.clone(), plan);
                }
                None => missing.push(platform.clone()),
            }
        }
        if !missing.is_empty() {
            return Err(self.unsupported_combination(mc, &missing));
        }
        // core compiles at the **lowest** enabled module target (§8.1).
        let core_java_target = plans
            .values()
            .map(|p| p.java_target)
            .min()
            .unwrap_or(jr.min);
        Ok(Resolved {
            mc_version: mc.to_string(),
            gradle_version: self.data.gradle.version.clone(),
            core_java_target,
            platforms: plans,
            thirdparty: self.thirdparty_coordinates(),
            quality: self.data.quality.clone(),
            gradle_plugins: self.gradle_plugins(),
        })
    }

    /// `resolve` plus the third-party Java-floor check (used by `init`). `features`
    /// are the `--features` values (`sqlite`, `bstats`, `placeholderapi`, …).
    pub fn resolve_with_features(
        &self,
        mc: &str,
        platforms: &[String],
        features: &[String],
    ) -> Result<Resolved> {
        let resolved = self.resolve(mc, platforms)?;
        let platform_target = resolved
            .platforms
            .values()
            .map(|p| p.java_target)
            .min()
            .unwrap_or(resolved.core_java_target);
        self.check_thirdparty(
            &thirdparty_keys_for_features(features),
            resolved.core_java_target,
            platform_target,
        )?;
        Ok(resolved)
    }

    /// Hard error when a requested library needs a newer Java than the module that
    /// consumes it provides (§11.1: never emit a project that cannot build).
    pub fn check_thirdparty(
        &self,
        keys: &[String],
        core_java_target: u8,
        platform_java_target: u8,
    ) -> Result<()> {
        for key in keys {
            // Unknown keys simply carry no third-party dependency.
            let Some(lib) = self.data.thirdparty.get(key) else {
                continue;
            };
            let (module, target) = match lib.scope {
                ThirdPartyScope::Core => ("core", core_java_target),
                ThirdPartyScope::Platform => ("platform", platform_java_target),
            };
            if target < lib.java_min {
                return Err(error::unsupported_combination(format!(
                    "附加模块 {key} 在此组合不可用：{} 要求 Java {}，而 {module} 模块目标为 Java {target}",
                    lib.coordinate, lib.java_min
                ))
                .with_hint("去掉该 --features 项，或选一个 Java 目标更高的 MC 版本"));
            }
        }
        Ok(())
    }

    /// Canonical key → coordinate map handed to the renderer.
    pub fn thirdparty_coordinates(&self) -> BTreeMap<String, String> {
        self.data
            .thirdparty
            .iter()
            .map(|(k, v)| (k.clone(), v.coordinate.clone()))
            .collect()
    }

    /// Human-readable support window, e.g. `1.8.9 … 26.2（共 61 个）`.
    pub fn support_window_text(&self) -> String {
        let v = self.supported_versions();
        match (v.first(), v.last()) {
            (Some(a), Some(b)) => format!("{a} … {b}（共 {} 个）", v.len()),
            _ => "（空）".to_string(),
        }
    }

    fn ensure_targetable(&self, mc: &str) -> Result<&JavaRequirement> {
        let jr = self.data.java.get(mc).ok_or_else(|| {
            error::unsupported_combination(format!(
                "MC 版本 {mc} 不在版本矩阵中\n  支持的版本: {}",
                self.support_window_text()
            ))
            .with_hint("矩阵按平台白名单建模，不做范围推导（spec §6.1）")
        })?;
        if !jr.selectable {
            return Err(error::unsupported_combination(format!(
                "MC 版本 {mc} 不作为可选目标（矩阵 selectable = false）\n  支持窗口: {}",
                self.support_window_text()
            ))
            .with_hint("上游转为 STABLE 后只改数据文件，不改代码"));
        }
        Ok(jr)
    }

    /// §11.3 shape: say what happened, list what is available both ways.
    fn unsupported_combination(&self, mc: &str, missing: &[String]) -> Error {
        let available = self.gated_platforms(mc);
        let available_text = if available.is_empty() {
            "（无）".to_string()
        } else {
            available.join(", ")
        };
        let mut msg = format!(
            "平台与版本组合不存在：{} × {mc}",
            missing.join(", ")
        );
        msg.push_str(&format!("\n  该版本可用平台: {available_text}"));
        for platform in missing {
            let versions = self.platform_versions(platform);
            let text = if versions.is_empty() {
                "（无）".to_string()
            } else {
                versions.join(", ")
            };
            if missing.len() == 1 {
                msg.push_str(&format!("\n  该平台可用版本: {text}"));
            } else {
                msg.push_str(&format!("\n  该平台可用版本（{platform}）: {text}"));
            }
        }
        error::unsupported_combination(msg)
            .with_hint(format!("复现: vinoa versions --matrix --mc {mc}"))
    }
}

/// `--features` values that pull in a matrix third-party coordinate.
pub fn thirdparty_keys_for_features(features: &[String]) -> Vec<String> {
    features
        .iter()
        .filter_map(|f| match f.as_str() {
            "sqlite" => Some("sqlite_jdbc".to_string()),
            "bstats" => Some("bstats".to_string()),
            "placeholderapi" => Some("placeholderapi".to_string()),
            // `update-check` and `gui` are self-written (no third-party coordinate).
            _ => None,
        })
        .collect()
}

/// Capability bits for one platform module (§7.1 / §7.6).
///
/// * `libraries_enabled` — `mc >= 1.16.5` (§7.5 `cap_libraries`).
/// * `plugin_yml_api_version` — `mc >= 1.13`, most permissive legal value ([LEG §5.1]).
/// * `legacy_namespace` — `paper` at `mc <= legacy_namespace_until` (§7.1).
/// * `paperweight` / `reobf` — **false** in v1: `ProjectSpec` carries no NMS flag and
///   the default is off (§7.1, §14). Floors for a future flag: `PAPERWEIGHT_FLOOR`
///   and `REOBF_UNTIL_EXCLUSIVE` above.
fn capabilities(platform: &str, mc: &str, p: &PlatformData) -> Capabilities {
    let plugin_host = matches!(platform, "paper" | "bukkit" | "folia");
    let api_version_line = plugin_host && cmp_mc(mc, API_VERSION_FLOOR) != Ordering::Less;
    let libraries_enabled = plugin_host && cmp_mc(mc, LIBRARIES_FLOOR) != Ordering::Less;
    let legacy_namespace = platform == "paper"
        && p.legacy_namespace_until
            .as_deref()
            .is_some_and(|until| cmp_mc(mc, until) != Ordering::Greater);
    Capabilities {
        plugin_yml_api_version: api_version_line.then(|| API_VERSION_VALUE.to_string()),
        libraries_enabled,
        run_task: matches!(platform, "paper" | "folia" | "velocity"),
        paperweight: false,
        reobf: false,
        legacy_namespace,
    }
}

/// Structural validation of a parsed matrix; any failure is a config error.
fn validate(data: &MatrixData) -> std::result::Result<(), String> {
    if data.schema != SCHEMA {
        return Err(format!("schema = {}（期望 {SCHEMA}）", data.schema));
    }
    if data.java.is_empty() {
        return Err("[java] 为空".to_string());
    }
    for mc in data.java.keys() {
        if mc_key(mc).is_none() {
            return Err(format!("[java] 键 {mc:?} 不是 X.Y[.Z] 形式的 MC 版本"));
        }
    }
    for (id, url) in data.repositories.iter() {
        if !url.starts_with("https://") {
            return Err(format!("repositories.{id} 不是 https URL: {url:?}"));
        }
    }
    let known_repo = |id: &str| data.repositories.contains_key(id);
    for (name, p) in data.platform.iter() {
        if p.api.is_empty() {
            return Err(format!("platform.{name} 缺 api"));
        }
        if let Some(repo) = p.repo.as_deref() {
            if !known_repo(repo) {
                return Err(format!("platform.{name}.repo = {repo:?} 不在 [repositories] 里"));
            }
        }
        match p.model.as_deref() {
            Some("protocol") => {
                if p.default_api.is_none() {
                    return Err(format!("platform.{name} 是 protocol 模型但缺 default_api"));
                }
                if p.java.is_none() {
                    return Err(format!("platform.{name} 是 protocol 模型但缺 java"));
                }
            }
            Some("major-grain") => {
                let default = p
                    .default
                    .as_ref()
                    .ok_or_else(|| format!("platform.{name} 缺 default"))?;
                if !p.coordinates.contains_key(default) {
                    return Err(format!(
                        "platform.{name}.default = {default:?} 不在 coordinates 里"
                    ));
                }
            }
            _ => {
                for v in p.versions.iter() {
                    let Some(java) = data.java.get(v) else {
                        return Err(format!("platform.{name}.versions 含未知版本 {v:?}"));
                    };
                    // A whitelisted but non-selectable version would be dead data
                    // (`platform_plan` refuses it): the whitelist and the support
                    // window must agree, so fail loudly instead.
                    if !java.selectable {
                        return Err(format!(
                            "platform.{name}.versions 含 {v:?}，但 [java].{v}.selectable = false"
                        ));
                    }
                    let has_coord = p.coordinate_map.contains_key(v)
                        || p.pinned.contains_key(v)
                        || p.coordinates.contains_key(v);
                    if !has_coord {
                        return Err(format!(
                            "platform.{name}.versions 含 {v:?} 但没有对应坐标（禁止推导）"
                        ));
                    }
                }
                for v in p.pinned.keys() {
                    if !p.versions.contains(v) {
                        return Err(format!("platform.{name}.pinned 的 {v:?} 不在 versions 里"));
                    }
                }
                for (v, c) in p.coordinates.iter() {
                    if !data.java.contains_key(v) {
                        return Err(format!("platform.{name}.coordinates 含未知版本 {v:?}"));
                    }
                    if !p.versions.contains(v) {
                        return Err(format!("platform.{name}.coordinates 的 {v:?} 不在 versions 里"));
                    }
                    if c.java == 0 {
                        return Err(format!("platform.{name}.coordinates.{v} 缺 java"));
                    }
                    if let Some(line) = p.api_versions.get(v) {
                        // The compile coordinate must live under the platform's
                        // artifact and on the same (major, minor) line as §7's
                        // `sponge_api` value; patch may differ because unreleased
                        // lines are pinned to the nearest published artifact.
                        let prefix = format!("{}:", p.api);
                        let artifact = c.api.rsplit(':').next().unwrap_or_default();
                        let same_line = match (artifact_key(artifact), artifact_key(line)) {
                            (Some(a), Some(b)) => a.0 == b.0 && a.1 == b.1 && a <= b,
                            _ => false,
                        };
                        if !c.api.starts_with(&prefix) || !same_line {
                            return Err(format!(
                                "platform.{name}: {v} 的坐标 {:?} 与 api_versions 的 {line:?} 不在同一线",
                                c.api
                            ));
                        }
                    }
                }
                for v in p.coordinate_map.keys() {
                    if !data.java.contains_key(v) {
                        return Err(format!("platform.{name}.coordinate_map 含未知版本 {v:?}"));
                    }
                }
                for v in p.api_versions.keys() {
                    if !p.versions.contains(v) {
                        return Err(format!("platform.{name}.api_versions 的 {v:?} 不在 versions 里"));
                    }
                }
                for (key, lib) in data.thirdparty.iter() {
                    for repo in &lib.repos {
                        if !known_repo(repo) {
                            return Err(format!(
                                "thirdparty.{key}.repos 含未知仓库 id {repo:?}"
                            ));
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
