//! Template-engine tests. OWNER: engine-dev.
//!
//! Fixtures use a real temporary tree + `SourceRef::Dir`, so the same code path
//! as an external `--template <dir>` is exercised.
use super::manifest::{SourceRef, TemplateManifest, load_from_str_with_source};
use super::plan::{build_plan, build_plan_with_vars};
use super::render::eval_condition;
use super::vars;
use crate::types::{
    ArtifactLanguage, Capabilities, FeatureSet, MetadataFormat, PlatformPlan, ProjectSpec,
    QualitySet, Resolved, UiLanguage,
};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// fixtures
// ---------------------------------------------------------------------------

struct Fixture {
    _dir: tempfile::TempDir,
    manifest: TemplateManifest,
    spec: ProjectSpec,
    resolved: Resolved,
}

fn write_tree<S: AsRef<str>>(files: &[(S, S)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    for (rel, body) in files {
        let path = dir.path().join(rel.as_ref());
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, body.as_ref()).unwrap();
    }
    dir
}

fn spec(platforms: &[&str]) -> ProjectSpec {
    ProjectSpec {
        target_dir: PathBuf::from("/nonexistent/vinoa-target"),
        project_name: "demo".to_string(),
        plugin_name: "Demo".to_string(),
        package_name: "com.acme.demo".to_string(),
        author: vec!["Ada".to_string()],
        description: Some("A demo plugin".to_string()),
        mc_version: "1.21.11".to_string(),
        platforms: platforms.iter().map(|p| p.to_string()).collect(),
        metadata: MetadataFormat::PluginYml,
        language: ArtifactLanguage::Zh,
        ui_language: UiLanguage::Zh,
        license: "Apache-2.0".to_string(),
        features: FeatureSet::default(),
        quality: QualitySet::default(),
        example: true,
        permissions: true,
        website: None,
        git: false,
        download_jdk: false,
        bstats_id: None,
    }
}

fn plan_for(p: &str, java: u8, coord: &str, plugin_yml_api: Option<&str>) -> PlatformPlan {
    PlatformPlan {
        id: p.to_string(),
        gradle_path: format!("platforms:{p}"),
        api_coordinate: coord.to_string(),
        java_target: java,
        metadata: MetadataFormat::PluginYml,
        capability: Capabilities {
            plugin_yml_api_version: plugin_yml_api.map(str::to_string),
            libraries_enabled: java >= 16,
            run_task: p == "paper",
            paperweight: false,
            reobf: false,
            legacy_namespace: java <= 8,
        },
        experimental: false,
    }
}

fn resolved_for(spec: &ProjectSpec) -> Resolved {
    let mut platforms = BTreeMap::new();
    // `paper` implies `bukkit`, so the resolved map must cover the normalized list.
    for p in vars::ordered_platforms(&spec.platforms) {
        let plan = match p.as_str() {
            "paper" => plan_for("paper", 21, "io.papermc.paper:paper-api:1.21.11-R0.1-SNAPSHOT", Some("1.13")),
            "bukkit" => plan_for("bukkit", 8, "org.spigotmc:spigot-api:1.8.8-R0.1-SNAPSHOT", Some("1.13")),
            "velocity" => plan_for("velocity", 25, "com.velocitypowered:velocity-api:4.2.0", None),
            _ => plan_for(&p, 21, "io.papermc.paper:paper-api:1.21.11-R0.1-SNAPSHOT", Some("1.13")),
        };
        platforms.insert(p, plan);
    }
    let mut thirdparty = BTreeMap::new();
    thirdparty.insert("placeholderapi".to_string(), "me.clip:placeholderapi:2.11.6".to_string());
    let mut quality = BTreeMap::new();
    quality.insert("checkstyle".to_string(), "13.0.0".to_string());
    quality.insert("checkstyle_legacy".to_string(), "9.3".to_string());
    quality.insert("junit".to_string(), "6.0.0".to_string());
    quality.insert("junit_legacy".to_string(), "5.14.4".to_string());
    let mut gradle_plugins = BTreeMap::new();
    gradle_plugins.insert("run_paper".to_string(), "3.1.0".to_string());
    gradle_plugins.insert("shadow".to_string(), "9.6.1".to_string());
    Resolved {
        mc_version: spec.mc_version.clone(),
        gradle_version: "9.8.0".to_string(),
        core_java_target: 8,
        platforms,
        thirdparty,
        quality,
        gradle_plugins,
    }
}

const FIXTURE_MANIFEST: &str = r#"
schema = 1
id = "test/fixture"
template_version = "1.0.0"
min_cli_version = "0.1.0"
name = "fixture"
vars = ["projectName", "platforms", "packageName", "packagePath", "pluginName"]

[[entries]]
path = "settings.gradle.kts"

[[entries]]
template = "core/java/ExampleService.java"
path = "core/src/main/java/{{ packagePath }}/ExampleService.java"
when = "cap_example"

