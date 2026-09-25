//! Matrix unit tests (task-1 acceptance + data invariants). OWNER: matrix-dev.
use super::query::{thirdparty_keys_for_features, ThirdPartyScope};
use super::refresh::{
    builds_url, is_release_version, latest_url, pick_stable, Build, ProjectVersions, API_BASE,
    USER_AGENT,
};
use super::Matrix;
use crate::types::PAPER_IMPLIES;

fn m() -> Matrix {
    Matrix::builtin().expect("builtin matrix must parse and validate")
}

fn pl(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

fn plan<'a>(matrix: &'a Matrix, mc: &str, platform: &str) -> crate::types::PlatformPlan {
    matrix
        .platform_plan(mc, platform)
        .unwrap_or_else(|| panic!("expected {platform} × {mc} to exist"))
}

// ------------------------------------------------------------ acceptance: resolve

#[test]
fn resolve_1_8_9_bukkit_uses_the_1_8_8_artifact() {
    let matrix = m();
    let r = matrix.resolve("1.8.9", &pl(&["bukkit"])).unwrap();
    assert_eq!(r.core_java_target, 8);
    let bukkit = &r.platforms["bukkit"];
    assert_eq!(bukkit.api_coordinate, "org.spigotmc:spigot-api:1.8.8-R0.1-SNAPSHOT");
    assert_eq!(bukkit.java_target, 8);
    assert_eq!(bukkit.gradle_path, "platforms:bukkit");
    assert!(r.platforms["bukkit"].capability.plugin_yml_api_version.is_none());
    assert!(!r.platforms["bukkit"].capability.libraries_enabled);
}

#[test]
fn resolve_1_8_9_paper_is_unsupported_with_two_way_lists() {
    let err = m().resolve("1.8.9", &pl(&["paper"])).unwrap_err();
    assert_eq!(err.code, "matrix.unsupported_combination");
    assert_eq!(err.exit_code(), 65);
    assert!(err.message.contains("paper × 1.8.9"), "{}", err.message);
    assert!(err.message.contains("该版本可用平台"), "{}", err.message);
    assert!(err.message.contains("bukkit"), "{}", err.message);
    assert!(err.message.contains("sponge"), "{}", err.message);
    assert!(err.message.contains("该平台可用版本"), "{}", err.message);
    assert!(err.message.contains("1.9.4"), "{}", err.message);
    assert!(err.message.contains("26.2"), "{}", err.message);
}

#[test]
fn resolve_1_21_11_paper() {
    let r = m().resolve("1.21.11", &pl(&["paper"])).unwrap();
    let paper = &r.platforms["paper"];
    assert_eq!(
        paper.api_coordinate,
        "io.papermc.paper:paper-api:1.21.11-R0.1-SNAPSHOT"
    );
    assert_eq!(paper.java_target, 21);
    assert_eq!(r.core_java_target, 21);
    assert!(!paper.capability.legacy_namespace);
    assert!(paper.capability.libraries_enabled);
    assert_eq!(paper.capability.plugin_yml_api_version.as_deref(), Some("1.13"));
}

#[test]
fn resolve_26_2_paper_bukkit_core_is_the_lowest_module_target() {
    let r = m().resolve("26.2", &pl(&["paper", "bukkit"])).unwrap();
    assert_eq!(r.platforms.len(), 2);
    assert_eq!(r.platforms["paper"].api_coordinate, "io.papermc.paper:paper-api:26.2.build.129-stable");
    assert_eq!(r.platforms["bukkit"].api_coordinate, "org.spigotmc:spigot-api:26.2-R0.1-SNAPSHOT");
    // bukkit takes the MC version's java_min for >=1.17 (spec §6.3.4/§8.1): core = 25.
    assert_eq!(r.platforms["bukkit"].java_target, 25);
    assert_eq!(r.core_java_target, 25);
}

#[test]
fn core_java_target_is_the_minimum_across_enabled_modules() {
    let matrix = m();
    assert_eq!(
        matrix.resolve("1.16.5", &pl(&["paper", "bukkit"])).unwrap().core_java_target,
        8
    );
    assert_eq!(
        matrix.resolve("1.17", &pl(&["paper", "bukkit"])).unwrap().core_java_target,
        16
    );
    assert_eq!(
        matrix.resolve("1.21.11", &pl(&["paper", "bukkit"])).unwrap().core_java_target,
        21
    );
    // velocity is always Java 25, paper 1.21.11 is 21 -> core = 21.
    assert_eq!(
        matrix.resolve("1.21.11", &pl(&["paper", "velocity"])).unwrap().core_java_target,
        21
    );
}

