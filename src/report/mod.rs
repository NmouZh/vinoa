//! Human and JSON reporting (spec §11). OWNER: cli-dev.
//!
//! Hard contract: with `--json`, stdout carries **exactly one** JSON document and
//! every human-readable line goes to stderr. Without `--json`, the JSON buffers
//! are never touched.
use crate::error::Error;
use crate::types::Plan;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// JSON schema tag for the `init` command (spec §11.5).
pub const JSON_SCHEMA: &str = "vinoa.init/v1";

static JSON_MODE: AtomicBool = AtomicBool::new(false);
static JSON_EMITTED: AtomicBool = AtomicBool::new(false);
static COMMAND: std::sync::Mutex<&'static str> = std::sync::Mutex::new("init");

pub fn set_json_mode(on: bool) {
    JSON_MODE.store(on, Ordering::SeqCst);
}

pub fn json_mode() -> bool {
    JSON_MODE.load(Ordering::SeqCst)
}

/// Record the active subcommand so error documents carry the right `command`.
pub fn set_command(command: &'static str) {
    if let Ok(mut c) = COMMAND.lock() {
        *c = command;
    }
}

pub fn current_command() -> &'static str {
    COMMAND.lock().map(|c| *c).unwrap_or("init")
}

/// The first document of a process goes to stdout; any later one is diverted to
/// stderr so stdout always carries **exactly one** JSON document (spec §11.5 —
/// the same "stderr bypass" the interrupt path needs).
pub fn emit_json(doc: &serde_json::Value) {
    let text = serde_json::to_string(doc).unwrap_or_else(|_| "{}".to_string());
    if emit_to_stdout() {
        let mut out = std::io::stdout().lock();
        let _ = writeln!(out, "{text}");
        let _ = out.flush();
    } else {
        let mut out = std::io::stderr().lock();
        let _ = writeln!(out, "{text}");
        let _ = out.flush();
    }
}

/// True for the first emission in this process (see [`emit_json`]).
fn emit_to_stdout() -> bool {
    !JSON_EMITTED.swap(true, Ordering::SeqCst)
}

/// Whether a JSON document was already written (used by tests and callers that
/// want to shape the final document).
pub fn json_emitted() -> bool {
    JSON_EMITTED.load(Ordering::SeqCst)
}

#[cfg(test)]
pub fn reset_emit_state() {
    JSON_EMITTED.store(false, Ordering::SeqCst);
}

pub fn info(msg: &str) {
    eprint!("{msg}");
    if !msg.ends_with('\n') {
        eprintln!();
    }
}

pub fn warn(msg: &str) {
    eprintln!("⚠ {msg}");
}

/// `phase` for a stable error code (spec §11.5: input|validate|matrix|precheck|plan|render|verify|write|post).
pub fn phase_for_code(code: &str) -> &'static str {
    if code.starts_with("matrix.") {
        "matrix"
    } else if code.starts_with("render.") {
        "render"
    } else if code.starts_with("template.") {
        "plan"
    } else if code.starts_with("precheck.") || code.starts_with("env.") {
        "precheck"
    } else if code.starts_with("verify") {
        "verify"
    } else if code.starts_with("write.") || code.starts_with("fs.") || code.starts_with("io.") {
        "write"
    } else if code.starts_with("config.") || code.starts_with("usage.") || code.starts_with("interrupt.") || code.starts_with("input.") {
        "input"
    } else if code.starts_with("var.") || code.starts_with("data.") {
        "validate"
    } else {
        "post"
    }
}

/// Classification string for `status` — agents branch on this, humans on `exit_code`.
pub fn status_for_exit(exit: u8) -> &'static str {
    match exit {
        crate::error::EXIT_OK => "ok",
        crate::error::EXIT_VERIFY_FAILED => "verify_failed",
        crate::error::EXIT_USAGE => "usage_error",
        crate::error::EXIT_DATA => "data_error",
        crate::error::EXIT_CANTCREAT => "cannot_create",
        crate::error::EXIT_IO => "io_error",
        crate::error::EXIT_TEMPFAIL => "tempfail",
        crate::error::EXIT_CONFIG => "config_error",
        crate::error::EXIT_INTERRUPT => "interrupted",
        _ => "error",
    }
}

/// Human-readable error: what happened, the stable code, and the next action.
pub fn error(err: &Error) {
    if json_mode() {
        emit_json(&error_document(err));
    }
    eprintln!("✗ {}", err.message);
    eprintln!("  错误码: {}", err.code);
    if let Some(hint) = &err.hint {
        eprintln!("  hint: {hint}");
    }
}