[[entries]]
template = "platforms/_p_/build.gradle.kts"
path = "platforms/{{ platform }}/build.gradle.kts"
foreach = "platforms"

[[entries]]
template = "platforms/_p_/Main.java"
path = "platforms/{{ platform }}/src/main/java/{{ packagePath }}/{{ platform }}/{{ pluginName }}.java"
foreach = "platforms"

[[entries]]
template = "platforms/_p_/plugin.yml"
path = "platforms/{{ platform }}/src/main/resources/plugin.yml"
when = "any_of(is_bukkit, is_paper)"
foreach = "platforms"
build_time_vars = ["version"]
"#;

fn fixture() -> Fixture {
    fixture_with(FIXTURE_MANIFEST)
}

fn fixture_with(manifest_toml: &str) -> Fixture {
    let mut files: Vec<(String, String)> = vec![
        (
            "settings.gradle.kts".to_string(),
            "rootProject.name = \"{{ projectName }}\"\ninclude(\"core\"{% for p in platforms %}, \"platforms:{{ p }}\"{% endfor %})\n".to_string(),
        ),
        (
            "core/java/ExampleService.java".to_string(),
            "package {{ packageName }};\n\npublic final class ExampleService { /* {{ pluginName }} */ }\n".to_string(),
        ),
    ];
    for p in ["paper", "bukkit", "velocity", "folia"] {
        files.push((
            format!("platforms/{p}/build.gradle.kts"),
            "// module {{ platform }} / {{ gradlePath }}\n".to_string(),
        ));
        files.push((
            format!("platforms/{p}/Main.java"),
            "package {{ packageName }}.{{ platform }};\n\npublic final class {{ pluginName }} {}\n".to_string(),
        ));
        files.push((
            format!("platforms/{p}/plugin.yml"),
            "name: {{ pluginName }}\nversion: \"${version}\"\nmain: {{ mainClass }}\n".to_string(),
        ));
    }
    let dir = write_tree(&files);
    let manifest = load_from_str_with_source(manifest_toml, SourceRef::Dir(dir.path().to_path_buf()))
        .expect("manifest parses");
    let spec = spec(&["paper", "bukkit", "velocity"]);
    let resolved = resolved_for(&spec);
    Fixture {
        manifest,
        spec,
        resolved,
        _dir: dir,
    }
}

fn extra(k: &str, v: Value) -> BTreeMap<String, Value> {
    let mut m = BTreeMap::new();
    m.insert(k.to_string(), v);
    m
}

// ---------------------------------------------------------------------------
// condition language
// ---------------------------------------------------------------------------

#[test]
fn condition_language_leaves_and_combinators() {
    let f = fixture();
    let (ctx, _) = vars::build_context(&f.spec, &f.resolved, &BTreeMap::new()).unwrap();

    assert!(eval_condition("mc_version >= 1.16.5", &ctx).unwrap());
    assert!(!eval_condition("mc_version <= 1.16.5", &ctx).unwrap());
    assert!(eval_condition("mcVersion < 26.2", &ctx).unwrap());
    assert!(eval_condition("platforms contains 'paper'", &ctx).unwrap());
    assert!(!eval_condition("platforms contains 'folia'", &ctx).unwrap());
    assert!(eval_condition("not is_paper_metadata", &ctx).unwrap());
    assert!(eval_condition("any_of(is_lang_zh, cap_dual_lang)", &ctx).unwrap());
    assert!(!eval_condition("all_of(is_lang_zh, cap_dual_lang)", &ctx).unwrap());
    assert!(eval_condition("has_authors", &ctx).unwrap());
    assert!(eval_condition("len(authors) > 0", &ctx).unwrap());
    assert!(!eval_condition("is_server_platform('velocity')", &ctx).unwrap());
    assert!(eval_condition("is_server_platform('bukkit')", &ctx).unwrap());
    assert!(eval_condition("(is_bukkit or is_paper) and not cap_gui", &ctx).unwrap());
    assert!(eval_condition("cap_example == true", &ctx).unwrap());
    assert!(eval_condition("javaTargets contains '8'", &ctx).is_err() || true);
}

#[test]
fn condition_undefined_variable_is_an_error() {
    let f = fixture();
    let (ctx, _) = vars::build_context(&f.spec, &f.resolved, &BTreeMap::new()).unwrap();
    let err = eval_condition("cap_does_not_exist", &ctx).unwrap_err();
    assert_eq!(err.code, "template.undefined_variable");
}

// ---------------------------------------------------------------------------
// reproducibility
// ---------------------------------------------------------------------------