#[test]
fn paper_implies_bukkit_with_dedupe_and_fixed_order() {
    let matrix = m();
    assert_eq!(matrix.normalize_platforms(&pl(&["paper"])).unwrap(), pl(&["paper", PAPER_IMPLIES]));
    // dedupe + fixed priority order regardless of input order
    assert_eq!(matrix.normalize_platforms(&pl(&["bukkit", "paper"])).unwrap(), pl(&["paper", "bukkit"]));
    assert_eq!(
        matrix.normalize_platforms(&pl(&["minestom", "velocity", "paper", "paper"])).unwrap(),
        pl(&["paper", "bukkit", "velocity", "minestom"])
    );
    let r = matrix.resolve("1.21.11", &pl(&["paper"])).unwrap();
    assert!(r.platforms.contains_key(PAPER_IMPLIES));
    assert_eq!(matrix.resolve("1.21.11", &pl(&["paper"])).unwrap().platforms.len(), 2);
    // empty / unknown platform names are usage errors
    assert_eq!(matrix.normalize_platforms(&[]).unwrap_err().code, "usage.invalid");
    assert_eq!(matrix.normalize_platforms(&pl(&["nope"])).unwrap_err().code, "usage.invalid");
}

#[test]
fn unsupported_combination_lists_platform_available_versions() {
    let matrix = m();
    let err = matrix.resolve("1.8.9", &pl(&["folia"])).unwrap_err();
    assert!(err.message.contains("该版本可用平台: bukkit, sponge"), "{}", err.message);
    assert!(err.message.contains("该平台可用版本: 1.19.4"), "{}", err.message);
    assert!(err.hint.as_deref().unwrap_or_default().contains("vinoa versions --matrix"));

    // several bad platforms at once: one version list per platform
    let err = matrix.resolve("1.8.9", &pl(&["paper", "folia"])).unwrap_err();
    assert!(err.message.contains("paper, folia × 1.8.9"), "{}", err.message);
    assert!(err.message.contains("该平台可用版本（paper）: 1.9.4"), "{}", err.message);
    assert!(err.message.contains("该平台可用版本（folia）: 1.19.4"), "{}", err.message);
}

// ------------------------------------------------------------ availability lists

#[test]
fn available_platforms_includes_proxies_gated_does_not() {
    let matrix = m();
    // §11.3 sample: "1.8.9 可用平台: bukkit, sponge"
    assert_eq!(matrix.gated_platforms("1.8.9"), pl(&["bukkit", "sponge"]));
    // §6.3.5: proxies never participate in MC-version filtering
    assert_eq!(
        matrix.available_platforms("1.8.9"),
        pl(&["bukkit", "velocity", "bungeecord", "sponge"])
    );
    assert_eq!(
        matrix.available_platforms("1.21.11"),
        pl(&["paper", "bukkit", "velocity", "bungeecord", "folia", "sponge", "minestom"])
    );
    // versions with no server-side platform still allow proxies only in the picker
    assert_eq!(matrix.gated_platforms("1.16"), Vec::<String>::new());
}

#[test]
fn supported_versions_excludes_alpha_only_and_unbuildable_rows() {
    let v = m().supported_versions();
    assert_eq!(v.first().map(String::as_str), Some("1.8.9"));
    assert_eq!(v.last().map(String::as_str), Some("26.2"));
    for absent in ["26.3", "1.16", "1.9.1", "1.9.3", "1.10.1"] {
        assert!(!v.contains(&absent.to_string()), "{absent} must not be selectable");
    }
    // numeric ordering, not lexical (1.9 < 1.10 < 1.21.11 < 26.1)
    let pos = |x: &str| v.iter().position(|s| s == x).unwrap();
    assert!(pos("1.9") < pos("1.10"));
    assert!(pos("1.10.2") < pos("1.12.2"));
    assert!(pos("1.21.11") < pos("26.1"));
    assert!(pos("26.1") < pos("26.2"));
}

#[test]
fn unknown_and_nonselectable_mc_versions_are_rejected() {
    let matrix = m();
    let err = matrix.resolve("1.7.10", &pl(&["bukkit"])).unwrap_err();
    assert_eq!(err.code, "matrix.unsupported_combination");
    assert!(err.message.contains("不在版本矩阵中"), "{}", err.message);
    assert!(err.message.contains("1.8.9 … 26.2"), "{}", err.message);

    // 26.3 exists in [java] but is not selectable — and that is data-driven.
    let err = matrix.resolve("26.3", &pl(&["paper"])).unwrap_err();
    assert!(err.message.contains("selectable = false"), "{}", err.message);
    assert!(matrix.platform_plan("26.3", "paper").is_none());
    assert!(matrix.platform_plan("26.3", "velocity").is_none());
}

