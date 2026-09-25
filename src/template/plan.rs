//! Plan construction: manifest + spec + matrix -> in-memory file set.
//! OWNER: engine-dev.
//!
//! This is the *only* code path that produces generated files: `--dry-run`
//! prints the plan, execution only executes it (§7.10). Everything is
//! deterministic — same `(spec, resolved)` in, byte-identical file set out
//! (ruling C2), so no timestamps, years, machine paths or random data.
use crate::error::{Error, Result, EXIT_CONFIG};
use crate::template::manifest::{self, Entry, Render, TemplateManifest};
use crate::template::render::{self, Ctx, Renderer};
use crate::template::vars;
use crate::types::{Plan, PlannedAction, PlannedFile, PlatformPlan, ProjectSpec, Resolved};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

/// Path segments may only use these four variables (spec §7.4).
pub const PATH_VARS: [&str; 4] = ["projectName", "pluginName", "packagePath", "platform"];

/// Template-set default identity strings that must never leak into output
/// (assertion A2 / §7.10).
const STALE_DEFAULTS: [&str; 5] = ["com.example", "example.com", "my-plugin", "MyPlugin", "Example-Plugin"];

const PLACEHOLDER_TOKENS: [&str; 4] = ["{{", "}}", "{%", "%}"];

/// Variables provided by `foreach = "platforms"` scope. They are locals for
/// A4 purposes (not manifest variables). `entry.paths` may name more.
const LOOP_LOCALS: [&str; 8] = [
    "platform",
    "gradlePath",
    "apiCoordinate",
    "apiModule",
    "apiVersion",
    "javaTarget",
    "mainClass",
    "isServerPlatform",
];

/// Platform API package roots the `core` module must never import (§8.2, R5).
const PLATFORM_API_ROOTS: [&str; 9] = [
    "org.bukkit",
    "io.papermc",
    "com.destroystokyo.paper",
    "com.velocitypowered",
    "net.md_5",
    "net.md-5",
    "org.spongepowered",
    "net.minestom",
    "net.kyori",
];

fn config_error(code: &'static str, msg: impl Into<String>) -> Error {
    Error::new(code, EXIT_CONFIG, msg)
}

/// Build the plan (the only source of truth for `--dry-run` and execution).
pub fn build_plan(manifest: &TemplateManifest, spec: &ProjectSpec, resolved: &Resolved) -> Result<Plan> {
    build_plan_with_vars(manifest, spec, resolved, &BTreeMap::new())
}