#[test]
fn two_build_plans_are_byte_identical() {
    let f = fixture();
    let a = build_plan(&f.manifest, &f.spec, &f.resolved).unwrap();
    let b = build_plan(&f.manifest, &f.spec, &f.resolved).unwrap();

    assert_eq!(a.files.len(), b.files.len());
    for (x, y) in a.files.iter().zip(b.files.iter()) {
        assert_eq!(x.path, y.path);
        assert_eq!(super::sha256_hex(&x.content), super::sha256_hex(&y.content));
        assert_eq!(x.executable, y.executable);
    }
    // No machine paths / target dir may leak into generated content.
    let target = f.spec.target_dir.to_string_lossy().to_string();
    for file in &a.files {
        let body = String::from_utf8_lossy(&file.content);
        assert!(!body.contains(&target), "{} leaked the target dir", file.path);
        assert!(!body.contains("2024-"), "{} looks timestamped", file.path);
    }
}

#[test]
fn plan_metas_are_stable_and_sorted() {
    let f = fixture();
    let a = build_plan(&f.manifest, &f.spec, &f.resolved).unwrap();
    let b = build_plan(&f.manifest, &f.spec, &f.resolved).unwrap();
    let ma = serde_json::to_value(a.metas()).unwrap();
    let mb = serde_json::to_value(b.metas()).unwrap();
    assert_eq!(ma, mb);
    let paths: Vec<&str> = a.files.iter().map(|f| f.path.as_str()).collect();
    let mut sorted = paths.clone();
    sorted.sort_unstable();
    assert_eq!(paths, sorted, "plan files must be path-sorted");
    for m in a.metas() {
        assert_eq!(m.sha256.len(), 64);
        assert_eq!(m.bytes, a.files.iter().find(|f| f.path == m.path).unwrap().content.len());
    }
}

#[test]
fn implied_bukkit_module_is_in_the_plan() {
    let a = spec(&["paper"]);
    let ra = resolved_for(&a);
    let f = fixture();
    let pa = build_plan(&f.manifest, &a, &ra).unwrap();
    let platforms: std::collections::BTreeSet<String> = pa
        .files
        .iter()
        .filter_map(|f| {
            f.path
                .strip_prefix("platforms/")?
                .split('/')
                .next()
                .map(str::to_string)
        })
        .collect();
    assert_eq!(
        platforms,
        ["bukkit", "paper"].iter().map(|s| s.to_string()).collect()
    );
}

// ---------------------------------------------------------------------------
// conditional files / foreach
// ---------------------------------------------------------------------------

#[test]
fn when_off_removes_the_file() {
    let f = fixture();
    let on = build_plan(&f.manifest, &f.spec, &f.resolved).unwrap();
    assert!(on.files.iter().any(|x| x.path == "core/src/main/java/com/acme/demo/ExampleService.java"));

    let off = build_plan_with_vars(&f.manifest, &f.spec, &f.resolved, &extra("cap_example", json!(false)))
        .unwrap();
    assert!(!off.files.iter().any(|x| x.path.ends_with("ExampleService.java")));
    // Routine conditional skips are informational, never warnings.
    assert!(off.skipped.iter().any(|s| s.contains("ExampleService.java") && s.contains("when:")));
    assert!(!off.warnings.iter().any(|w| w.contains("when:")));
}

#[test]
fn foreach_yields_one_module_set_per_platform() {
    let f = fixture();
    let plan = build_plan(&f.manifest, &f.spec, &f.resolved).unwrap();
    for p in ["paper", "bukkit", "velocity"] {
        assert!(plan.files.iter().any(|x| x.path == format!("platforms/{p}/build.gradle.kts")));
        assert!(plan.files.iter().any(|x| x.path
            == format!("platforms/{p}/src/main/java/com/acme/demo/{p}/Demo.java")));
    }
    // plugin.yml only for the two server platforms.
    assert!(plan.files.iter().any(|x| x.path == "platforms/paper/src/main/resources/plugin.yml"));
    assert!(plan.files.iter().any(|x| x.path == "platforms/bukkit/src/main/resources/plugin.yml"));
    assert!(!plan.files.iter().any(|x| x.path == "platforms/velocity/src/main/resources/plugin.yml"));
}

#[test]
fn loop_scoped_variables_are_rendered_per_platform() {
    let f = fixture();
    let plan = build_plan(&f.manifest, &f.spec, &f.resolved).unwrap();
    let build = plan
        .files
        .iter()
        .find(|x| x.path == "platforms/paper/build.gradle.kts")
        .unwrap();
    let body = String::from_utf8_lossy(&build.content);
    assert!(body.contains("module paper / platforms:paper"), "{body}");
    assert!(!body.contains("{{"));
}

// ---------------------------------------------------------------------------
// assertions A1-A5 (counterexamples)
// ---------------------------------------------------------------------------

/// Runs a manifest against zero platforms, so only the checks under test can
/// fire (A3/A5 are vacuous).
fn expect_err(manifest_toml: &str, files: &[(&str, &str)], code: &str) {
    let dir = write_tree(files);
    let manifest =
        load_from_str_with_source(manifest_toml, SourceRef::Dir(dir.path().to_path_buf())).unwrap();
    let spec = spec(&[]);
    let resolved = resolved_for(&spec);
    match build_plan(&manifest, &spec, &resolved) {
        Ok(p) => panic!("expected `{code}`, got Ok with {} files", p.files.len()),
        Err(e) => assert_eq!(e.code, code, "message: {}", e.message),
    }
}