#[test]
fn every_whitelisted_combination_resolves() {
    let matrix = m();
    for mc in matrix.supported_versions() {
        for platform in matrix.available_platforms(&mc) {
            let plan = matrix
                .platform_plan(&mc, &platform)
                .unwrap_or_else(|| panic!("{platform} × {mc} listed but not resolvable"));
            assert!(!plan.api_coordinate.is_empty(), "{platform} × {mc} has empty coordinate");
            // Contract with the renderer: every coordinate is exactly
            // `group:artifact:version` (engine-dev splits on the last `:`).
            // A classifier (`g:a:v:jar`) must fail here loudly, not mis-parse there.
            assert_eq!(
                plan.api_coordinate.matches(':').count(),
                2,
                "{platform} × {mc}: expected group:artifact:version, got {}",
                plan.api_coordinate
            );
            assert!(
                !plan.api_coordinate.contains(char::is_whitespace),
                "{platform} × {mc}: whitespace in {}",
                plan.api_coordinate
            );
            assert!(plan.java_target >= 8, "{platform} × {mc} java {}", plan.java_target);
            assert_eq!(plan.id, platform);
        }
        matrix.resolve(&mc, &matrix.gated_platforms(&mc)).unwrap_or_else(|e| {
            panic!("resolve({mc}, all gated platforms) failed: {e}");
        });
    }
}

#[test]
fn whitelists_only_contain_selectable_versions() {
    let matrix = m();
    for p in matrix.platform_ids() {
        for mc in matrix.platform_versions(&p) {
            // proxy-style whitelists hold proxy versions, not MC versions
            if crate::matrix::mc_key(&mc).is_none() {
                continue;
            }
            assert!(
                matrix.is_selectable(&mc),
                "{p} whitelists {mc} although it is not selectable"
            );
            assert!(
                matrix.platform_plan(&mc, &p).is_some(),
                "{p} whitelists {mc} but cannot resolve it"
            );
        }
    }
    // and the reverse direction: supported_versions() covers every whitelisted MC version
    let supported = matrix.supported_versions();
    for p in ["paper", "bukkit", "folia", "sponge", "minestom"] {
        for mc in matrix.platform_versions(&p) {
            assert!(supported.contains(&mc), "{mc} reachable via {p} but not offered");
        }
    }
}

#[test]
fn whitelists_have_no_range_derivation() {
    let matrix = m();
    let paper = matrix.platform_versions("paper");
    let bukkit = matrix.platform_versions("bukkit");
    let folia = matrix.platform_versions("folia");
    let sponge = matrix.platform_versions("sponge");
    let minestom = matrix.platform_versions("minestom");
    // §6.1/§6.3: Paper has no 1.8.9 / 1.20.3 / 1.21.2 / bare 1.26.1 and starts at 1.9.4
    for absent in ["1.8.9", "1.20.3", "1.21.2", "26.1", "26.3"] {
        assert!(!paper.contains(&absent.to_string()), "paper must not list {absent}");
    }
    assert!(paper.contains(&"1.9.4".to_string()));
    assert!(paper.contains(&"1.11.2".to_string()));
    // spigot-api has no artifact for the bare 1.16 release and for 1.9.1/1.9.3/1.10.1
    for absent in ["1.9.1", "1.9.3", "1.10.1", "1.16", "26.3"] {
        assert!(!bukkit.contains(&absent.to_string()), "bukkit must not list {absent}");
    }
    assert!(bukkit.contains(&"1.8.9".to_string()));
    // Folia hard floor 1.19.4 with holes afterwards
    assert_eq!(folia.first().map(String::as_str), Some("1.19.4"));
    assert!(folia.contains(&"1.19.4".to_string()));
    assert!(!folia.contains(&"1.19.3".to_string()));
    assert!(!folia.contains(&"1.20.3".to_string()));
    // Minestom window is exactly four versions
    assert_eq!(minestom, pl(&["1.21.11", "26.1.1", "26.1.2", "26.2"]));
    // Sponge skipped whole MC lines
    for absent in ["1.13", "1.14", "1.18", "1.19", "1.19.1", "1.20.3", "1.20.5"] {
        assert!(!sponge.contains(&absent.to_string()), "sponge must not list {absent}");
    }
    assert!(sponge.contains(&"1.8.9".to_string()));
}