/// Same as [`build_plan`], with explicit context overrides (tests / future
/// CLI wiring for variables that do not live on `ProjectSpec`).
pub fn build_plan_with_vars(
    manifest: &TemplateManifest,
    spec: &ProjectSpec,
    resolved: &Resolved,
    extras: &BTreeMap<String, Value>,
) -> Result<Plan> {
    let mut warnings = manifest.validate()?;
    let (mut ctx, ctx_warnings) = vars::build_context(spec, resolved, extras)?;
    warnings.extend(ctx_warnings);
    resolve_conditions(manifest, &mut ctx, &mut warnings)?;

    let renderer = Renderer::new();
    let platforms: Vec<String> = ctx
        .get("platforms")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
        .unwrap_or_default();

    let mut state = BuildState {
        files: BTreeMap::new(),
        consumed_raw: BTreeSet::new(),
        consumed_switches: BTreeSet::new(),
        foreach_modules: BTreeSet::new(),
        raw_exempt: BTreeMap::new(),
        template_nonempty: BTreeSet::new(),
        skipped: Vec::new(),
    };

    // Manifest conditions take part in A4 as well.
    for (name, expr) in &manifest.conditions {
        state.consumed_switches.insert(name.clone());
        for id in render::expr_identifiers(expr) {
            if manifest.conditions.contains_key(&id) {
                state.consumed_switches.insert(id);
            } else if manifest::is_switch_name(&id) {
                state.consumed_switches.insert(id);
            } else {
                state.consumed_raw.insert(render::camel(&id));
            }
        }
    }

    for entry in &manifest.entries {
        match entry.foreach.as_deref() {
            None => {
                let ctx_snapshot = ctx.clone();
                process_entry(
                    manifest,
                    entry,
                    &ctx_snapshot,
                    &renderer,
                    &mut state,
                    None,
                )?;
            }
            Some("platforms") => {
                for p in &platforms {
                    state.foreach_modules.insert(p.clone());
                    let mut ictx = vars::loop_scope(&ctx, p);
                    apply_platform_overrides(&mut ictx, resolved.platforms.get(p));
                    process_entry(
                        manifest,
                        entry,
                        &ictx,
                        &renderer,
                        &mut state,
                        Some(p.clone()),
                    )?;
                }
            }
            Some(other) => {
                return Err(config_error(
                    "template.manifest_invalid",
                    format!("不支持 foreach = \"{other}\"（v1 只支持 \"platforms\"）"),
                ));
            }
        }
    }

    let files: Vec<PlannedFile> = state.files.values().cloned().collect();
    let mut plan = Plan {
        root: spec.target_dir.clone(),
        files,
        actions: Vec::new(),
        skipped: std::mem::take(&mut state.skipped),
        warnings,
    };

    // --- self-checks (A1-A5). Run for `--dry-run` too: same code path. -----
    check_a1(&plan, &state.raw_exempt, &state.template_nonempty)?;
    check_a2(&plan, &ctx)?;
    check_a3(&plan, &ctx, &platforms)?;
    check_r3(&plan, &ctx)?;
    check_r4(&plan)?;
    check_r5(&plan)?;
    check_r6(&plan)?;
    check_a4(manifest, &state, &ctx, &mut plan.warnings)?;
    check_a5(&plan, &platforms, &state.foreach_modules)?;
    check_manifest_assertions(manifest, &plan, &ctx, &renderer)?;

    if spec.git {
        plan.actions.push(PlannedAction::GitInit {
            branch: "main".to_string(),
            message: format!("chore: scaffold {} with vinoa", spec.project_name),
        });
    }
    Ok(plan)
}

struct BuildState {
    files: BTreeMap<String, PlannedFile>,
    consumed_raw: BTreeSet<String>,
    consumed_switches: BTreeSet<String>,
    foreach_modules: BTreeSet<String>,
    /// Rendered file -> literal chunks that a `{% raw %}` block intentionally
    /// prints verbatim (A1 exemption).
    raw_exempt: BTreeMap<String, Vec<String>>,
    /// Targets whose template source was non-empty: rendering to an empty
    /// string is then a defect (A1's "rendered empty" net).
    template_nonempty: BTreeSet<String>,
    /// Conditional files intentionally not generated (informational, `--json`
    /// `skipped[]` — not warnings).
    skipped: Vec<String>,
}