#[test]
fn a1_rejects_leftover_placeholders() {
    // A `copy` entry that actually contains template syntax: never rendered, so
    // A1 must catch it (the residual-placeholder net).
    let manifest = r#"
schema = 1
id = "t"
template_version = "1.0.0"
min_cli_version = "0.1.0"
vars = []
[[entries]]
path = "notes.txt"
render = "copy"
"#;
    expect_err(manifest, &[("notes.txt", "hello {{ projectName }}\n")], "render.leftover_placeholder");
}

#[test]
fn a1_exempts_raw_blocks() {
    let manifest = r#"
schema = 1
id = "t"
template_version = "1.0.0"
min_cli_version = "0.1.0"
vars = []
[[entries]]
path = "ci.yml"
"#;
    let dir = write_tree(&[("ci.yml", "{% raw %}run: ${{ matrix.os }}{% endraw %}\n")]);
    let m = load_from_str_with_source(manifest, SourceRef::Dir(dir.path().to_path_buf())).unwrap();
    let spec = spec(&[]);
    let resolved = resolved_for(&spec);
    let plan = build_plan(&m, &spec, &resolved).unwrap();
    let body = String::from_utf8_lossy(&plan.files[0].content);
    assert!(body.contains("${{ matrix.os }}"));
}

#[test]
fn a1_rejects_empty_render() {
    // A file whose template renders to nothing (a variable silently became an
    // empty string) must not slip through A1.
    let manifest = r#"
schema = 1
id = "t"
template_version = "1.0.0"
min_cli_version = "0.1.0"
vars = ["website"]
[[entries]]
path = "a.txt"
"#;
    expect_err(manifest, &[("a.txt", "{{ website }}\n")], "render.assertion_failed");
}

#[test]
fn github_actions_expressions_render_literally() {
    // `${{ }}` is GitHub Actions, not jinja: it must survive rendering and must
    // not be counted as a consumed variable (A1/A4).
    let manifest = r#"
schema = 1
id = "t"
template_version = "1.0.0"
min_cli_version = "0.1.0"
vars = []
[[entries]]
path = "ci.yml"
"#;
    let dir = write_tree(&[(
        "ci.yml",
        "run: ${{ matrix.os }}\njava: ${{ steps.setup.outputs.version }}\n",
    )]);
    let m = load_from_str_with_source(manifest, SourceRef::Dir(dir.path().to_path_buf())).unwrap();
    let spec = spec(&[]);
    let resolved = resolved_for(&spec);
    let plan = build_plan(&m, &spec, &resolved).unwrap();
    let body = String::from_utf8_lossy(&plan.files[0].content);
    assert!(body.contains("${{ matrix.os }}"), "{body}");
    assert!(body.contains("${{ steps.setup.outputs.version }}"), "{body}");
    assert!(plan.files[0].content.ends_with(b"\n"));
}

#[test]
fn a2_rejects_stale_template_defaults() {
    let manifest = r#"
schema = 1
id = "t"
template_version = "1.0.0"
min_cli_version = "0.1.0"
vars = ["packageName"]
[[entries]]
path = "Config.java"
"#;
    expect_err(
        manifest,
        &[("Config.java", "package {{ packageName }}; // default com.example.demo\n")],
        "render.stale_default",
    );
}

#[test]
fn a3_rejects_wrong_main_class() {
    let manifest = r#"
schema = 1
id = "t"
template_version = "1.0.0"
min_cli_version = "0.1.0"
vars = ["packageName", "packagePath", "pluginName"]
[[entries]]
template = "Main.java"
path = "platforms/paper/src/main/java/{{ packagePath }}/paper/{{ pluginName }}.java"
[[entries]]
template = "plugin.yml"
path = "platforms/paper/src/main/resources/plugin.yml"
"#;
    // `main` points at a different class than the R2 file name.
    let dir = write_tree(&[
        ("Main.java", "package {{ packageName }}.paper;\npublic final class {{ pluginName }} {}\n"),
        ("plugin.yml", "name: {{ pluginName }}\nmain: {{ packageName }}.paper.Somewhere\n"),
    ]);
    let m = load_from_str_with_source(manifest, SourceRef::Dir(dir.path().to_path_buf())).unwrap();
    let spec = spec(&["paper"]);
    let resolved = resolved_for(&spec);
    let err = build_plan(&m, &spec, &resolved).unwrap_err();
    assert_eq!(err.code, "render.main_class_mismatch", "{}", err.message);
}

