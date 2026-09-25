//! Rendering context: `ProjectSpec` + `Resolved` -> deterministic variable map.
//! OWNER: engine-dev.
//!
//! Single source of truth for the template context. Everything derived (slugs,
//! PascalCase, package paths, module maps, capability switches) is computed
//! here — templates only ever print variables and branch on switches
//! (spec §7.1/§7.2/§7.5). Nothing time-, machine- or random-dependent may
//! enter this map: reproducibility is a hard requirement (§13).
use crate::error::{Result, unsupported_combination};
use crate::template::manifest;
use crate::types::{
    ArtifactLanguage, FeatureSet, MetadataFormat, PlatformPlan, ProjectSpec, Resolved, PLATFORMS,
};
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;

/// The template context. `serde_json::Map` keeps insertion order (deterministic
/// because we always build it in the same order) and serializes as a JSON object.
pub type Ctx = Map<String, Value>;

/// Platforms that are proxies (no server metadata / resources, spec §9.2).
pub const PROXY_PLATFORMS: [&str; 2] = ["velocity", "bungeecord"];

/// Canonical matrix keys (Lead-frozen):
/// `[quality]` -> quality scalar name, `[gradle]` -> plugin version name.
const QUALITY_SCALARS: [(&str, &str); 6] = [
    ("checkstyle", "checkstyleModernVersion"),
    ("checkstyle_legacy", "checkstyleLegacyVersion"),
    ("junit", "junitModernVersion"),
    ("junit_legacy", "junitLegacyVersion"),
    ("spotbugs", "spotbugsModernVersion"),
    ("spotbugs_legacy", "spotbugsLegacyVersion"),
];

const GRADLE_PLUGIN_SCALARS: [(&str, &str); 4] = [
    ("run_paper", "runPaperVersion"),
    ("run_velocity", "runVelocityVersion"),
    ("shadow", "shadowVersion"),
    ("paperweight", "paperweightVersion"),
];

/// §9.5 hard floors, used only when the matrix has no value (a warning is
/// emitted), so the matrix stays the single source of truth.
const SPEC_DEFAULTS: [(&str, &str); 8] = [
    ("checkstyle", "13.0.0"),
    ("checkstyle_legacy", "9.3"),
    ("junit", "6.1.3"),
    ("junit_legacy", "5.14.4"),
    ("spotbugs", "4.10.0"),
    ("spotbugs_legacy", "4.8.6"),
    ("run_paper", "3.1.0"),
    ("shadow", "9.6.1"),
];

/// `--yes`/flag defaults: `example` and `permissions` are on by default.
pub fn default_extras() -> BTreeMap<String, Value> {
    BTreeMap::new()
}

/// Ordered platform list: dedupe, fixed priority order, `paper` implies `bukkit`.
pub fn ordered_platforms(requested: &[String]) -> Vec<String> {
    let mut seen: Vec<String> = Vec::new();
    for p in requested {
        let p = p.trim().to_ascii_lowercase();
        if p.is_empty() || !PLATFORMS.contains(&p.as_str()) {
            continue;
        }
        if !seen.contains(&p) {
            seen.push(p);
        }
    }
    if seen.iter().any(|p| p == "paper") && !seen.iter().any(|p| p == crate::types::PAPER_IMPLIES)
    {
        seen.push(crate::types::PAPER_IMPLIES.to_string());
    }
    let mut out: Vec<String> = PLATFORMS
        .iter()
        .filter(|p| seen.iter().any(|s| s == *p))
        .map(|p| p.to_string())
        .collect();
    out.dedup();
    out
}

