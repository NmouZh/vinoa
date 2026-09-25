//! Init orchestrator: phases 0–8 (docs/spec/vinoa-cli.md §5). OWNER: lead.
//!
//! Single code path: the plan is computed once (phase 4) and executed (phases 6–7).
//! `--dry-run` only stops between those two.
use crate::cli::InitArgs;
use crate::error::{self, Error, Result, EXIT_OK, EXIT_TEMPFAIL, EXIT_VERIFY_FAILED};
use crate::matrix::{self, Matrix};
use crate::types::{
    normalize_platforms, slug, to_pascal, ArtifactLanguage, FeatureSet, MetadataFormat, Plan,
    PlannedAction, ProjectSpec, QualitySet, UiLanguage, PLATFORMS,
};
use crate::{fsx, precheck, report, template, verify, wizard};
use std::path::PathBuf;
use std::process::ExitCode;

pub fn run(args: InitArgs) -> Result<ExitCode> {
    // ── phase 0: context ────────────────────────────────────────────────────
    let matrix = Matrix::builtin()?;
    // `cli::parse` has already merged `-c/--config` into `args`, so a config file
    // means "every answer is supplied" and the wizard must not open.
    let interactive = is_tty() && !args.yes && args.config.is_none();

    // ── phase 1: inputs (flags / config file / wizard) ──────────────────────
    let mut spec = if interactive {
        wizard::run(args.clone(), &matrix)?
    } else {
        spec_from_args(&args, &matrix)?
    };

    // ── phase 2: validate ───────────────────────────────────────────────────
    validate(&spec)?;

    // `--print-config` exports exactly what `-c/--config` consumes, then stops.
    if args.print_config {
        print!("{}", crate::cli::print_config_toml(&spec)?);
        return Ok(ExitCode::from(EXIT_OK));
    }

    // ── phase 3: matrix resolution ──────────────────────────────────────────
    let resolved = matrix::resolve(&matrix, &spec.mc_version, &spec.platforms)?;

    // ── phase 3.5: environment precheck ─────────────────────────────────────
    let java = precheck::check_java(&resolved);
    print_precheck(&java, spec.ui_language);
    if !java.all_satisfied() {
        let missing: Vec<String> = java
            .missing()
            .iter()
            .map(|c| format!("Java {} ({})", c.required, c.platform))
            .collect();
        if interactive {
            let ask = format!("缺少 {} —— 是否让 Gradle 首次构建时自动下载？", missing.join("、"));
            spec.download_jdk = ask_confirm(&ask)?;
        } else if spec.download_jdk {
            eprintln!("  → 已启用 Gradle toolchain 自动下载（download_jdk）");
        } else {
            eprintln!(
                "  ⚠ 缺少 {}；构建会失败。加 --download-jdk 让 Gradle 自动下载，或先自行安装。",
                missing.join("、")
            );
        }
    }

    // ── phase 4: plan (the single source of truth) ──────────────────────────
    let manifest = match args.template.as_deref() {
        Some(path) if !path.contains("://") => {
            template::load_manifest_from(std::path::Path::new(path))?
        }
        Some(url) => {
            return Err(error::usage(format!(
                "外部模板 {url} 暂不支持：v1 只接受本地路径（--template <path>）"
            )))
        }
        None => template::load_manifest()?,
    };

    // Guard: every requested capability must actually be implemented by the
    // template set that is about to render. Silently generating nothing is worse
    // than failing loudly — the user would believe the feature is there.
    let requested = requested_features(&spec);
    let implemented = manifest.implemented_features();
    let unimplemented: Vec<&str> = requested
        .iter()
        .copied()
        .filter(|token| !implemented.iter().any(|done| done == token))
        .collect();
    if !unimplemented.is_empty() {
        let have = if implemented.is_empty() {
            "(无)".to_string()
        } else {
            implemented.join(", ")
        };
        return Err(Error::new(
            "template.feature_unimplemented",
            error::EXIT_DATA,
            format!("请求的功能当前模板集尚未实现: {}", unimplemented.join(", ")),
        )
        .with_hint(format!("该模板集已实现: {have}")));
    }

    let mut plan: Plan = template::plan(&manifest, &spec, &resolved)?;
    if args.verify && !plan.actions.contains(&PlannedAction::Verify) {
        plan.actions.push(PlannedAction::Verify);
    }

    report::plan_human(&plan);

    let precheck_json = java.to_json(spec.download_jdk);
    let template_source = if args.template.is_some() { "external" } else { "builtin" };

    // `--dry-run` prints the plan and stops — nothing is written.
    if args.dry_run {
        if args.json {
            let extra = serde_json::json!({
                "spec": &spec,
                "precheck": &precheck_json,
                "matrix": &resolved,
                "template_source": template_source,
                "dry_run": true,
            });
            report::plan_json(&plan, &extra);
        }
        return Ok(ExitCode::from(EXIT_OK));
    }

    if interactive && !ask_confirm("确认创建？")? {
        eprintln!("已取消，未写入任何文件。");
        if args.json {
            report::emit_json(&serde_json::json!({
                "schema": "vinoa.init/v1",
                "vinoa": env!("CARGO_PKG_VERSION"),
                "ok": false,
                "command": "init",
                "status": "cancelled",
                "exit_code": EXIT_OK,
                "errors": [],
            }));
        }
        return Ok(ExitCode::from(EXIT_OK));
    }

    // ── phase 6: atomic write ───────────────────────────────────────────────
    fsx::ensure_target_writable(&plan.root)?;
    fsx::write_atomic(&plan)?;

    // ── phase 7: post actions ───────────────────────────────────────────────
    let mut verify_failed = false;
    let mut verify_json = verify::VerifyOutcome::not_run_json();
    let mut git_json = serde_json::json!({ "initialized": false });
    for action in &plan.actions {
        match action {
            PlannedAction::GitInit { branch, message } => {
                fsx::git::init_and_commit(&plan.root, branch, message)?;
                git_json = serde_json::json!({
                    "initialized": true,
                    "branch": branch,
                    "message": message,
                });
            }
            PlannedAction::Verify => {
                let options = verify::VerifyOptions {
                    online: args.verify_online,
                    tail: args.verify_tail.unwrap_or(verify::DEFAULT_TAIL),
                    timeout_s: verify::DEFAULT_TIMEOUT_S,
                };
                let outcome = verify::run_with(&plan, &options)?;
                verify_json = outcome.to_json();
                if outcome.ok {
                    eprintln!("  ✓ 验证通过（构建日志: {}）", outcome.log_path);
                } else {
                    verify_failed = true;
                    eprintln!(
                        "  ✗ 生成成功，但验证失败（{}）；工程保留在 {}",
                        outcome.reason.as_deref().unwrap_or("build_failed"),
                        plan.root.display()
                    );
                }
            }
        }
    }

    // ── phase 8: report — exactly one JSON document, after everything settled ─
    print_done(&plan);
    if args.json {
        let status = if verify_failed { "verify_failed" } else { "ok" };
        let exit_code = if verify_failed { EXIT_VERIFY_FAILED } else { EXIT_OK };
        let errors = if verify_failed {
            serde_json::json!([{
                "code": "verify.build_failed",
                "phase": "verify",
                "severity": "error",
                "message": "生成的工程构建失败；工程已保留，可修复后重试",
            }])
        } else {
            serde_json::json!([])
        };
        let extra = serde_json::json!({
            "spec": &spec,
            "precheck": &precheck_json,
            "matrix": &resolved,
            "template_source": template_source,
            "git": &git_json,
            "verify": &verify_json,
            "ok": !verify_failed,
            "status": status,
            "exit_code": exit_code,
            "errors": errors,
        });
        report::plan_json(&plan, &extra);
    }
    if verify_failed {
        return Ok(ExitCode::from(EXIT_VERIFY_FAILED));
    }
    Ok(ExitCode::from(EXIT_OK))
}