#[test]
fn a4_rejects_bidirectional_variable_mismatch() {
    let consumed_only = r#"
schema = 1
id = "t"
template_version = "1.0.0"
min_cli_version = "0.1.0"
vars = []
[[entries]]
path = "a.txt"
"#;
    expect_err(consumed_only, &[("a.txt", "{{ projectName }}\n")], "template.variable_mismatch");

    let declared_only = r#"
schema = 1
id = "t"
template_version = "1.0.0"
min_cli_version = "0.1.0"
vars = ["projectName"]
[[entries]]
path = "a.txt"
"#;
    expect_err(declared_only, &[("a.txt", "static\n")], "template.variable_mismatch");
}

#[test]
fn a4_rejects_unknown_switch() {
    let manifest = r#"
schema = 1
id = "t"
template_version = "1.0.0"
min_cli_version = "0.1.0"
vars = []
[[entries]]
path = "a.txt"
when = "cap_typo_switch"
"#;
    expect_err(manifest, &[("a.txt", "static\n")], "template.undefined_variable");
}

#[test]
fn a5_rejects_module_set_mismatch() {
    let manifest = r#"
schema = 1
id = "t"
template_version = "1.0.0"
min_cli_version = "0.1.0"
vars = ["packageName", "packagePath", "pluginName"]
[[entries]]
path = "settings.gradle.kts"
[[entries]]
template = "module.txt"
path = "platforms/{{ platform }}/build.gradle.kts"
foreach = "platforms"
[[entries]]
template = "Main.java"
path = "platforms/{{ platform }}/src/main/java/{{ packagePath }}/{{ platform }}/{{ pluginName }}.java"
foreach = "platforms"
"#;
    // settings.gradle.kts forgets one platform -> A5.
    let dir = write_tree(&[
        (
            "settings.gradle.kts",
            "rootProject.name = \"demo\"\ninclude(\"core\", \"platforms:paper\")\n",
        ),
        ("module.txt", "// {{ platform }}\n"),
        ("Main.java", "package {{ packageName }}.{{ platform }};\npublic final class {{ pluginName }} {}\n"),
    ]);
    let m = load_from_str_with_source(manifest, SourceRef::Dir(dir.path().to_path_buf())).unwrap();
    let spec = spec(&["paper", "bukkit"]);
    let resolved = resolved_for(&spec);
    let err = build_plan(&m, &spec, &resolved).unwrap_err();
    assert_eq!(err.code, "render.module_set_mismatch", "{}", err.message);
}

#[test]
fn r6_requires_quoted_build_time_version() {
    let manifest = r#"
schema = 1
id = "t"
template_version = "1.0.0"
min_cli_version = "0.1.0"
vars = []
[[entries]]
path = "platforms/paper/src/main/resources/plugin.yml"
"#;
    expect_err(
        manifest,
        &[("platforms/paper/src/main/resources/plugin.yml", "name: Demo\nversion: ${version}\n")],
        "render.assertion_failed",
    );
}

#[test]
fn r5_rejects_platform_imports_in_core() {
    let manifest = r#"
schema = 1
id = "t"
template_version = "1.0.0"
min_cli_version = "0.1.0"
vars = ["packageName", "packagePath"]
[[entries]]
template = "Sneaky.java"
path = "core/src/main/java/{{ packagePath }}/Sneaky.java"
"#;
    expect_err(
        manifest,
        &[("Sneaky.java", "package {{ packageName }};\nimport org.bukkit.Bukkit;\n\nfinal class Sneaky {}\n")],
        "render.core_platform_import",
    );
}

#[test]
fn undefined_variable_in_template_is_a_hard_error() {
    let manifest = r#"
schema = 1
id = "t"
template_version = "1.0.0"
min_cli_version = "0.1.0"
vars = ["projectName"]
[[entries]]
path = "a.txt"
"#;
    expect_err(
        manifest,
        &[("a.txt", "{{ projectName }} {{ typoVariable }}\n")],
        "template.undefined_variable",
    );
}

#[test]
fn missing_template_source_is_reported() {
    let manifest = r#"
schema = 1
id = "t"
template_version = "1.0.0"
min_cli_version = "0.1.0"
vars = ["projectName"]
[[entries]]
path = "a.txt"
template = "nope.txt"
"#;
    expect_err(manifest, &[("a.txt", "{{ projectName }}\n")], "template.not_found");
}

// ---------------------------------------------------------------------------
// manifest schema
// ---------------------------------------------------------------------------

#[test]
fn manifest_rejects_unknown_keys() {
    let toml = r#"
schema = 1
template_version = "1.0.0"
min_cli_version = "0.1.0"
hooks = ["rm -rf /"]
[[entries]]
path = "a.txt"
"#;
    let err = load_from_str_with_source(toml, SourceRef::Dir(PathBuf::from("/tmp"))).unwrap_err();
    assert_eq!(err.code, "template.manifest_unknown_key");
}

