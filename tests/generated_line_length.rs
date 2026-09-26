//! Regression guard: generated source must survive the generated checkstyle config.
//!
//! The built-in template set ships a checkstyle `LineLength` limit of 120, and the
//! generated sources interpolate the user's project/package names. A hand-wrapped
//! template line therefore only *looks* safe: it can overflow as soon as the name is
//! a few characters longer than the one the author tested with. That is exactly how
//! `AllfeatIntegrationTest.java` failed at 121 characters while `my-plugin` passed.
//!
//! This test renders with a deliberately long identity and asserts that nothing the
//! generated build would check exceeds the limit.
use vinoa::cli::InitArgs;
use vinoa::matrix::Matrix;
use vinoa::{init, template};

const LINE_LIMIT: usize = 120;

fn spec(args: &InitArgs) -> vinoa::types::ProjectSpec {
    let matrix = Matrix::builtin().expect("builtin matrix must load");
    init::spec_from_args(args, &matrix).expect("spec must build from args")
}

fn plan_for(name: &str, package: &str, platforms: &[&str], features: &[&str]) -> vinoa::types::Plan {
    let matrix = Matrix::builtin().expect("builtin matrix must load");
    let args = InitArgs {
        name: Some(name.to_string()),
        package: Some(package.to_string()),
        mc: Some("1.21.11".to_string()),
        platforms: platforms.iter().map(|p| p.to_string()).collect(),
        features: features.iter().map(|f| f.to_string()).collect(),
        bstats_id: Some("12345".to_string()),
        ..InitArgs::default()
    };
    let spec = spec(&args);
    let resolved = vinoa::matrix::resolve(&matrix, &spec.mc_version, &spec.platforms)
        .expect("combination must resolve");
    let manifest = template::load_manifest().expect("builtin manifest must load");
    template::plan(&manifest, &spec, &resolved).expect("plan must build")
}

/// Files the generated build actually runs checkstyle over.
fn is_checked(path: &str) -> bool {
    path.ends_with(".java") || path.ends_with(".kts") || path.ends_with(".gradle")
}

fn overlong_lines(plan: &vinoa::types::Plan) -> Vec<String> {
    let mut out = Vec::new();
    for file in &plan.files {
        if !is_checked(&file.path) {
            continue;
        }
        let Ok(text) = std::str::from_utf8(&file.content) else {
            continue;
        };
        for (index, line) in text.lines().enumerate() {
            let width = line.chars().count();
            if width > LINE_LIMIT {
                out.push(format!("{}:{} ({width} chars)", file.path, index + 1));
            }
        }
    }
    out
}

#[test]
fn long_project_identity_keeps_every_checked_line_within_the_limit() {
    // Everything on at once, across every platform that generates sources, with an
    // identity long enough to push interpolated lines past the limit if they are not
    // wrapped in the template.
    let plan = plan_for(
        "a-fairly-long-plugin-name-for-line-length",
        "com.example.averylongpackagename",
        &["paper", "folia", "bukkit", "velocity", "bungeecord"],
        &[
            "sqlite",
            "bstats",
            "gui",
            "update-check",
            "placeholderapi",
            "spotbugs",
            "coverage",
            "release-ci",
        ],
    );
    let bad = overlong_lines(&plan);
    assert!(
        bad.is_empty(),
        "generated sources exceed the {LINE_LIMIT}-char checkstyle LineLength limit; \
         wrap the offending template line(s):\n{}",
        bad.join("\n")
    );
}

#[test]
fn short_project_identity_also_stays_within_the_limit() {
    let plan = plan_for("t", "com.example.t", &["paper", "bukkit"], &[]);
    let bad = overlong_lines(&plan);
    assert!(bad.is_empty(), "generated sources exceed the limit:\n{}", bad.join("\n"));
}