/// Pure document behind the `--json` error path (spec §11.5): exactly one
/// document, `ok:false`, stable `code` + `phase`, same `exit_code` as the human
/// path.
pub fn error_document(err: &Error) -> serde_json::Value {
    let phase = phase_for_code(err.code);
    serde_json::json!({
        "schema": JSON_SCHEMA,
        "vinoa": env!("CARGO_PKG_VERSION"),
        "ok": false,
        "command": current_command(),
        "phase": phase,
        "generated_at": utc_now_rfc3339(),
        "status": status_for_exit(err.exit_code()),
        "exit_code": err.exit_code(),
        "warnings": [],
        "errors": [error_entry(err)],
    })
}

/// One `errors[]` entry (spec §11.5). `line`/`column` are omitted — the frozen
/// [`Error`] contract does not carry positions yet.
pub fn error_entry(err: &Error) -> serde_json::Value {
    serde_json::json!({
        "code": err.code,
        "phase": phase_for_code(err.code),
        "severity": "error",
        "message": err.message,
        "hint": err.hint,
    })
}

/// Human-readable plan summary on **stderr** (spec §5 phase 8).
///
/// Conditional skips get one compact line; warnings are marked so they are not
/// mistaken for skips (they are two different things in `--json` too).
pub fn plan_human(plan: &Plan) {
    let mut out = String::new();
    out.push_str(&format!("即将创建（共 {} 个文件）: {}\n", plan.files.len(), plan.root.display()));
    for f in &plan.files {
        let mark = if f.executable { " *" } else { "" };
        out.push_str(&format!("  {}{mark}\n", f.path));
    }
    if !plan.skipped.is_empty() {
        out.push_str(&format!("  跳过 {} 项（条件未命中）:\n", plan.skipped.len()));
        for s in &plan.skipped {
            out.push_str(&format!("    · {s}\n"));
        }
    }
    for action in &plan.actions {
        out.push_str(&format!("  动作: {action:?}\n"));
    }
    for w in &plan.warnings {
        out.push_str(&format!("  ⚠ 警告: {w}\n"));
    }
    info(&out);
}

/// One JSON document for `init` results (spec §11.5). `extra` is deep-merged on
/// top of the base document, so the orchestrator supplies `project`, `precheck`,
/// `matrix`, `template_source`, `verify`, `errors`, … and may override defaults.
pub fn plan_json(plan: &Plan, extra: &serde_json::Value) {
    plan_json_for(plan, extra, current_command());
}

pub fn plan_json_for(plan: &Plan, extra: &serde_json::Value, command: &'static str) {
    emit_json(&build_plan_document(plan, extra, command));
}

/// Pure document builder behind [`plan_json_for`] — testable without stdout.
pub fn build_plan_document(
    plan: &Plan,
    extra: &serde_json::Value,
    command: &'static str,
) -> serde_json::Value {
    let dry_run = extra.get("dry_run").and_then(|v| v.as_bool()).unwrap_or(false);
    let count = plan.files.len();
    let mut base = serde_json::json!({
        "schema": JSON_SCHEMA,
        "vinoa": env!("CARGO_PKG_VERSION"),
        "ok": true,
        "command": command,
        "phase": "post",
        "generated_at": utc_now_rfc3339(),
        "status": "ok",
        "exit_code": crate::error::EXIT_OK,
        "files": if dry_run {
            serde_json::json!({ "count": count, "planned": count, "written": 0 })
        } else {
            serde_json::json!({ "count": count, "written": count })
        },
        "warnings": plan.warnings.clone(),
        "skipped": plan.skipped.clone(),
    });

    if dry_run {
        let render_map = extra.get("render_map").cloned().unwrap_or(serde_json::Value::Null);
        let planned: Vec<serde_json::Value> = plan
            .files
            .iter()
            .map(|f| {
                let render = render_map
                    .get(&f.path)
                    .and_then(|v| v.as_str())
                    .unwrap_or("template");
                serde_json::json!({
                    "target": f.path,
                    "render": render,
                    "bytes": f.content.len(),
                    "sha256": crate::template::sha256_hex(&f.content),
                })
            })
            .collect();
        base["planned"] = serde_json::Value::Array(planned);
        // `skipped[]` is the condition-skip list (each entry carries its `when`
        // reason), never mixed with `warnings[]`.
        if extra.get("skipped").is_some() {
            base["skipped"] = extra["skipped"].clone();
        }
    }

    let mut merged = base;
    merge(&mut merged, extra);
    merged
}