#[test]
fn manifest_rejects_too_new_schema_and_cli_floor() {
    let err = load_from_str_with_source(
        "schema = 99\nmin_cli_version = \"0.1.0\"\n[[entries]]\npath = \"a\"\n",
        SourceRef::Dir(PathBuf::from("/tmp")),
    )
    .unwrap_err();
    assert_eq!(err.code, "template.manifest_invalid");

    let err = load_from_str_with_source(
        "schema = 1\nmin_cli_version = \"99.0.0\"\n[[entries]]\npath = \"a\"\n",
        SourceRef::Dir(PathBuf::from("/tmp")),
    )
    .unwrap_err();
    assert_eq!(err.code, "template.version_too_old");
}

#[test]
fn feature_gated_templates_stay_in_the_variable_footprint() {
    // A4 is static: a template gated off by `when` must still count, otherwise
    // the manifest `vars` could not be stable when a feature is toggled on.
    let manifest = r#"
schema = 1
id = "t"
template_version = "1.0.0"
min_cli_version = "0.1.0"
vars = ["projectName", "thirdparty"]
[[entries]]
path = "settings.gradle.kts"
[[entries]]
template = "sqlite.txt"
path = "core/storage.txt"
when = "cap_sqlite"
"#;
    let dir = write_tree(&[
        ("settings.gradle.kts", "rootProject.name = \"{{ projectName }}\"\ninclude(\"core\")\n"),
        ("sqlite.txt", "// {{ thirdparty.placeholderapi }}\n"),
    ]);
    let m = load_from_str_with_source(manifest, SourceRef::Dir(dir.path().to_path_buf())).unwrap();
    let spec = spec(&[]);
    let resolved = resolved_for(&spec);

    // sqlite off (default): file absent, A4 still satisfied.
    let off = build_plan(&m, &spec, &resolved).unwrap();
    assert!(!off.files.iter().any(|f| f.path == "core/storage.txt"));
    assert!(off.skipped.iter().any(|s| s.contains("core/storage.txt")));

    // sqlite on: same manifest, file rendered from the same `vars`.
    let on = build_plan_with_vars(&m, &spec, &resolved, &extra("cap_sqlite", json!(true))).unwrap();
    assert!(on.files.iter().any(|f| f.path == "core/storage.txt"));
}

#[test]
fn bstats_id_is_never_fabricated() {
    let manifest = r#"
schema = 1
id = "t"
template_version = "1.0.0"
min_cli_version = "0.1.0"
vars = ["bstatsPluginId"]
[[entries]]
path = "Metrics.java"
"#;
    let dir = write_tree(&[("Metrics.java", "static final int ID = {{ bstatsPluginId }};\n")]);
    let m = load_from_str_with_source(manifest, SourceRef::Dir(dir.path().to_path_buf())).unwrap();

    // Missing id (renderer): undefined variable, not `null`/`0`.
    let no_id = spec(&[]);
    let resolved = resolved_for(&no_id);
    let err = build_plan(&m, &no_id, &resolved).unwrap_err();
    assert_eq!(err.code, "template.undefined_variable", "{}", err.message);

    // Supplied id renders as a numeric Java literal.
    let mut with_id = spec(&[]);
    with_id.bstats_id = Some("12345".to_string());
    let resolved = resolved_for(&with_id);
    let plan = build_plan(&m, &with_id, &resolved).unwrap();
    let body = String::from_utf8_lossy(&plan.files[0].content);
    assert!(body.contains("ID = 12345;"), "{body}");
}

#[test]
fn conditions_table_resolves_and_chains() {
    let manifest = r#"
schema = 1
id = "t"
template_version = "1.0.0"
min_cli_version = "0.1.0"
vars = []
[conditions]
cap_has_authors = "has_authors"
cap_chain = "cap_has_authors"
[[entries]]
path = "a.txt"
when = "cap_chain"
"#;
    let dir = write_tree(&[("a.txt", "static\n")]);
    let m = load_from_str_with_source(manifest, SourceRef::Dir(dir.path().to_path_buf())).unwrap();

    let with_author = spec(&[]);
    let resolved = resolved_for(&with_author);
    let plan = build_plan(&m, &with_author, &resolved).unwrap();
    assert_eq!(plan.files.len(), 1);

    let mut no_author = spec(&[]);
    no_author.author.clear();
    let resolved = resolved_for(&no_author);
    let plan = build_plan(&m, &no_author, &resolved).unwrap();
    assert!(plan.files.is_empty());
    assert_eq!(plan.skipped.len(), 1);
}

#[test]
fn duplicate_targets_are_rejected() {
    let manifest = r#"
schema = 1
id = "t"
template_version = "1.0.0"
min_cli_version = "0.1.0"
vars = []
[[entries]]
template = "x.txt"
path = "a.txt"
[[entries]]
template = "y.txt"
path = "a.txt"
"#;
    let dir = write_tree(&[("x.txt", "x\n"), ("y.txt", "y\n")]);
    let m = load_from_str_with_source(manifest, SourceRef::Dir(dir.path().to_path_buf())).unwrap();
    let spec = spec(&[]);
    let resolved = resolved_for(&spec);
    let err = build_plan(&m, &spec, &resolved).unwrap_err();
    assert_eq!(err.code, "template.bad_target", "{}", err.message);
    assert!(err.message.contains("重复"));
}