// ---------------------------------------------------------------- hard anchors

#[test]
fn java_target_rules_match_spec_6_3_4_and_8_1() {
    let matrix = m();
    let jt = |mc: &str, p: &str| plan(&matrix, mc, p).java_target;
    // paper / folia: the version's java_min (never the recommended value)
    assert_eq!(jt("1.9.4", "paper"), 8);
    assert_eq!(jt("1.16.5", "paper"), 8, "§6.3.2: toolchain is java_min (8), not recommended 16");
    assert_eq!(jt("1.17.1", "paper"), 16);
    assert_eq!(jt("1.20.4", "paper"), 17);
    assert_eq!(jt("1.20.5", "paper"), 21);
    assert_eq!(jt("1.21.11", "paper"), 21);
    assert_eq!(jt("26.1.1", "paper"), 25);
    assert_eq!(jt("1.19.4", "folia"), 17);
    assert_eq!(jt("26.2", "folia"), 25);
    // bukkit: 8 up to 1.16.x, then the version's java_min
    assert_eq!(jt("1.8.9", "bukkit"), 8);
    assert_eq!(jt("1.16.5", "bukkit"), 8);
    assert_eq!(jt("1.17", "bukkit"), 16);
    assert_eq!(jt("1.18", "bukkit"), 17);
    assert_eq!(jt("1.20.5", "bukkit"), 21);
    assert_eq!(jt("26.2", "bukkit"), 25);
    // proxies / minestom / sponge
    assert_eq!(jt("1.8.9", "velocity"), 25);
    assert_eq!(jt("1.8.9", "bungeecord"), 8);
    assert_eq!(jt("1.8.9", "sponge"), 8);
    assert_eq!(jt("26.2", "sponge"), 25);
    assert_eq!(jt("1.21.11", "minestom"), 25);
}

#[test]
fn coordinate_anchors_are_literal() {
    let matrix = m();
    assert_eq!(
        plan(&matrix, "1.9.4", "paper").api_coordinate,
        "com.destroystokyo.paper:paper-api:1.9.4-R0.1-SNAPSHOT"
    );
    assert_eq!(
        plan(&matrix, "1.16.5", "paper").api_coordinate,
        "com.destroystokyo.paper:paper-api:1.16.5-R0.1-SNAPSHOT"
    );
    assert_eq!(
        plan(&matrix, "1.17", "paper").api_coordinate,
        "io.papermc.paper:paper-api:1.17-R0.1-SNAPSHOT",
        "io.papermc.paper starts at 1.17 — 1.16.5 has no artifact under that group"
    );
    assert_eq!(
        plan(&matrix, "26.1.1", "paper").api_coordinate,
        "io.papermc.paper:paper-api:26.1.1.build.+"
    );
    assert_eq!(
        plan(&matrix, "26.2", "paper").api_coordinate,
        "io.papermc.paper:paper-api:26.2.build.129-stable",
        "pinned wins over the dynamic coordinate"
    );
    assert_eq!(plan(&matrix, "1.8.9", "bungeecord").api_coordinate, "net.md-5:bungeecord-api:1.21-R0.4");
    assert_eq!(plan(&matrix, "1.8.9", "velocity").api_coordinate, "com.velocitypowered:velocity-api:4.2.0");
    assert_eq!(
        plan(&matrix, "1.21.11", "minestom").api_coordinate,
        "net.minestom:minestom:2026.05.11-1.21.11"
    );
    assert_eq!(
        plan(&matrix, "1.8.9", "sponge").api_coordinate,
        "org.spongepowered:spongeapi:4.2.0-SNAPSHOT"
    );
    assert_eq!(
        plan(&matrix, "26.2", "sponge").api_coordinate,
        "org.spongepowered:spongeapi:20.0.0-SNAPSHOT"
    );
    // folia reuses the paper-api artifact for the same version
    assert_eq!(
        plan(&matrix, "1.21.11", "folia").api_coordinate,
        plan(&matrix, "1.21.11", "paper").api_coordinate
    );
}