pub fn build_context(
    spec: &ProjectSpec,
    resolved: &Resolved,
    extras: &BTreeMap<String, Value>,
) -> Result<(Ctx, Vec<String>)> {
    let mut warnings = Vec::new();
    let platforms = ordered_platforms(&spec.platforms);
    let mut plans: BTreeMap<String, &PlatformPlan> = BTreeMap::new();
    for p in &platforms {
        match resolved.platforms.get(p) {
            Some(plan) => {
                plans.insert(p.clone(), plan);
            }
            None => {
                return Err(unsupported_combination(format!(
                    "版本矩阵结果缺少平台 `{p}`（resolved.platforms 里没有它）"
                )));
            }
        }
    }

    let plugin_id = slug(&spec.project_name);
    let plugin_name = pascal(&spec.project_name);
    let package_path = spec.package_name.replace('.', "/");
    let language = match spec.language {
        ArtifactLanguage::Zh => "zh",
        ArtifactLanguage::En => "en",
        ArtifactLanguage::Both => "both",
    };
    let metadata_format = match spec.metadata {
        MetadataFormat::PluginYml => "plugin.yml",
        MetadataFormat::PaperPluginYml => "paper-plugin.yml",
    };
    let mc = spec.mc_version.as_str();

    let mut api_coordinates = Map::new();
    let mut api_modules = Map::new();
    let mut api_versions = Map::new();
    let mut java_targets = Map::new();
    let mut gradle_paths = Map::new();
    let mut main_classes = Map::new();
    let mut is_server_platform = Map::new();
    for p in &platforms {
        let plan = plans[p];
        let (module, version) = split_coordinate(&plan.api_coordinate);
        api_coordinates.insert(p.clone(), json!(plan.api_coordinate));
        api_modules.insert(p.clone(), json!(module));
        api_versions.insert(p.clone(), json!(version));
        java_targets.insert(p.clone(), json!(plan.java_target));
        gradle_paths.insert(p.clone(), json!(format!("platforms:{p}")));
        main_classes.insert(
            p.clone(),
            json!(format!("{}.{}.{}", spec.package_name, p, plugin_name)),
        );
        is_server_platform.insert(p.clone(), json!(!is_proxy(p)));
    }

    let java_target_max = platforms
        .iter()
        .map(|p| plans[p].java_target)
        .max()
        .unwrap_or(17)
        .max(17);

    let any = |f: fn(&PlatformPlan) -> bool| platforms.iter().any(|p| f(plans[p]));
    let libraries_enabled =
        mc_ge(mc, "1.16.5") || any(|p| p.capability.libraries_enabled);
    let api_version_line = mc_ge(mc, "1.13")
        || any(|p| p.capability.plugin_yml_api_version.is_some());
    let legacy_namespace = any(|p| p.capability.legacy_namespace) || mc_le(mc, "1.16.5");
    let paperweight = any(|p| p.capability.paperweight);
    let reobf = any(|p| p.capability.reobf);
    let run_task = if platforms.iter().any(|p| p == "paper") {
        "run-paper"
    } else if platforms.iter().any(|p| p == "velocity") {
        "run-velocity"
    } else {
        "none"
    };
    let api_flavor = plans
        .get("bukkit")
        .map(|p| {
            if p.api_coordinate.contains("spigot") {
                "spigot"
            } else {
                "paper"
            }
        })
        .unwrap_or("spigot");
    let api_version_format = plans
        .get("bukkit")
        .map(|p| {
            let v = p.api_coordinate.rsplit(':').next().unwrap_or("");
            if v.contains("SNAPSHOT") {
                "R0.1-SNAPSHOT"
            } else if v.contains("build") {
                "build"
            } else {
                "pinned"
            }
        })
        .unwrap_or("R0.1-SNAPSHOT");

    let mut ctx = Map::new();
    // --- project identity -------------------------------------------------
    ctx.insert("projectName".into(), json!(spec.project_name));
    ctx.insert("pluginId".into(), json!(plugin_id));
    ctx.insert("pluginName".into(), json!(plugin_name));
    ctx.insert("packageName".into(), json!(spec.package_name));
    ctx.insert("packagePath".into(), json!(package_path));
    ctx.insert("projectVersion".into(), json!("0.1.0-SNAPSHOT"));
    ctx.insert("group".into(), json!(spec.package_name));
    ctx.insert("artifactName".into(), json!(spec.project_name));
    ctx.insert("commandName".into(), json!(truncate(&plugin_id, 32)));
    ctx.insert("permissionNode".into(), json!(format!("{plugin_id}.command")));
    ctx.insert("mcVersion".into(), json!(spec.mc_version));
    ctx.insert("language".into(), json!(language));
    ctx.insert("lang".into(), json!(language));
    ctx.insert("license".into(), json!(spec.license));
    ctx.insert("metadataFormat".into(), json!(metadata_format));
    ctx.insert("gradleVersion".into(), json!(resolved.gradle_version));
    ctx.insert("coreJavaTarget".into(), json!(resolved.core_java_target));
    ctx.insert("javaTargetMax".into(), json!(java_target_max));
    ctx.insert("ciJavaVersion".into(), json!(java_target_max));

    // --- version matrix maps (Lead-frozen names) --------------------------
    ctx.insert("platforms".into(), Value::Array(platforms.iter().cloned().map(Value::String).collect()));
    ctx.insert("apiCoordinates".into(), Value::Object(api_coordinates));
    ctx.insert("apiModules".into(), Value::Object(api_modules));
    ctx.insert("apiVersions".into(), Value::Object(api_versions));
    ctx.insert("javaTargets".into(), Value::Object(java_targets));
    ctx.insert("gradlePaths".into(), Value::Object(gradle_paths));
    ctx.insert("mainClasses".into(), Value::Object(main_classes));
    ctx.insert("isServerPlatform".into(), Value::Object(is_server_platform.clone()));

    // --- metadata / language / optional content ---------------------------
    ctx.insert(
        "pluginDescription".into(),
        json!(spec.description.clone().unwrap_or_default()),
    );
    ctx.insert("authors".into(), json!(spec.author));
    ctx.insert(
        "website".into(),
        json!(spec.website.clone().unwrap_or_default()),
    );
    ctx.insert(
        "bstatsPluginId".into(),
        match &spec.bstats_id {
            Some(id) => json!(id),
            None => Value::Null,
        },
    );

    // --- capability switches (spec §7.5/§7.6) -----------------------------
    let mut sw = Map::new();
    for p in &PLATFORMS {
        let on = platforms.iter().any(|x| x == p);
        sw.insert(format!("is_{p}"), json!(on));
        sw.insert(format!("cap_{p}_module"), json!(on));
    }
    let f: &FeatureSet = &spec.features;
    sw.insert("cap_example".into(), json!(spec.example));
    sw.insert("has_example".into(), json!(spec.example));
    sw.insert("exampleEnabled".into(), json!(spec.example));
    sw.insert("cap_permissions".into(), json!(spec.permissions));
    sw.insert("permissionsEnabled".into(), json!(spec.permissions));
    sw.insert(
        "cap_quality".into(),
        json!(spec.quality.checkstyle || spec.quality.unit_tests || spec.quality.ci),
    );
    sw.insert("qualityEnabled".into(), json!(spec.quality.checkstyle || spec.quality.unit_tests));
    sw.insert("cap_git".into(), json!(spec.git));
    sw.insert("gitInit".into(), json!(spec.git));
    sw.insert("cap_dual_lang".into(), json!(language == "both"));
    sw.insert("is_lang_zh".into(), json!(language == "zh"));
    sw.insert("is_lang_en".into(), json!(language == "en"));
    sw.insert(
        "is_paper_metadata".into(),
        json!(spec.metadata == MetadataFormat::PaperPluginYml),
    );
    sw.insert("cap_libraries".into(), json!(libraries_enabled));
    sw.insert("librariesEnabled".into(), json!(libraries_enabled));
    sw.insert("cap_api_version_line".into(), json!(api_version_line));
    sw.insert("cap_sqlite".into(), json!(f.sqlite));
    sw.insert("cap_bstats".into(), json!(f.bstats));
    sw.insert("cap_update_check".into(), json!(f.update_check));
    sw.insert("cap_placeholderapi".into(), json!(f.placeholderapi));
    sw.insert("cap_gui".into(), json!(f.gui));
    sw.insert("cap_spotbugs".into(), json!(f.spotbugs));
    sw.insert("cap_coverage".into(), json!(f.coverage));
    sw.insert("cap_release_ci".into(), json!(f.release_ci));
    sw.insert("cap_run_paper".into(), json!(run_task == "run-paper"));
    sw.insert(
        "cap_paper_brigadier".into(),
        json!(platforms.iter().any(|p| p == "paper") && mc_ge(mc, "1.20.6")),
    );
    sw.insert("cap_toolchain_download".into(), json!(spec.download_jdk));
    sw.insert("toolchainAutoDownload".into(), json!(spec.download_jdk));
    sw.insert("cap_legacy_namespace".into(), json!(legacy_namespace));
    sw.insert("legacyNamespace".into(), json!(legacy_namespace));
    sw.insert("has_authors".into(), json!(!spec.author.is_empty()));
    sw.insert(
        "has_description".into(),
        json!(spec.description.as_deref().is_some_and(|d| !d.trim().is_empty())),
    );
    sw.insert(
        "has_website".into(),
        json!(spec.website.as_deref().is_some_and(|w| !w.trim().is_empty())),
    );
    sw.insert("paperweightEnabled".into(), json!(paperweight));
    sw.insert("reobfEnabled".into(), json!(reobf));
    sw.insert("runTask".into(), json!(run_task));
    sw.insert("apiFlavor".into(), json!(api_flavor));
    sw.insert("apiVersionFormat".into(), json!(api_version_format));
    sw.insert(
        "pluginYmlApiVersion".into(),
        json!(if api_version_line { "1.13" } else { "" }),
    );
    sw.insert("metadataApiVersion".into(), json!("1.19"));
    sw.insert(
        "is_legacy_java".into(),
        json!(platforms.iter().any(|p| plans[p].java_target <= 8)),
    );
    ctx.extend(sw);

    // --- thirdparty / quality / run plugin versions -----------------------
    ctx.insert("thirdparty".into(), json!(resolved.thirdparty));
    ctx.insert("quality".into(), json!(resolved.quality));
    ctx.insert("gradlePlugins".into(), json!(resolved.gradle_plugins));
    let mut quality = Map::new();
    for key in ["checkstyle", "junit", "spotbugs"] {
        let legacy = format!("{key}_legacy");
        quality.insert(
            key.to_string(),
            json!(resolved.quality.get(key).cloned().unwrap_or_default()),
        );
        quality.insert(
            legacy.clone(),
            json!(resolved.quality.get(&legacy).cloned().unwrap_or_default()),
        );
    }
    ctx.insert("qualityToolVersions".into(), Value::Object(quality));
    for (key, name) in QUALITY_SCALARS {
        let value = match resolved.quality.get(key) {
            Some(v) => v.clone(),
            None => spec_default(&key, &mut warnings),
        };
        ctx.insert(name.to_string(), json!(value));
    }
    for (key, name) in GRADLE_PLUGIN_SCALARS {
        let value = match resolved.gradle_plugins.get(key) {
            Some(v) => v.clone(),
            None => spec_default(&key, &mut warnings),
        };
        ctx.insert(name.to_string(), json!(value));
    }

    // --- explicit overrides (tests / future CLI wiring) -------------------
    for (k, v) in extras {
        ctx.insert(k.clone(), v.clone());
    }

    Ok((ctx, warnings))
}