/// Recursive object merge: values in `over` win over `base`.
fn merge(base: &mut serde_json::Value, over: &serde_json::Value) {
    match (base, over) {
        (serde_json::Value::Object(b), serde_json::Value::Object(o)) => {
            for (k, v) in o {
                match b.get_mut(k) {
                    Some(slot) => merge(slot, v),
                    None => {
                        b.insert(k.clone(), v.clone());
                    }
                }
            }
        }
        (slot, value) => *slot = value.clone(),
    }
}

/// UTC timestamp for any user-visible JSON (`generated_at`).
pub fn utc_now_rfc3339() -> String {
    let (y, mo, d, h, mi, s) = utc_now_parts();
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}

/// Compact UTC stamp used in `verify-<stamp>.log` file names.
pub fn utc_stamp_compact() -> String {
    let (y, mo, d, h, mi, s) = utc_now_parts();
    format!("{y:04}{mo:02}{d:02}T{h:02}{mi:02}{s:02}Z")
}

fn utc_now_parts() -> (i64, u32, u32, u32, u32, u32) {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    utc_parts(secs)
}

/// Split a Unix timestamp (UTC) into `(year, month, day, hour, minute, second)`.
pub fn utc_parts(secs: i64) -> (i64, u32, u32, u32, u32, u32) {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (y, mo, d) = civil_from_days(days);
    (y, mo, d, (rem / 3600) as u32, ((rem % 3600) / 60) as u32, (rem % 60) as u32)
}