fn process_entry(
    manifest: &TemplateManifest,
    entry: &Entry,
    ctx: &Ctx,
    renderer: &Renderer,
    state: &mut BuildState,
    platform: Option<String>,
) -> Result<()> {
    // 1. `when`
    if let Some(expr) = &entry.when {
        collect_condition_consumption(manifest, expr, state);
        if !render::eval_condition(expr, ctx)? {
            // Routine conditional skip: informational, goes to `skipped[]`.
            let label = render_target_path(&entry.path, ctx, renderer)
                .unwrap_or_else(|_| entry.path.clone());
            state.skipped.push(match &platform {
                Some(p) => format!("{label}（platform={p}, when: {expr}）"),
                None => format!("{label}（when: {expr}）"),
            });
            return Ok(());
        }
    }

    // 2. target path
    let target = render_target_path(&entry.path, ctx, renderer)?;
    // Path placeholders are variables too (A4); `platform` is a loop local.
    for name in placeholders(&entry.path) {
        if name != "platform" {
            state.consumed_raw.insert(render::camel(&name));
        }
    }

    // 3. source path
    let raw_source = match &entry.template {
        Some(src) => src.clone(),
        None => {
            if entry.path.contains("{{") {
                return Err(config_error(
                    "template.bad_target",
                    format!(
                        "条目 {} 的目标路径含占位符却没有 `template` 源路径",
                        entry.path
                    ),
                ));
            }
            entry.path.clone()
        }
    };
    let mut source = raw_source;
    if let Some(p) = &platform {
        source = source.replace("_p_", p);
    }
    if source.contains("{{") || source.contains("{%") {
        return Err(config_error(
            "template.bad_target",
            format!("源路径必须是字面量（{source}）；平台分叉请用 `_p_`"),
        ));
    }
    let bytes = manifest::read_source(&manifest.source, &source)?;

    // 4. render / copy
    let content: Vec<u8> = match entry.render {
        Render::Template => {
            let src = std::str::from_utf8(&bytes).map_err(|e| {
                config_error(
                    "render.syntax_error",
                    format!("模板 {source} 不是合法 UTF-8: {e}"),
                )
            })?;
            let mut locals: Vec<String> = entry.paths.clone();
            if platform.is_some() {
                locals.extend(LOOP_LOCALS.iter().map(|s| s.to_string()));
            }
            let ex = render::extract_vars(src, &locals);
            state.consumed_raw.extend(ex.raw);
            state.consumed_switches.extend(ex.switches);
            let rendered = renderer.render(&source, src, ctx)?;
            let exempt = render::raw_literals(src);
            if !exempt.is_empty() {
                state.raw_exempt.insert(target.clone(), exempt);
            }
            if !src.trim().is_empty() {
                state.template_nonempty.insert(target.clone());
            }
            rendered.into_bytes()
        }
        Render::Copy => bytes,
    };

    let is_text = match entry.render {
        Render::Template => true,
        Render::Copy => entry.text.unwrap_or_else(|| !looks_binary(&content)),
    };
    let content = if is_text {
        let text = String::from_utf8_lossy(&content).to_string();
        normalize_text(&text, &target).into_bytes()
    } else {
        content
    };
    let executable = match entry.mode.as_deref() {
        Some(m) => {
            let m = manifest::parse_mode(m).ok_or_else(|| {
                config_error(
                    "template.manifest_invalid",
                    format!("mode 非法: {m}"),
                )
            })?;
            m & 0o111 != 0
        }
        None => target.rsplit('/').next() == Some("gradlew"),
    };

    if let Some(prev) = state.files.get(&target) {
        return Err(config_error(
            "template.bad_target",
            format!(
                "目标路径重复: {target}（{} 字节 vs {} 字节）",
                prev.content.len(),
                content.len()
            ),
        ));
    }
    state.files.insert(
        target.clone(),
        PlannedFile {
            path: target,
            content,
            executable,
        },
    );
    Ok(())
}

fn collect_condition_consumption(manifest: &TemplateManifest, expr: &str, state: &mut BuildState) {
    for id in render::expr_identifiers(expr) {
        if manifest.conditions.contains_key(&id) || manifest::is_switch_name(&id) {
            state.consumed_switches.insert(id);
        } else {
            state.consumed_raw.insert(render::camel(&id));
        }
    }
}

fn apply_platform_overrides(ctx: &mut Ctx, plan: Option<&PlatformPlan>) {
    let Some(plan) = plan else { return };
    let cap = &plan.capability;
    ctx.insert(
        "pluginYmlApiVersion".into(),
        json!(cap.plugin_yml_api_version.clone().unwrap_or_default()),
    );
    let libs = cap.libraries_enabled
        || ctx.get("cap_libraries").and_then(Value::as_bool).unwrap_or(false);
    ctx.insert("cap_libraries".into(), json!(libs));
    ctx.insert(
        "cap_api_version_line".into(),
        json!(cap.plugin_yml_api_version.is_some()),
    );
    ctx.insert("cap_legacy_namespace".into(), json!(cap.legacy_namespace));
    ctx.insert("paperweightEnabled".into(), json!(cap.paperweight));
    ctx.insert("reobfEnabled".into(), json!(cap.reobf));
    ctx.insert(
        "runTask".into(),
        json!(match plan.id.as_str() {
            "paper" | "folia" => "run-paper",
            "velocity" => "run-velocity",
            _ => "none",
        }),
    );
}