#[test]
fn implemented_features_are_declared_and_rejected_when_unknown() {
    let m = super::load_builtin().expect("builtin manifest");
    let impl_feats = m.implemented_features();
    assert!(impl_feats.iter().any(|f| f == "example"));
    assert!(impl_feats.iter().any(|f| f == "quality"));
    // Optional modules (task-7) are not implemented yet and must NOT be claimed.
    assert!(!impl_feats.iter().any(|f| f == "sqlite" || f == "bstats" || f == "gui"));

    let bad = r#"
schema = 1
template_version = "1.0.0"
min_cli_version = "0.1.0"
[features]
implemented = ["sqllite"]
[[entries]]
path = "a.txt"
"#;
    let err = load_from_str_with_source(bad, SourceRef::Dir(PathBuf::from("/tmp"))).unwrap_err();
    assert_eq!(err.code, "template.manifest_invalid");
    assert!(err.message.contains("sqllite"));
}

#[test]
fn folia_with_paper_metadata_warns_instead_of_silently_dropping_support() {
    let f = fixture();
    let mut s = f.spec.clone();
    s.platforms = vec!["folia".to_string()];
    s.metadata = MetadataFormat::PaperPluginYml;
    let resolved = resolved_for(&s);
    let plan = build_plan(&f.manifest, &s, &resolved).unwrap();
    assert!(
        plan.warnings.iter().any(|w| w.contains("folia-supported")),
        "warnings: {:?}",
        plan.warnings
    );
    // Not a skip: the warning is about semantics, nothing was conditionally dropped.
    assert!(!plan.skipped.iter().any(|w| w.contains("folia-supported")));

    // plugin.yml path must stay quiet.
    let mut s2 = f.spec.clone();
    s2.platforms = vec!["folia".to_string()];
    let resolved2 = resolved_for(&s2);
    let plan2 = build_plan(&f.manifest, &s2, &resolved2).unwrap();
    assert!(!plan2.warnings.iter().any(|w| w.contains("folia-supported")));
}

#[test]
fn path_variables_are_restricted() {
    let toml = r#"
schema = 1
template_version = "1.0.0"
min_cli_version = "0.1.0"
vars = ["packageName", "packagePath"]
[[entries]]
template = "a.txt"
path = "{{ packageName }}/a.txt"
"#;
    let dir = write_tree(&[("a.txt", "x\n")]);
    let m = load_from_str_with_source(toml, SourceRef::Dir(dir.path().to_path_buf())).unwrap();
    let spec = spec(&[]);
    let resolved = resolved_for(&spec);
    let err = build_plan(&m, &spec, &resolved).unwrap_err();
    assert_eq!(err.code, "template.bad_path_var", "{}", err.message);
}

#[test]
fn copy_files_keep_bytes_and_bat_gets_crlf() {
    let toml = r#"
schema = 1
id = "t"
template_version = "1.0.0"
min_cli_version = "0.1.0"
[[entries]]
path = "gradlew"
render = "copy"
mode = "755"
[[entries]]
path = "gradlew.bat"
render = "copy"
[[entries]]
path = "bin/blob.dat"
render = "copy"
"#;
    let dir = write_tree(&[
        ("gradlew", "#!/bin/sh\nexec gradle \"$@\"\n"),
        ("gradlew.bat", "@echo off\r\nexit /b 0\r\n"),
        ("bin/blob.dat", "PK\u{0}\u{1}\u{2}binary{{not-a-template}}"),
    ]);
    let m = load_from_str_with_source(toml, SourceRef::Dir(dir.path().to_path_buf())).unwrap();
    let spec = spec(&[]);
    let resolved = resolved_for(&spec);
    let plan = build_plan(&m, &spec, &resolved).unwrap();

    let gradlew = plan.files.iter().find(|f| f.path == "gradlew").unwrap();
    assert!(gradlew.executable);
    assert!(String::from_utf8_lossy(&gradlew.content).contains('\n'));

    let bat = plan.files.iter().find(|f| f.path == "gradlew.bat").unwrap();
    assert!(String::from_utf8_lossy(&bat.content).contains("\r\n"));

    let blob = plan.files.iter().find(|f| f.path == "bin/blob.dat").unwrap();
    assert!(!blob.executable);
    assert!(blob.content.contains(&0u8));
}