/// Overlay for one `foreach = "platforms"` iteration. All platform-sensitive
/// values are switched to the current platform (spec §7.4/§7.5).
pub fn loop_scope(base: &Ctx, platform: &str) -> Ctx {
    let mut ctx = base.clone();
    let map_get = |k: &str| {
        base.get(k)
            .and_then(|v| v.get(platform))
            .cloned()
            .unwrap_or(Value::Null)
    };
    let java_target = map_get("javaTargets");
    ctx.insert("platform".into(), json!(platform));
    ctx.insert("gradlePath".into(), json!(format!("platforms:{platform}")));
    ctx.insert("apiCoordinate".into(), map_get("apiCoordinates"));
    ctx.insert("apiModule".into(), map_get("apiModules"));
    ctx.insert("apiVersion".into(), map_get("apiVersions"));
    ctx.insert("javaTarget".into(), java_target.clone());
    ctx.insert("mainClass".into(), map_get("mainClasses"));
    ctx.insert("isServerPlatform".into(), json!(!is_proxy(platform)));
    // `is_<platform>` / `cap_<platform>_module` become "is it the current one".
    for p in PLATFORMS {
        let on = p == platform;
        ctx.insert(format!("is_{p}"), json!(on));
        ctx.insert(format!("cap_{p}_module"), json!(on));
    }
    ctx.insert(
        "is_legacy_java".into(),
        json!(java_target.as_u64().is_some_and(|t| t <= 8)),
    );
    ctx
}