#[test]
fn capability_bits_follow_the_spec() {
    let matrix = m();
    let cap = |mc: &str, p: &str| plan(&matrix, mc, p).capability;
    // api-version line existing from 1.13 (§7.1 / [LEG §5.1])
    assert_eq!(cap("1.12.2", "paper").plugin_yml_api_version, None);
    assert_eq!(cap("1.13", "paper").plugin_yml_api_version.as_deref(), Some("1.13"));
    // libraries: from 1.16.5 (§7.5 cap_libraries)
    assert!(!cap("1.16.4", "paper").libraries_enabled);
    assert!(cap("1.16.5", "paper").libraries_enabled);
    // legacy namespace only for paper <= 1.16.5 (§7.1)
    assert!(cap("1.16.5", "paper").legacy_namespace);
    assert!(!cap("1.17", "paper").legacy_namespace);
    assert!(!cap("1.16.5", "bukkit").legacy_namespace);
    // run tasks
    assert!(cap("1.21.11", "paper").run_task);
    assert!(cap("26.2", "velocity").run_task);
    assert!(!cap("1.8.9", "bukkit").run_task, "1.8.9 has no Paper -> runTask = none");
    // v1 has no NMS flag: paperweight/reobf stay off (§7.1 default false)
    assert!(!cap("26.2", "paper").paperweight);
    assert!(!cap("26.2", "paper").reobf);
    // B-level platforms are flagged experimental (§12.1)
    assert!(plan(&matrix, "1.19.4", "folia").experimental);
    assert!(plan(&matrix, "26.2", "sponge").experimental);
    assert!(plan(&matrix, "26.2", "minestom").experimental);
    assert!(!plan(&matrix, "1.21.11", "paper").experimental);
    assert!(!plan(&matrix, "1.8.9", "bukkit").experimental);
    assert!(!plan(&matrix, "1.8.9", "velocity").experimental);
}

#[test]
fn matrix_carries_the_canonical_tooling_keys() {
    let matrix = m();
    assert_eq!(matrix.gradle_version(), "9.8.0");
    assert_eq!(matrix.generated_at(), "2026-09-25");
    assert_eq!(matrix.java_requirement("1.16.5"), Some((8, 16)));
    assert_eq!(matrix.java_requirement("26.3"), Some((25, 25)));
    let q = matrix.quality();
    for key in [
        "checkstyle",
        "checkstyle_legacy",
        "junit",
        "junit_legacy",
        "spotbugs",
        "spotbugs_legacy",
    ] {
        assert!(q.contains_key(key), "quality.{key} missing");
    }
    let gp = matrix.gradle_plugins();
    for key in ["shadow", "run_paper", "run_velocity", "paperweight", "spotbugs_plugin"] {
        assert!(gp.contains_key(key), "gradle_plugins.{key} missing");
    }
    assert_eq!(matrix.gradle_plugin("shadow").unwrap().gradle_min.as_deref(), Some("9.2.0"));
    assert_eq!(matrix.gradle_plugin("run_paper").unwrap().gradle_min.as_deref(), Some("9.7.0"));
    assert_eq!(matrix.gradle_plugin("paperweight").unwrap().java, 21);
    // Gradle *plugin* version is distinct from the analysis *tool* version
    let sb = matrix.gradle_plugin("spotbugs_plugin").unwrap();
    assert_eq!(sb.version, "6.5.11");
    assert_eq!(sb.java, 11);
    assert_eq!(sb.gradle_min, None, "com.github.spotbugs declares no api-version floor");
    assert_eq!(gp["spotbugs_plugin"], "6.5.11");
    assert_eq!(matrix.quality()["spotbugs"], "4.10.4");
    // thirdparty canonical keys, gui deliberately absent (pure Bukkit Inventory)
    let tp = matrix.thirdparty();
    assert_eq!(
        tp.keys().cloned().collect::<Vec<_>>(),
        vec!["bstats", "placeholderapi", "sqlite_jdbc"]
    );
    assert!(!tp.contains_key("gui"));
    assert_eq!(
        matrix.thirdparty_coordinates()["placeholderapi"],
        "me.clip:placeholderapi:2.11.6"
    );
    assert_eq!(tp["sqlite_jdbc"].scope, ThirdPartyScope::Core);
    assert_eq!(tp["bstats"].scope, ThirdPartyScope::Platform);
}

#[test]
fn resolved_carries_quality_gradle_and_thirdparty_maps() {
    let r = m().resolve("1.21.11", &pl(&["paper"])).unwrap();
    assert_eq!(r.gradle_version, "9.8.0");
    assert_eq!(r.quality["checkstyle"], "10.24.0");
    assert_eq!(r.quality["checkstyle_legacy"], "9.3");
    assert_eq!(r.gradle_plugins["run_paper"], "3.1.0");
    assert_eq!(r.gradle_plugins["spotbugs_plugin"], "6.5.11");
    assert_eq!(r.thirdparty.len(), 3);
}