/// Non-interactive: build the spec from flags, filling documented defaults.
pub fn spec_from_args(args: &InitArgs, matrix: &Matrix) -> Result<ProjectSpec> {
    let project_name = args
        .name
        .clone()
        .ok_or_else(|| error::usage("缺少工程名：用 `vinoa init <名称>` 给出"))?;
    let mc_version = args.mc.clone().ok_or_else(|| {
        error::usage("缺少 --mc <版本>").with_hint(format!(
            "可用示例: {}",
            matrix.supported_versions().join(" / ")
        ))
    })?;
    let platforms = normalize_platforms(&args.platforms);
    if platforms.is_empty() {
        return Err(error::usage(format!(
            "缺少 --platform；可选: {}",
            PLATFORMS.join(" / ")
        )));
    }

    Ok(ProjectSpec {
        target_dir: PathBuf::from(
            args.output
                .clone()
                .unwrap_or_else(|| project_name.clone()),
        ),
        plugin_name: to_pascal(&project_name),
        package_name: args
            .package
            .clone()
            .unwrap_or_else(|| format!("com.example.{}", slug(&project_name))),
        project_name,
        author: args.author.clone(),
        description: args.description.clone(),
        mc_version,
        platforms,
        metadata: parse_metadata(args.metadata.as_deref())?,
        language: parse_language(args.language.as_deref()),
        ui_language: parse_ui_language(args.ui_language.as_deref()),
        license: args
            .license
            .clone()
            .unwrap_or_else(|| "Apache-2.0".to_string()),
        features: parse_features(&args.features),
        quality: QualitySet {
            checkstyle: !args.no_quality,
            unit_tests: !args.no_quality,
            ci: !args.no_quality,
        },
        example: !args.no_example,
        permissions: !args.no_permissions,
        website: args.website.clone(),
        git: !args.no_git,
        download_jdk: args.download_jdk.unwrap_or(false),
        bstats_id: args.bstats_id.clone(),
    })
}