// ---------------------------------------------------------------------------
// Condition resolution
// ---------------------------------------------------------------------------

fn resolve_conditions(
    manifest: &TemplateManifest,
    ctx: &mut Ctx,
    warnings: &mut Vec<String>,
) -> Result<()> {
    let mut done: BTreeSet<String> = BTreeSet::new();
    for name in manifest.conditions.keys() {
        resolve_one(&manifest.conditions, name, ctx, &mut done, &mut Vec::new())?;
    }
    let _ = warnings;
    Ok(())
}

fn resolve_one(
    conditions: &BTreeMap<String, String>,
    name: &str,
    ctx: &mut Ctx,
    done: &mut BTreeSet<String>,
    stack: &mut Vec<String>,
) -> Result<bool> {
    if done.contains(name) {
        return Ok(ctx.get(name).and_then(Value::as_bool).unwrap_or(false));
    }
    let Some(expr) = conditions.get(name) else {
        return ctx
            .get(name)
            .and_then(Value::as_bool)
            .ok_or_else(|| {
                config_error(
                    "template.undefined_variable",
                    format!("条件 `{name}` 未在 [conditions] 中声明"),
                )
            });
    };
    if stack.iter().any(|s| s == name) {
        return Err(config_error(
            "template.manifest_invalid",
            format!("[conditions] 存在循环依赖: {}", stack.join(" -> ")),
        ));
    }
    stack.push(name.to_string());
    for dep in render::expr_identifiers(expr) {
        if conditions.contains_key(&dep) {
            resolve_one(conditions, &dep, ctx, done, stack)?;
        }
    }
    let value = render::eval_condition(expr, ctx).map_err(|e| {
        config_error(
            "template.manifest_invalid",
            format!("[conditions] {name} = \"{expr}\": {}", e.message),
        )
    })?;
    stack.pop();
    ctx.insert(name.to_string(), json!(value));
    done.insert(name.to_string());
    Ok(value)
}

// ---------------------------------------------------------------------------
// Paths / text helpers
// ---------------------------------------------------------------------------

fn render_target_path(raw: &str, ctx: &Ctx, renderer: &Renderer) -> Result<String> {
    for name in placeholders(raw) {
        if !PATH_VARS.contains(&name.as_str()) {
            return Err(config_error(
                "template.bad_path_var",
                format!("路径 `{raw}` 使用了不可入路径的变量 `{name}`（§7.4 只允许 {PATH_VARS:?}）"),
            )
            .with_hint("把该值渲染进文件内容，或改用允许的四个变量之一"));
        }
    }
    let rendered = renderer.render(raw, raw, ctx)?;
    validate_target(&rendered, raw)?;
    Ok(rendered)
}

/// `{{ name }}` occurrences in a raw target path (pub(crate) for tests).
pub(crate) fn placeholders(raw: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = raw;
    while let Some(i) = rest.find("{{") {
        let after = &rest[i + 2..];
        match after.find("}}") {
            Some(j) => {
                out.push(after[..j].trim().to_string());
                rest = &after[j + 2..];
            }
            None => break,
        }
    }
    out
}