/// Howard Hinnant's `civil_from_days` (proleptic Gregorian, no dependencies).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    (if m <= 2 { y + 1 } else { y }, m as u32, d as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{PlannedAction, PlannedFile};
    use std::path::PathBuf;

    fn sample_plan() -> Plan {
        Plan {
            root: PathBuf::from("/tmp/my-plugin"),
            files: vec![
                PlannedFile {
                    path: "settings.gradle.kts".into(),
                    content: b"rootProject.name = \"my-plugin\"\n".to_vec(),
                    executable: false,
                },
                PlannedFile {
                    path: "gradlew".into(),
                    content: b"#!/bin/sh\n".to_vec(),
                    executable: true,
                },
            ],
            actions: vec![PlannedAction::GitInit {
                branch: "main".into(),
                message: "chore: scaffold my-plugin with vinoa".into(),
            }],
            warnings: vec!["矩阵联网刷新失败，回退内置矩阵".into()],
            skipped: vec!["README.en.md (when: cap_dual_lang)".into()],
        }
    }

    /// The acceptance test for `--json`: exactly one parseable document on stdout.
    #[test]
    fn plan_json_is_exactly_one_parseable_document() {
        let plan = sample_plan();
        let mut buf = Vec::new();
        {
            // Capture stdout through a pipe by rendering to a string first.
            let doc = super::build_plan_document(&plan, &serde_json::json!({ "dry_run": false }), "init");
            buf.extend_from_slice(serde_json::to_string(&doc).unwrap().as_bytes());
        }
        let text = String::from_utf8(buf).unwrap();
        assert_eq!(text.matches('{').count(), text.matches('}').count());
        let value: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(value["schema"], JSON_SCHEMA);
        assert_eq!(value["files"]["count"], 2);
        assert_eq!(value["files"]["written"], 2);
        assert_eq!(value["ok"], true);
        // Skips and warnings are separate lists (a `zh` run has no en resources —
        // that is not a warning).
        assert_eq!(value["skipped"].as_array().unwrap().len(), 1);
        assert_eq!(value["warnings"].as_array().unwrap().len(), 1);
        assert!(value["skipped"][0].as_str().unwrap().contains("cap_dual_lang"));
        assert!(value["warnings"][0].as_str().unwrap().contains("回退内置矩阵"));
        assert!(!value["warnings"].as_array().unwrap().iter().any(|w| w.as_str().unwrap().contains("cap_dual_lang")));
    }

    #[test]
    fn dry_run_document_uses_planned_and_skipped() {
        let plan = sample_plan();
        let doc = build_plan_document(
            &plan,
            &serde_json::json!({ "dry_run": true, "skipped": [{"target": "x", "when": "has_example"}] }),
            "init",
        );
        assert_eq!(doc["files"]["written"], 0);
        assert_eq!(doc["files"]["planned"], 2);
        assert_eq!(doc["planned"][0]["target"], "settings.gradle.kts");
        assert!(doc["planned"][0]["sha256"].as_str().unwrap().len() == 64);
        // An explicit structured `skipped` from the orchestrator wins.
        assert_eq!(doc["skipped"][0]["when"], "has_example");
    }

    #[test]
    fn dry_run_without_explicit_skipped_falls_back_to_the_plan() {
        let plan = sample_plan();
        let doc = build_plan_document(&plan, &serde_json::json!({ "dry_run": true }), "init");
        assert_eq!(doc["skipped"][0], "README.en.md (when: cap_dual_lang)");
    }

    #[test]
    fn extra_overrides_base_fields() {
        let plan = sample_plan();
        let doc = build_plan_document(
            &plan,
            &serde_json::json!({
                "ok": false,
                "exit_code": 2,
                "status": "verify_failed",
                "project": {"name": "my-plugin", "mc": "1.21.11", "platforms": ["paper", "bukkit"]},
                "verify": {"ran": true, "ok": false, "reason": "build_failed"},
            }),
            "init",
        );
        assert_eq!(doc["exit_code"], 2);
        assert_eq!(doc["status"], "verify_failed");
        assert_eq!(doc["project"]["platforms"][1], "bukkit");
        assert_eq!(doc["verify"]["reason"], "build_failed");
    }

    #[test]
    fn phases_and_status_follow_the_frozen_tables() {
        assert_eq!(phase_for_code("render.leftover_placeholder"), "render");
        assert_eq!(phase_for_code("matrix.unsupported_combination"), "matrix");
        assert_eq!(phase_for_code("env.missing_java"), "precheck");
        assert_eq!(phase_for_code("write.exists"), "write");
        assert_eq!(phase_for_code("var.invalid_value"), "validate");
        assert_eq!(phase_for_code("verify_failed"), "verify");
        for (code, exit, status) in [
            ("render.stale_default", 65u8, "data_error"),
            ("fs.cannot_create", 73, "cannot_create"),
            ("config.invalid", 78, "config_error"),
            ("usage.invalid", 64, "usage_error"),
            ("verify_failed", 2, "verify_failed"),
            ("env.missing_java", 75, "tempfail"),
        ] {
            assert_eq!(status_for_exit(exit), status, "{code}");
        }
        // Helpers added to `error.rs` by the Lead.
        assert_eq!(phase_for_code("interrupt.cancelled"), "input");
        assert_eq!(phase_for_code("input.failed"), "input");
        assert_eq!(phase_for_code("write.io"), "write");
        assert_eq!(phase_for_code("matrix.empty"), "matrix");
    }

    #[test]
    fn only_the_first_document_reaches_stdout() {
        reset_emit_state();
        assert!(!json_emitted());
        assert!(emit_to_stdout(), "first document goes to stdout");
        assert!(json_emitted());
        assert!(!emit_to_stdout(), "later documents must go to the stderr bypass");
        assert!(!emit_to_stdout());
        reset_emit_state();
        assert!(emit_to_stdout(), "state resets for the next process/test");
        reset_emit_state();
    }

    #[test]
    fn utc_timestamp_formatting_is_correct() {
        // 2026-09-25T10:22:33Z == 1790331753
        let (y, mo, d, h, mi, s) = utc_parts(1_790_331_753);
        assert_eq!((y, mo, d, h, mi, s), (2026, 9, 25, 10, 22, 33));
        // Epoch + leap-day sanity.
        assert_eq!(utc_parts(0), (1970, 1, 1, 0, 0, 0));
        assert_eq!(utc_parts(951_782_400), (2000, 2, 29, 0, 0, 0));
    }

    #[test]
    fn error_entry_shape() {
        let err = crate::error::cant_create("目录已存在且非空: /tmp/x").with_hint("--output <other>");
        let entry = error_entry(&err);
        assert_eq!(entry["code"], "fs.cannot_create");
        assert_eq!(entry["phase"], "write");
        assert_eq!(entry["hint"], "--output <other>");
    }

    /// The `--json` error path is one parseable document with the same exit code
    /// as the human path (spec §11.5: machine decision is `exit_code` only).
    #[test]
    fn error_document_is_one_parseable_document() {
        set_command("init");
        let err = crate::error::usage("未知平台 `nukkit`").with_hint("复现: vinoa init --help");
        let doc = error_document(&err);
        let text = serde_json::to_string(&doc).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed["schema"], JSON_SCHEMA);
        assert_eq!(parsed["ok"], false);
        assert_eq!(parsed["command"], "init");
        assert_eq!(parsed["phase"], "input");
        assert_eq!(parsed["status"], "usage_error");
        assert_eq!(parsed["exit_code"], 64);
        assert_eq!(parsed["errors"][0]["code"], "usage.invalid");
        assert_eq!(parsed["errors"][0]["severity"], "error");

        let verify = error_document(&Error::new("verify_failed", 2, "Gradle build failed (exit 1)"));
        assert_eq!(verify["phase"], "verify");
        assert_eq!(verify["status"], "verify_failed");
        assert_eq!(verify["exit_code"], 2);
    }
}