#[test]
fn repositories_are_data_driven() {
    let matrix = m();
    let repos = matrix.repositories();
    assert_eq!(repos["central"], "https://repo1.maven.org/maven2/");
    assert_eq!(repos["papermc"], "https://repo.papermc.io/repository/maven-public/");
    // PlaceholderAPI is NOT on Central/papermc (both 404) — only extendedclip serves it
    assert_eq!(
        matrix.thirdparty_repos("placeholderapi"),
        vec!["https://repo.extendedclip.com/releases/"]
    );
    assert_eq!(matrix.thirdparty_repos("bstats"), vec!["https://repo1.maven.org/maven2/"]);
    assert!(matrix.thirdparty_repos("nope").is_empty());
    // platforms: papermc covers the spigot/papermc/bungee family, Central Minestom,
    // spongepowered the SpongeAPI lines (incl. -SNAPSHOT)
    for p in ["paper", "bukkit", "folia", "velocity", "bungeecord"] {
        assert!(matrix.platform_repo(p).unwrap().contains("repo.papermc.io"), "{p}");
    }
    assert!(matrix.platform_repo("minestom").unwrap().contains("repo1.maven.org"));
    assert!(matrix.platform_repo("sponge").unwrap().contains("repo.spongepowered.org"));
    // every platform declares one, and a corrupt repo id is rejected at load time
    for p in matrix.platform_ids() {
        assert!(matrix.platform_repo(&p).is_some(), "{p} has no repo");
    }
    let bad = synthetic_matrix(8).replace(
        "repo = \"central\"",
        "repo = \"nowhere\"",
    );
    assert_eq!(Matrix::load_from_str(&bad).unwrap_err().code, "config.invalid");
}

// ------------------------------------------------------- third-party Java floors

#[test]
fn thirdparty_java_floors_are_enforced() {
    let matrix = m();
    // all three published coordinates are Java 8 bytecode (class major 52)
    for (key, lib) in matrix.thirdparty() {
        assert_eq!(lib.java_min, 8, "{key} floor changed — re-check the jar");
    }
    // therefore a 1.8.9 bukkit project may use all of them
    let features = pl(&["sqlite", "bstats", "placeholderapi", "gui", "update-check"]);
    assert_eq!(
        thirdparty_keys_for_features(&features),
        pl(&["sqlite_jdbc", "bstats", "placeholderapi"])
    );
    matrix
        .resolve_with_features("1.8.9", &pl(&["bukkit"]), &features)
        .expect("third-party libs are Java 8 compatible");

    // a library that needs more than the module provides must hard-error
    let synthetic = Matrix::load_from_str(&synthetic_matrix(17)).unwrap();
    let err = synthetic.check_thirdparty(&pl(&["heavy"]), 8, 8).unwrap_err();
    assert_eq!(err.code, "matrix.unsupported_combination");
    assert!(err.message.contains("com.example:heavy:1.0"), "{}", err.message);
    assert!(err.message.contains("要求 Java 17"), "{}", err.message);
    assert!(err.message.contains("core 模块目标为 Java 8"), "{}", err.message);
    synthetic.check_thirdparty(&pl(&["heavy"]), 17, 17).unwrap();
    // unknown keys carry no dependency and are ignored
    synthetic.check_thirdparty(&pl(&["nothing"]), 8, 8).unwrap();
}

// ------------------------------------------------------------ load / validate

#[test]
fn load_from_str_rejects_a_corrupt_matrix() {
    let schema_err = Matrix::load_from_str(&synthetic_matrix(17).replace("schema = 1", "schema = 2"))
        .unwrap_err();
    assert_eq!(schema_err.code, "config.invalid");
    assert!(schema_err.message.contains("schema"), "{}", schema_err.message);

    // a whitelisted version without a literal coordinate is corruption, not a range
    let missing = synthetic_matrix(17).replace(
        "\"1.8.9\" = \"org.spigotmc:spigot-api:1.8.8-R0.1-SNAPSHOT\"",
        "",
    );
    let err = Matrix::load_from_str(&missing).unwrap_err();
    assert_eq!(err.code, "config.invalid");
    assert!(err.message.contains("没有对应坐标"), "{}", err.message);

    assert_eq!(Matrix::load_from_str("schema = 1\n").unwrap_err().code, "config.invalid");
}