fn validate_target(rendered: &str, raw: &str) -> Result<()> {
    if rendered.starts_with('/') || rendered.starts_with('\\') {
        return Err(config_error(
            "template.bad_target",
            format!("目标路径必须是相对路径: {rendered}（来自 `{raw}`）"),
        ));
    }
    if rendered.contains('\\') {
        return Err(config_error(
            "template.bad_target",
            format!("目标路径不能含反斜杠（统一 `/`）: {rendered}"),
        ));
    }
    if rendered.contains(':') {
        return Err(config_error(
            "template.bad_target",
            format!("目标路径不能含 `:`: {rendered}"),
        ));
    }
    for seg in rendered.split('/') {
        if seg.is_empty() || seg == "." || seg == ".." {
            return Err(config_error(
                "template.bad_target",
                format!("目标路径存在非法段 `{seg}`: {rendered}"),
            ));
        }
        if seg.chars().any(|c| c.is_control()) {
            return Err(config_error(
                "template.bad_target",
                format!("目标路径含控制字符: {rendered}"),
            ));
        }
    }
    if rendered.contains("{{") || rendered.contains("{%") {
        return Err(config_error(
            "template.bad_path_var",
            format!("目标路径渲染后仍有占位符: {rendered}"),
        ));
    }
    Ok(())
}

/// LF everywhere; `.bat` is CRLF (spec §7.4).
fn normalize_text(s: &str, target: &str) -> String {
    let unified = s.replace("\r\n", "\n").replace('\r', "\n");
    if target.ends_with(".bat") {
        unified.replace('\n', "\r\n")
    } else {
        unified
    }
}

fn looks_binary(bytes: &[u8]) -> bool {
    bytes.iter().take(8192).any(|b| *b == 0)
}

fn is_text_file(f: &PlannedFile) -> bool {
    if f.content.iter().take(8192).any(|b| *b == 0) {
        return false;
    }
    // Files explicitly copied as binary stay exempt from text scans.
    !f.path.ends_with(".jar")
}

fn content_str(f: &PlannedFile) -> String {
    String::from_utf8_lossy(&f.content).to_string()
}