fn validate(spec: &ProjectSpec) -> Result<()> {
    if spec.project_name.trim().is_empty() {
        return Err(error::usage("工程名不能为空"));
    }
    if spec.package_name.split('.').any(|seg| seg.is_empty()) {
        return Err(error::usage(format!("包名不合法: {}", spec.package_name)));
    }
    if spec.features.bstats && spec.bstats_id.is_none() {
        return Err(error::usage(
            "勾选了 bStats 必须提供数字 plugin id（--bstats-id）；我们不生成假 id",
        )
        .with_hint("plugin id 在 bstats.org 新建插件后获得"));
    }
    Ok(())
}

pub fn parse_features(list: &[String]) -> FeatureSet {
    let mut f = FeatureSet::default();
    for raw in list {
        for item in raw.split(',') {
            match item.trim().to_ascii_lowercase().as_str() {
                "sqlite" => f.sqlite = true,
                "bstats" => f.bstats = true,
                "update-check" => f.update_check = true,
                "placeholderapi" => f.placeholderapi = true,
                "gui" => f.gui = true,
                "spotbugs" => f.spotbugs = true,
                "coverage" => f.coverage = true,
                "release-ci" => f.release_ci = true,
                _ => {}
            }
        }
    }
    f
}

fn parse_metadata(value: Option<&str>) -> Result<MetadataFormat> {
    match value.map(|v| v.to_ascii_lowercase()).as_deref() {
        None | Some("plugin.yml") | Some("plugin-yml") => Ok(MetadataFormat::PluginYml),
        Some("paper-plugin.yml") | Some("paper-plugin-yml") | Some("paper-plugin") => {
            Ok(MetadataFormat::PaperPluginYml)
        }
        Some(other) => Err(error::usage(format!(
            "未知元数据格式: {other}（可用: plugin.yml | paper-plugin.yml）"
        ))),
    }
}

fn parse_language(value: Option<&str>) -> ArtifactLanguage {
    match value.map(|v| v.to_ascii_lowercase()).as_deref() {
        Some("en") => ArtifactLanguage::En,
        Some("both") => ArtifactLanguage::Both,
        _ => ArtifactLanguage::Zh,
    }
}

fn parse_ui_language(value: Option<&str>) -> UiLanguage {
    if let Some(v) = value {
        return match v.to_ascii_lowercase().as_str() {
            "zh" => UiLanguage::Zh,
            _ => UiLanguage::En,
        };
    }
    let lang = std::env::var("LANG").unwrap_or_default();
    if lang.to_ascii_lowercase().starts_with("zh") {
        UiLanguage::Zh
    } else {
        UiLanguage::En
    }
}

fn is_tty() -> bool {
    use std::io::IsTerminal;
    std::io::stdin().is_terminal() && std::io::stdout().is_terminal()
}

fn ask_confirm(prompt: &str) -> Result<bool> {
    inquire::Confirm::new(prompt)
        .with_default(true)
        .prompt()
        .map_err(|e| Error::new("input.cancelled", EXIT_TEMPFAIL, e.to_string()))
}

fn print_precheck(java: &precheck::JavaReport, ui: UiLanguage) {
    let header = match ui {
        UiLanguage::Zh => "本机环境预检:",
        UiLanguage::En => "Environment precheck:",
    };
    eprintln!("{header}");
    for check in &java.checks {
        match &check.found_path {
            Some(path) => eprintln!(
                "  Java {}  ✓ {}   （{} 模块需要）",
                check.required, path, check.platform
            ),
            None => eprintln!(
                "  Java {}  ✗ 未检测到   （{} 模块需要）",
                check.required, check.platform
            ),
        }
    }
}

fn print_done(plan: &Plan) {
    eprintln!("✓ 完成: {}", plan.root.display());
    eprintln!();
    eprintln!("下一步:");
    eprintln!("  cd {}", plan.root.display());
    eprintln!("  ./gradlew build        # 构建插件 jar");
}

/// Capability tokens the user asked for. Must match the manifest's
/// `[features] implemented` vocabulary (CLI words, e.g. `update-check`).
pub fn requested_features(spec: &ProjectSpec) -> Vec<&'static str> {
    let mut out = Vec::new();
    let f = &spec.features;
    if f.sqlite {
        out.push("sqlite");
    }
    if f.bstats {
        out.push("bstats");
    }
    if f.update_check {
        out.push("update-check");
    }
    if f.placeholderapi {
        out.push("placeholderapi");
    }
    if f.gui {
        out.push("gui");
    }
    if f.spotbugs {
        out.push("spotbugs");
    }
    if f.coverage {
        out.push("coverage");
    }
    if f.release_ci {
        out.push("release-ci");
    }
    if spec.example {
        out.push("example");
    }
    if spec.permissions {
        out.push("permissions");
    }
    if spec.quality.checkstyle || spec.quality.unit_tests || spec.quality.ci {
        out.push("quality");
    }
    if spec.git {
        out.push("git");
    }
    out
}