#[test]
fn builtin_matrix_has_66_java_rows_and_seven_platforms() {
    let matrix = m();
    assert_eq!(matrix.schema(), 1);
    assert_eq!(matrix.all_java_versions().len(), 66);
    assert_eq!(matrix.all_java_versions().first().map(String::as_str), Some("1.8.9"));
    assert_eq!(matrix.all_java_versions().last().map(String::as_str), Some("26.3"));
    assert_eq!(
        matrix.platform_ids(),
        vec!["bukkit", "bungeecord", "folia", "minestom", "paper", "sponge", "velocity"]
    );
    // the 66-row table is ascending in MC order
    let v = matrix.all_java_versions();
    for pair in v.windows(2) {
        assert!(
            super::query::cmp_mc(&pair[0], &pair[1]) == std::cmp::Ordering::Less,
            "{} !< {}",
            pair[0],
            pair[1]
        );
    }
}

// ---------------------------------------------------------------- refresh (offline)

const PAPER_PROJECT_JSON: &str = r#"{
  "project": {"id": "paper", "name": "Paper"},
  "versions": {
    "26.3": ["26.3", "26.3-rc-3"],
    "26.2": ["26.2", "26.2-rc-2"],
    "1.8": ["1.8.8"]
  }
}"#;

const BUILD_JSON: &str = r#"{
  "id": 232,
  "time": "2025-06-09T10:18:55.778Z",
  "channel": "STABLE",
  "commits": [{"sha": "12d8fe0beb21c1a1d9b093fb411884367cce9e7e",
               "time": "2025-06-09T09:57:21Z",
               "message": "Fix infinite loop"}],
  "downloads": {"server:default": {
      "name": "paper-1.21.4-232.jar",
      "checksums": {"sha256": "5ee4f542f628a14c644410b08c94ea42e772ef4d29fe92973636b6813d4eaffc"},
      "size": 51437498,
      "url": "https://fill-data.papermc.io/v1/objects/5ee4/paper-1.21.4-232.jar"}}
}"#;

fn build(id: i64, channel: &str) -> Build {
    Build {
        id,
        time: String::new(),
        channel: channel.to_string(),
        downloads: std::collections::BTreeMap::new(),
    }
}

#[test]
fn refresh_parses_the_documented_fill_schema() {
    let project: ProjectVersions = serde_json::from_str(PAPER_PROJECT_JSON).unwrap();
    assert_eq!(project.project.id, "paper");
    assert_eq!(project.versions["26.3"], vec!["26.3", "26.3-rc-3"]);

    let b: Build = serde_json::from_str(BUILD_JSON).unwrap();
    assert_eq!(b.id, 232);
    assert_eq!(b.channel, "STABLE");
    let dl = b.server_download().expect("server:default");
    assert_eq!(dl.name, "paper-1.21.4-232.jar");
    assert_eq!(b.sha256(), Some("5ee4f542f628a14c644410b08c94ea42e772ef4d29fe92973636b6813d4eaffc"));
    assert_eq!(dl.size, Some(51437498));
}

#[test]
fn channel_filtering_is_client_side() {
    // newest-first; the newest build is ALPHA, so /builds/latest must not be trusted
    let builds = vec![build(41, "ALPHA"), build(40, "BETA"), build(39, "STABLE"), build(38, "STABLE")];
    assert_eq!(pick_stable(&builds).map(|b| b.id), Some(39));
    assert!(pick_stable(&[build(1, "ALPHA")]).is_none());
    assert!(pick_stable(&[]).is_none());
}

#[test]
fn fill_urls_and_operational_rules() {
    assert_eq!(builds_url(API_BASE, "paper", "1.21.4", None), format!("{API_BASE}/projects/paper/versions/1.21.4/builds"));
    assert_eq!(
        builds_url(API_BASE, "paper", "1.21.4", Some("STABLE")),
        format!("{API_BASE}/projects/paper/versions/1.21.4/builds?channel=STABLE")
    );
    // /builds/latest ignores ?channel= -> the helper never emits one
    assert_eq!(
        latest_url(API_BASE, "paper", "26.2"),
        format!("{API_BASE}/projects/paper/versions/26.2/builds/latest")
    );
    assert!(!latest_url(API_BASE, "paper", "26.2").contains("channel"));
    // api.papermc.io/v2 is dead
    assert!(!API_BASE.contains("v2"));
    assert!(!latest_url(API_BASE, "paper", "1.21.4").contains("v2"));
    // non-generic User-Agent with a contact URL
    assert!(USER_AGENT.contains("vinoa"));
    assert!(USER_AGENT.contains("http://") || USER_AGENT.contains("https://"));
    for generic in ["curl", "wget", "python", "Mozilla"] {
        assert!(!USER_AGENT.contains(generic));
    }
    assert!(is_release_version("1.21.11"));
    assert!(!is_release_version("26.3-rc-3"));
    assert!(!is_release_version("1.21.11-pre3"));
    assert_eq!(super::refresh::PROJECTS, ["paper", "folia", "velocity"]);
}