fn loc_of(content: &str, byte_offset: usize) -> (usize, usize) {
    let mut line = 1;
    let mut col = 1;
    for (i, c) in content.char_indices() {
        if i >= byte_offset {
            break;
        }
        if c == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

// ---------------------------------------------------------------------------
// Assertions A1-A5 + rename rules R3/R5/R6
// ---------------------------------------------------------------------------

/// A1: no leftover `{{` / `}}` / `{%` / `%}` in any text file (paths are
/// validated during rendering). `{% raw %}` literals are exempt.
fn check_a1(
    plan: &Plan,
    raw_exempt: &BTreeMap<String, Vec<String>>,
    template_nonempty: &BTreeSet<String>,
) -> Result<()> {
    for f in &plan.files {
        if !is_text_file(f) {
            continue;
        }
        let content = content_str(f);
        // Emptiness is judged on the real output, before `{% raw %}` chunks
        // (which may legitimately be the whole file) are stripped for the scan.
        let rendered_empty = content.trim().is_empty();
        // GitHub Actions `${{ ... }}` is legitimate output, not a leftover
        // placeholder (spec §7.2 raw rules keep working on top of this).
        let mut scan = render::strip_github_expressions(&content);
        if let Some(chunks) = raw_exempt.get(&f.path) {
            for c in chunks {
                if !c.is_empty() {
                    scan = scan.replace(c.as_str(), "");
                }
            }
        }
        for token in PLACEHOLDER_TOKENS {
            if let Some(off) = scan.find(token) {
                let (line, col) = loc_of(&scan, off);
                return Err(config_error(
                    "render.leftover_placeholder",
                    format!(
                        "渲染后仍有未替换的占位符 `{token}`: {}:{line}:{col}",
                        f.path
                    ),
                )
                .with_hint("检查 render=\"copy\" 的条目是否其实是模板，或 `{{` 是否漏写"));
            }
        }
        // A1 also catches "a variable rendered to an empty string" at file level.
        if template_nonempty.contains(&f.path) && rendered_empty {
            return Err(config_error(
                "render.assertion_failed",
                format!("{} 的模板非空，但渲染结果为空（变量渲染成了空串）", f.path),
            )
            .with_hint("A1/A5：双向核对清单 vars 与模板消费的变量，别让缺失值静默成空串"));
        }
    }
    Ok(())
}

/// R4: cross-module references must go through `gradlePath` (`platforms:<p>`),
/// never a hand-written `platforms/...` path.
fn check_r4(plan: &Plan) -> Result<()> {
    for f in &plan.files {
        if !f.path.ends_with(".gradle.kts") {
            continue;
        }
        let content = content_str(f);
        for needle in ["project(\":platforms/", "project(':platforms/", "project(\":platforms\\"] {
            if let Some(off) = content.find(needle) {
                let (line, col) = loc_of(&content, off);
                return Err(config_error(
                    "render.assertion_failed",
                    format!("跨模块引用手写了路径 `{needle}`: {}:{line}:{col}", f.path),
                )
                .with_hint("R4: Gradle 用 `:`，一律走 `gradlePath`（platforms:<p>）"));
            }
        }
    }
    Ok(())
}

/// A2: template-set default identity strings must not leak.
fn check_a2(plan: &Plan, ctx: &Ctx) -> Result<()> {
    let get = |k: &str| ctx.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    let package_name = get("packageName");
    let project_name = get("projectName");
    let plugin_name = get("pluginName");
    let website = get("website");
    for f in &plan.files {
        if !is_text_file(f) {
            continue;
        }
        let content = content_str(f);
        for token in STALE_DEFAULTS {
            let allowed = match token {
                "com.example" => package_name.contains(token),
                "example.com" => website.contains(token),
                "my-plugin" => project_name.contains(token),
                "MyPlugin" => plugin_name.contains(token),
                _ => false,
            };
            if allowed {
                continue;
            }
            if let Some(off) = content.find(token) {
                let (line, col) = loc_of(&content, off);
                return Err(config_error(
                    "render.stale_default",
                    format!("生成物里残留模板默认身份串 `{token}`: {}:{line}:{col}", f.path),
                )
                .with_hint("模板正文不得硬编码默认值；使用对应变量渲染"));
            }
        }
    }
    Ok(())
}

/// A3: every platform module has its entry class, and metadata `main` resolves
/// to that file (R2/A3).
fn check_a3(plan: &Plan, ctx: &Ctx, platforms: &[String]) -> Result<()> {
    let package_path = ctx.get("packagePath").and_then(Value::as_str).unwrap_or("").to_string();
    let plugin_name = ctx.get("pluginName").and_then(Value::as_str).unwrap_or("").to_string();
    let main_classes = ctx.get("mainClasses").cloned().unwrap_or(Value::Null);
    for p in platforms {
        let expected_rel = format!(
            "platforms/{p}/src/main/java/{package_path}/{p}/{plugin_name}.java"
        );
        let metadata = [
            format!("platforms/{p}/src/main/resources/plugin.yml"),
            format!("platforms/{p}/src/main/resources/paper-plugin.yml"),
        ]
        .into_iter()
        .find(|path| plan.files.iter().any(|f| f.path == *path));
        if let Some(path) = metadata {
            let f = plan.files.iter().find(|f| f.path == path).unwrap();
            let content = content_str(f);
            let main = content
                .lines()
                .find_map(|l| l.trim().strip_prefix("main:"))
                .map(|v| v.trim().trim_matches('"').trim_matches('\'').to_string());
            let expected = main_classes.get(p).and_then(Value::as_str).unwrap_or("");
            match main {
                None => {
                    return Err(config_error(
                        "render.main_class_mismatch",
                        format!("元数据 {path} 缺少 `main:` 行（期望 {expected}）"),
                    ));
                }
                Some(got) if got != expected => {
                    return Err(config_error(
                        "render.main_class_mismatch",
                        format!(
                            "元数据 {path} 的 main = `{got}`，与 mainClass.{p} = `{expected}` 不一致"
                        ),
                    )
                    .with_hint("元数据必须写 `main: {{ mainClass }}`（R6）"));
                }
                Some(_) => {}
            }
        }
        if !plan.files.iter().any(|f| f.path == expected_rel) {
            return Err(config_error(
                "render.main_class_mismatch",
                format!("平台模块 {p} 缺少入口类文件: {expected_rel}"),
            )
            .with_hint("R2：文件 `{{packagePath}}/<platform>/{{pluginName}}.java` 与类名同源"));
        }
    }
    Ok(())
}

/// R3: `settings.gradle.kts` must set `rootProject.name` to the project name.
fn check_r3(plan: &Plan, ctx: &Ctx) -> Result<()> {
    let Some(f) = plan.files.iter().find(|f| f.path == "settings.gradle.kts") else {
        return Ok(());
    };
    let project = ctx.get("projectName").and_then(Value::as_str).unwrap_or("");
    let content = content_str(f);
    let line = content
        .lines()
        .find(|l| l.contains("rootProject.name"))
        .unwrap_or("");
    if line.is_empty() || !line.contains(project) {
        return Err(config_error(
            "render.assertion_failed",
            format!(
                "settings.gradle.kts 未把 rootProject.name 设为 `{project}`（实际行: `{}`）",
                line.trim()
            ),
        )
        .with_hint("R3: rootProject.name = \"{{ projectName }}\""));
    }
    Ok(())
}

/// R5: `core` must not import any platform API (§8.2).
fn check_r5(plan: &Plan) -> Result<()> {
    for f in &plan.files {
        if !f.path.starts_with("core/") || !f.path.ends_with(".java") {
            continue;
        }
        let content = content_str(f);
        for root in PLATFORM_API_ROOTS {
            if let Some(off) = content.find(root) {
                let (line, col) = loc_of(&content, off);
                return Err(config_error(
                    "render.core_platform_import",
                    format!("core 模块 import 了平台 API `{root}`: {}:{line}:{col}", f.path),
                )
                .with_hint("R5/core 边界（§8.2）：core 只能放平台无关接口与纯逻辑"));
            }
        }
    }
    Ok(())
}

/// R6: `plugin.yml`'s build-time `version` must be quoted (SnakeYAML would
/// otherwise coerce it to a number).
fn check_r6(plan: &Plan) -> Result<()> {
    for f in &plan.files {
        let is_meta = f.path.ends_with("plugin.yml") || f.path.ends_with("paper-plugin.yml");
        if !is_meta {
            continue;
        }
        let content = content_str(f);
        for line in content.lines() {
            let t = line.trim();
            if let Some(rest) = t.strip_prefix("version:") {
                let v = rest.trim();
                if v.contains("${version}") && !v.contains("\"${version}\"") {
                    return Err(config_error(
                        "render.assertion_failed",
                        format!("{} 的 version 必须带双引号: `{}`", f.path, t),
                    )
                    .with_hint("R6: version: \"${version}\"（防 SnakeYAML 数字强转）"));
                }
            }
        }
    }
    Ok(())
}

/// A4: declared variables and consumed variables are equal sets (bidirectional).
fn check_a4(
    manifest: &TemplateManifest,
    state: &BuildState,
    ctx: &Ctx,
    warnings: &mut Vec<String>,
) -> Result<()> {
    let declared: BTreeSet<String> = manifest.vars.iter().map(|v| render::camel(v)).collect();
    let consumed = &state.consumed_raw;
    let mut problems = Vec::new();
    let missing: Vec<&String> = consumed.difference(&declared).collect();
    if !missing.is_empty() {
        problems.push(format!("模板消费但清单未声明: {}", join(&missing)));
    }
    let unused: Vec<&String> = declared.difference(consumed).collect();
    if !unused.is_empty() {
        problems.push(format!("清单声明但模板未消费: {}", join(&unused)));
    }
    if !problems.is_empty() {
        return Err(config_error(
            "template.variable_mismatch",
            format!("清单 vars 与实际消费的变量不互为子集（A4）: {}", problems.join("; ")),
        )
        .with_hint("双向核对方能拦住拼写错误与渲染成空串的变量"));
    }
    // Declared-but-unused conditions are a smell, not an error.
    let unused_conditions: Vec<String> = manifest
        .conditions
        .keys()
        .filter(|k| !state.consumed_switches.contains(*k))
        .cloned()
        .collect();
    if !unused_conditions.is_empty() {
        warnings.push(format!("[conditions] 未被任何模板/when 消费: {}", unused_conditions.join(", ")));
    }
    // Consumed switches must resolve to a built-in or a declared condition.
    for sw in &state.consumed_switches {
        if ctx.get(sw).is_none() {
            return Err(config_error(
                "template.undefined_variable",
                format!("开关 `{sw}` 未定义（既不是内置开关，也未在 [conditions] 声明）"),
            )
            .with_hint("在清单 [conditions] 里声明它"));
        }
    }
    Ok(())
}

/// A5: settings `include` set == `platforms/*` dir set == `foreach` output set.
fn check_a5(
    plan: &Plan,
    platforms: &[String],
    foreach_modules: &BTreeSet<String>,
) -> Result<()> {
    let expected: BTreeSet<String> = platforms.iter().cloned().collect();
    let mut dirs: BTreeSet<String> = BTreeSet::new();
    for f in &plan.files {
        let mut segs = f.path.split('/');
        if segs.next() == Some("platforms") {
            if let Some(p) = segs.next() {
                dirs.insert(p.to_string());
            }
        }
    }
    let includes: Option<BTreeSet<String>> = plan
        .files
        .iter()
        .find(|f| f.path == "settings.gradle.kts")
        .map(|f| {
            let content = content_str(f);
            let mut set = BTreeSet::new();
            for (i, _) in content.match_indices("\"platforms:") {
                let rest = &content[i + 1..];
                if let Some(end) = rest.find('"') {
                    let after_colon = &rest["platforms:".len()..end];
                    if !after_colon.is_empty() {
                        set.insert(after_colon.to_string());
                    }
                }
            }
            set
        });
    let mut problems = Vec::new();
    if !expected.is_empty() || !platforms.is_empty() {
        if dirs != expected {
            problems.push(format!(
                "platforms/ 目录集合 {:?} != platforms 集合 {:?}",
                sorted(&dirs),
                sorted(&expected)
            ));
        }
        if foreach_modules != &expected {
            problems.push(format!(
                "foreach 产出集合 {:?} != platforms 集合 {:?}",
                sorted(foreach_modules),
                sorted(&expected)
            ));
        }
        if let Some(includes) = &includes {
            if includes != &expected {
                problems.push(format!(
                    "settings.gradle.kts include 集合 {:?} != platforms 集合 {:?}",
                    sorted(includes),
                    sorted(&expected)
                ));
            }
        }
    }
    if !problems.is_empty() {
        return Err(config_error(
            "render.module_set_mismatch",
            format!("模块集合不一致（A5）: {}", problems.join("; ")),
        ));
    }
    Ok(())
}

/// `[[assertions]]`: rendered `contains[]` must appear in the target.
fn check_manifest_assertions(
    manifest: &TemplateManifest,
    plan: &Plan,
    ctx: &Ctx,
    renderer: &Renderer,
) -> Result<()> {
    for a in &manifest.assertions {
        let target = renderer.render(&a.target, &a.target, ctx)?;
        let Some(f) = plan.files.iter().find(|f| f.path == target) else {
            return Err(config_error(
                "template.bad_target",
                format!("[[assertions]] target 不存在于计划中: {target}"),
            ));
        };
        let content = content_str(f);
        for needle in &a.contains {
            let rendered = renderer.render(needle, needle, ctx)?;
            if !content.contains(&rendered) {
                return Err(config_error(
                    "render.assertion_failed",
                    format!("断言失败: {target} 不含 `{rendered}`"),
                ));
            }
        }
    }
    Ok(())
}

fn join(names: &[&String]) -> String {
    names.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
}

fn sorted(set: &BTreeSet<String>) -> Vec<&str> {
    set.iter().map(String::as_str).collect()
}