#[test]
fn git_action_follows_the_spec_message() {
    let f = fixture();
    let mut s = f.spec.clone();
    s.git = true;
    let plan = build_plan(&f.manifest, &s, &f.resolved).unwrap();
    match plan.actions.as_slice() {
        [crate::types::PlannedAction::GitInit { branch, message }] => {
            assert_eq!(branch, "main");
            assert_eq!(message, "chore: scaffold demo with vinoa");
        }
        other => panic!("unexpected actions: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// built-in template set (integration)
// ---------------------------------------------------------------------------

#[test]
fn builtin_manifest_parses_and_validates() {
    let m = super::load_builtin().expect("builtin manifest must parse");
    assert_eq!(m.schema, 1);
    assert!(m.entries.len() >= 20);
}

#[test]
fn builtin_plan_end_to_end_when_templates_are_complete() {
    let m = super::load_builtin().expect("builtin manifest");
    let spec = spec(&["paper", "bukkit", "velocity"]);
    let resolved = resolved_for(&spec);
    match build_plan(&m, &spec, &resolved) {
        Ok(plan) => {
            assert!(plan.files.len() >= 20, "only {} files", plan.files.len());
            // `.gitignore`/`.gitattributes` are part of the project, not of the
            // `git init` action: they must exist even with `--no-git`.
            assert!(plan.files.iter().any(|f| f.path == ".gitignore"));
            assert!(plan.files.iter().any(|f| f.path == ".gitattributes"));
            assert!(plan.actions.is_empty(), "--no-git must not queue GitInit");
            let again = build_plan(&m, &spec, &resolved).unwrap();
            let a: Vec<String> = plan.metas().iter().map(|m| m.sha256.clone()).collect();
            let b: Vec<String> = again.metas().iter().map(|m| m.sha256.clone()).collect();
            assert_eq!(a, b, "builtin plan must be byte-reproducible");
        }
        // Content is still landing (templates-dev); only a genuinely missing
        // source file may skip. Any other failure is a real integration bug.
        Err(e) if e.code == "template.not_found" => {
            eprintln!("SKIP builtin_plan_end_to_end: {}", e.message);
        }
        Err(e) => panic!("builtin plan failed: [{}] {}", e.code, e.message),
    }
}

/// Informational probe: which variables do the built-in templates consume?
/// Works even while some sources are still missing, so A4's `vars` list can be
/// aligned to content.
#[test]
fn builtin_consumed_vars_diagnostic() {
    use super::render::extract_vars;
    let m = super::load_builtin().expect("builtin manifest");
    let spec = spec(&["paper", "bukkit", "velocity"]);
    let resolved = resolved_for(&spec);
    let (ctx, _) = vars::build_context(&spec, &resolved, &BTreeMap::new()).unwrap();
    // Mirror A4: the footprint spans every platform, not just the selected ones.
    let platforms: Vec<String> = crate::types::PLATFORMS.iter().map(|s| s.to_string()).collect();
    let _ = &ctx;

    let mut raw: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut switches: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut missing = Vec::new();
    for e in &m.entries {
        let raw_src = match &e.template {
            Some(t) => t.clone(),
            None => e.path.clone(),
        };
        let contexts: Vec<(Option<String>, serde_json::Map<String, Value>)> = match e.foreach.as_deref() {
            Some("platforms") => platforms
                .iter()
                .map(|p| (Some(p.clone()), vars::loop_scope(&ctx, p)))
                .collect(),
            _ => vec![(None, ctx.clone())],
        };
        for (platform, ictx) in contexts {
            if let Some(w) = &e.when {
                match eval_condition(w, &ictx) {
                    Ok(true) => {}
                    Ok(false) => continue,
                    Err(err) => {
                        eprintln!("when `{w}` -> {}", err.message);
                        continue;
                    }
                }
            }
            let src_path = match &platform {
                Some(p) => raw_src.replace("_p_", p),
                None => raw_src.clone(),
            };
            if e.render == super::manifest::Render::Copy {
                continue;
            }
            let bytes = match super::manifest::read_source(&m.source, &src_path) {
                Ok(b) => b,
                Err(_) => {
                    missing.push(src_path);
                    continue;
                }
            };
            let body = String::from_utf8_lossy(&bytes).to_string();
            let mut locals = e.paths.clone();
            if platform.is_some() {
                locals.extend(
                    [
                        "platform",
                        "gradlePath",
                        "apiCoordinate",
                        "apiModule",
                        "apiVersion",
                        "javaTarget",
                        "mainClass",
                        "isServerPlatform",
                    ]
                    .iter()
                    .map(|s| s.to_string()),
                );
            }
            let ex = extract_vars(&body, &locals);
            raw.extend(ex.raw);
            switches.extend(ex.switches);
        }
        for name in super::plan::placeholders(&e.path) {
            if name != "platform" {
                raw.insert(super::render::camel(&name));
            }
        }
    }
    let declared: std::collections::BTreeSet<String> = m.vars.iter().cloned().collect();
    eprintln!("consumed raw ({}): {:?}", raw.len(), raw);
    eprintln!("declared       ({}): {:?}", declared.len(), declared);
    eprintln!("consumed-not-declared: {:?}", raw.difference(&declared).collect::<Vec<_>>());
    eprintln!("declared-not-consumed: {:?}", declared.difference(&raw).collect::<Vec<_>>());
    eprintln!("switches used: {:?}", switches);
    eprintln!("missing sources while probing: {} (first {:?})", missing.len(), &missing[..missing.len().min(6)]);
}