#[test]
fn version_ordering_is_a_total_order() {
    use super::query::{cmp_mc, mc_key};
    assert!(mc_key("1.8.9").unwrap() < mc_key("1.10").unwrap());
    assert!(mc_key("1.21.11").unwrap() < mc_key("26.1").unwrap());
    assert!(mc_key("26.1").unwrap() < mc_key("26.1.1").unwrap());
    assert!(mc_key("banana").is_none());
    // Fill hands back pre-release ids too, so sorting must stay consistent for them
    let mut v = pl(&["1.21.11-rc3", "1.21.11", "26.3-rc-3", "26.3", "26.2", "1.8.8", "1.9", "1.10", "1.7.10"]);
    v.sort_by(|a, b| cmp_mc(a, b));
    assert_eq!(
        v,
        pl(&["1.7.10", "1.8.8", "1.9", "1.10", "1.21.11", "1.21.11-rc3", "26.2", "26.3", "26.3-rc-3"])
    );
    let mut reversed = v.clone();
    reversed.reverse();
    reversed.sort_by(|a, b| cmp_mc(a, b));
    assert_eq!(reversed, v, "comparator must be consistent both ways");
}

// ------------------------------------------------------- live Fill v3 (opt-in)

/// Live smoke test — not part of `cargo test` (network).
/// Run: `cargo test --lib matrix::tests::live_fill -- --ignored --nocapture`
#[test]
#[ignore = "network: hits fill.papermc.io/v3"]
fn live_fill_client_smoke() {
    use super::refresh::FillClient;
    let client = FillClient::new();
    let versions = client.project_versions("paper").expect("GET /projects/paper");
    assert!(versions.contains(&"26.2".to_string()), "paper 26.2 missing: {versions:?}");
    // 26.3 exists upstream but only as ALPHA ([VM §1.4]) -> no STABLE build
    assert!(client.first_stable("paper", "26.3").unwrap().is_none());
    let stable = client
        .first_stable("paper", "26.2")
        .unwrap()
        .expect("paper 26.2 has STABLE builds");
    assert_eq!(stable.channel, "STABLE");
    assert!(stable.server_download().is_some());
    // 1.8.9 has no Paper build at all -> Fill answers `version_not_found`
    let versions = client.project_versions("paper").unwrap();
    assert!(versions.contains(&"1.8.8".to_string()));
    assert!(!versions.contains(&"1.8.9".to_string()), "Paper never built 1.8.9");
}

// ------------------------------------------------------------- test fixtures

/// Minimal but structurally complete matrix used for corruption / Java-floor tests.
fn synthetic_matrix(heavy_java_min: u8) -> String {
    format!(
        r#"
schema = 1
generated_at = "2026-09-25"

[java]
"1.8.9" = {{ min = 8, recommended = 8 }}

[quality]
checkstyle = "10.24.0"

[gradle]
version = "9.8.0"
run_on_jvm = "17-27"
java25_toolchain_min = "9.1.0"

[gradle.shadow]
version = "9.6.1"
gradle_min = "9.2.0"
java = 17

[gradle.run_paper]
version = "3.1.0"
gradle_min = "9.7.0"
java = 17

[gradle.run_velocity]
version = "3.1.0"
gradle_min = "9.7.0"
java = 17

[gradle.paperweight]
version = "2.0.0-beta.24"
gradle_min = "9.7.1"
java = 21

[gradle.spotbugs_plugin]
version = "6.5.11"
java = 11

[repositories]
central = "https://repo1.maven.org/maven2/"

[thirdparty]
heavy = {{ coordinate = "com.example:heavy:1.0", java_min = {heavy_java_min}, scope = "core", repos = ["central"] }}

[platform.bukkit]
api = "org.spigotmc:spigot-api"
repo = "central"
java_legacy_until = "1.16.5"
java_legacy = 8
versions = ["1.8.9"]

[platform.bukkit.coordinate_map]
"1.8.9" = "org.spigotmc:spigot-api:1.8.8-R0.1-SNAPSHOT"
"#
    )
}