pub fn is_proxy(platform: &str) -> bool {
    PROXY_PLATFORMS.contains(&platform)
}

fn spec_default(key: &str, warnings: &mut Vec<String>) -> String {
    let default = SPEC_DEFAULTS
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| (*v).to_string())
        .unwrap_or_default();
    if !default.is_empty() {
        warnings.push(format!(
            "版本矩阵缺少键 `{key}`，暂用 §9.5 默认值 {default}"
        ));
    }
    default
}

/// `group:artifact:version` -> (`group:artifact`, `version`).
pub fn split_coordinate(coord: &str) -> (String, String) {
    match coord.rfind(':') {
        Some(i) => (coord[..i].to_string(), coord[i + 1..].to_string()),
        None => (coord.to_string(), String::new()),
    }
}

/// `my-plugin` -> `myplugin` (spec §7.1: keep `[a-z0-9-_]`, lowercase).
pub fn slug(name: &str) -> String {
    name.to_ascii_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect()
}

/// `my-plugin` -> `MyPlugin` (class-name leaf, metadata `name`).
pub fn pascal(name: &str) -> String {
    let mut out = String::new();
    for part in name.split(|c: char| !c.is_ascii_alphanumeric()) {
        if part.is_empty() {
            continue;
        }
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            out.extend(first.to_uppercase());
            out.push_str(chars.as_str());
        }
    }
    out
}

/// A safe ASCII identifier-ish marker used for minijinja variable names.
pub fn is_identifier(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn truncate(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

pub fn mc_ge(mc: &str, floor: &str) -> bool {
    !manifest::version_gt(floor, mc)
}

pub fn mc_le(mc: &str, ceiling: &str) -> bool {
    !manifest::version_gt(mc, ceiling)
}
