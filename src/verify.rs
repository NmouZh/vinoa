//! `--verify`: run the generated project's build (spec §11.4). OWNER: cli-dev.
//!
//! Opt-in only. Runs `<target>/gradlew build` (Windows: `gradlew.bat build` via
//! `cmd`), offline by default, with a 600 s timeout (cap 1800 s). On failure the
//! project is **kept** — no rollback, no state file — and the caller maps the
//! outcome to `EXIT_VERIFY_FAILED (2)`.
use crate::error::{Error, Result, EXIT_VERIFY_FAILED};
use crate::types::Plan;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// Default timeout (spec §11.4).
pub const DEFAULT_TIMEOUT_S: u64 = 600;
/// Hard cap, even when `VINOA_VERIFY_TIMEOUT` asks for more.
pub const MAX_TIMEOUT_S: u64 = 1800;
/// Default `--verify-tail`.
pub const DEFAULT_TAIL: usize = 20;
/// `--verify-tail` upper bound.
pub const MAX_TAIL: usize = 200;
/// Environment override for the timeout.
pub const TIMEOUT_ENV: &str = "VINOA_VERIFY_TIMEOUT";

#[derive(Debug, Clone)]
pub struct VerifyOptions {
    /// `--verify-online`: drop `--offline` (default is offline).
    pub online: bool,
    /// `--verify-tail <N>`, 20 by default, capped at 200.
    pub tail: usize,
    /// Timeout in seconds, 600 by default, capped at 1800.
    pub timeout_s: u64,
}

impl Default for VerifyOptions {
    fn default() -> Self {
        Self { online: false, tail: DEFAULT_TAIL, timeout_s: DEFAULT_TIMEOUT_S }
    }
}

impl VerifyOptions {
    /// Apply the env override (only when set) and clamp both limits (spec §11.4).
    pub fn resolved(mut self) -> Self {
        if let Ok(raw) = std::env::var(TIMEOUT_ENV) {
            self.timeout_s = resolve_timeout(Some(&raw));
        }
        self.timeout_s = self.timeout_s.clamp(1, MAX_TIMEOUT_S);
        self.tail = self.tail.clamp(1, MAX_TAIL);
        self
    }
}

#[derive(Debug, Clone, Default)]
pub struct VerifyOutcome {
    pub ok: bool,
    /// Workspace-relative log path, e.g. `.vinoa/verify/verify-20260925T102233Z.log`.
    pub log_path: String,
    /// `network_unavailable` | `build_failed` | `timeout`.
    pub reason: Option<String>,
    /// Process exit code when the build ran to completion.
    pub exit_code: Option<i32>,
    /// Full command line for the report / `--json`.
    pub command: String,
    pub timeout_s: u64,
    pub duration_ms: u64,
    /// Last `--verify-tail` non-empty lines of the log.
    pub tail: Vec<String>,
    /// Absolute path of the log file.
    pub log_abs: PathBuf,
}

impl VerifyOutcome {
    /// Spec §11.5 `verify` object.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "ran": true,
            "ok": self.ok,
            "reason": self.reason,
            "command": self.command,
            "timeout_s": self.timeout_s,
            "duration_ms": self.duration_ms,
            "log_path": self.log_path,
            "exit_code": self.exit_code,
            "tail": self.tail,
        })
    }

    /// Spec §11.5: `--verify` not requested.
    pub fn not_run_json() -> serde_json::Value {
        serde_json::json!({ "ran": false, "ok": serde_json::Value::Null, "reason": serde_json::Value::Null })
    }

    /// Map a failed verification onto the frozen error contract.
    pub fn into_error(self, root: &Path) -> Option<Error> {
        if self.ok {
            return None;
        }
        let reason = self.reason.as_deref().unwrap_or("build_failed");
        let message = match reason {
            "timeout" => format!("Gradle build 超时（{}s）", self.timeout_s),
            _ => format!(
                "Gradle build 失败（exit {}）",
                self.exit_code.map(|c| c.to_string()).unwrap_or_else(|| "?".into())
            ),
        };
        Some(
            Error::new("verify_failed", EXIT_VERIFY_FAILED, message)
                .with_hint(format!(
                    "工程保留在 {}\n  日志: {}\n  复现: cd {} && ./gradlew build",
                    root.display(),
                    self.log_path,
                    root.display()
                )),
        )
    }
}

/// Thin wrapper kept for the orchestrator's existing call site.
pub fn run(plan: &Plan, online: bool) -> Result<VerifyOutcome> {
    run_with(plan, &VerifyOptions { online, ..VerifyOptions::default() })
}

/// Run the build with explicit options.
pub fn run_with(plan: &Plan, options: &VerifyOptions) -> Result<VerifyOutcome> {
    let options = options.clone().resolved();
    let root = &plan.root;
    let command = build_command(options.online);
    let display = command.command_line();

    // `<target>/.vinoa/verify/verify-<UTC>.log` — always written, pass or fail.
    let rel_log = PathBuf::from(".vinoa")
        .join("verify")
        .join(format!("verify-{}.log", crate::report::utc_stamp_compact()));
    let log_abs = root.join(&rel_log);
    if let Some(dir) = log_abs.parent() {
        std::fs::create_dir_all(dir).map_err(|e| {
            Error::new(
                "verify_failed",
                EXIT_VERIFY_FAILED,
                format!("无法创建验证日志目录 {}: {e}", dir.display()),
            )
        })?;
    }
    let log_file = std::fs::File::create(&log_abs).map_err(|e| {
        Error::new(
            "verify_failed",
            EXIT_VERIFY_FAILED,
            format!("无法写入验证日志 {}: {e}", log_abs.display()),
        )
    })?;

    // Missing wrapper: a generation defect, reported through the same exit code 2
    // path so the project stays for inspection.
    let wrapper = root.join(command.wrapper);
    if !wrapper.is_file() {
        let mut log = log_file;
        let _ = writeln!(log, "缺少构建包装器: {}", wrapper.display());
        let _ = log.flush();
        return Ok(VerifyOutcome {
            ok: false,
            log_path: rel_log.to_string_lossy().to_string(),
            reason: Some("build_failed".into()),
            exit_code: None,
            command: display,
            timeout_s: options.timeout_s,
            duration_ms: 0,
            tail: vec![format!("缺少构建包装器: {}", wrapper.display())],
            log_abs,
        });
    }

    let started = Instant::now();
    let mut child = spawn(&command, root, &log_abs)?;

    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => {}
            Err(e) => {
                kill_tree(&mut child);
                return Err(Error::new(
                    "verify_failed",
                    EXIT_VERIFY_FAILED,
                    format!("无法等待 Gradle 进程: {e}"),
                ));
            }
        }
        if started.elapsed() >= Duration::from_secs(options.timeout_s) {
            kill_tree(&mut child);
            break None;
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    let duration_ms = started.elapsed().as_millis() as u64;
    let text = std::fs::read_to_string(&log_abs).unwrap_or_default();
    let tail = tail_lines(&text, options.tail);

    let (ok, reason, exit_code) = match status {
        Some(status) if status.success() => (true, None, Some(0)),
        Some(status) => (
            false,
            Some(classify(&text).to_string()),
            status.code(),
        ),
        None => (false, Some("timeout".to_string()), None),
    };

    Ok(VerifyOutcome {
        ok,
        log_path: rel_log.to_string_lossy().to_string(),
        reason,
        exit_code,
        command: display,
        timeout_s: options.timeout_s,
        duration_ms,
        tail,
        log_abs,
    })
}

/// Everything needed to spawn the build, platform-specific (spec §11.4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildCommand {
    pub program: String,
    pub args: Vec<String>,
    /// Wrapper file that must exist inside the target.
    pub wrapper: &'static str,
    pub display: String,
}

impl BuildCommand {
    /// Command line as shown to the user / recorded in `--json`.
    pub fn command_line(&self) -> String {
        self.display.clone()
    }
}

/// Linux/macOS `./gradlew build …`; Windows `cmd /C gradlew.bat build …`.
pub fn build_command(online: bool) -> BuildCommand {
    let mut flags = vec!["build".to_string()];
    if !online {
        flags.push("--offline".to_string());
    }
    flags.push("--no-daemon".to_string());
    flags.push("--console=plain".to_string());
    flags.push("--stacktrace".to_string());

    if cfg!(windows) {
        let mut args = vec!["/C".to_string(), "gradlew.bat".to_string()];
        args.extend(flags.iter().cloned());
        BuildCommand {
            program: "cmd".to_string(),
            args,
            wrapper: "gradlew.bat",
            display: format!("gradlew.bat {}", flags.join(" ")),
        }
    } else {
        BuildCommand {
            program: "./gradlew".to_string(),
            args: flags.clone(),
            wrapper: "gradlew",
            display: format!("./gradlew {}", flags.join(" ")),
        }
    }
}

fn spawn(command: &BuildCommand, root: &Path, log_abs: &Path) -> Result<Child> {
    let log = std::fs::OpenOptions::new()
        .append(true)
        .open(log_abs)
        .map_err(|e| {
            Error::new(
                "verify_failed",
                EXIT_VERIFY_FAILED,
                format!("无法打开验证日志 {}: {e}", log_abs.display()),
            )
        })?;
    let err_log = log.try_clone().map_err(|e| {
        Error::new("verify_failed", EXIT_VERIFY_FAILED, format!("无法复制日志句柄: {e}"))
    })?;

    let mut cmd = Command::new(&command.program);
    cmd.args(&command.args)
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(err_log));
    // Put the build in its own process group so a timeout can kill the tree.
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    cmd.spawn().map_err(|e| {
        Error::new(
            "verify_failed",
            EXIT_VERIFY_FAILED,
            format!("无法启动 {}: {e}", command.program),
        )
        .with_hint("工程已保留；修好后可手动执行 ./gradlew build")
    })
}

/// Kill the whole process tree (best effort, no extra dependencies).
fn kill_tree(child: &mut Child) {
    let pid = child.id();
    #[cfg(unix)]
    {
        // The child is its own process group leader (`process_group(0)`).
        let _ = Command::new("kill")
            .arg("-KILL")
            .arg(format!("-{pid}"))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    #[cfg(windows)]
    {
        let _ = Command::new("taskkill")
            .args(["/F", "/T", "/PID", &pid.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    let _ = child.kill();
    let _ = child.wait();
}

/// `network_unavailable` vs `build_failed` (spec §11.4 failure classification).
pub fn classify(log: &str) -> &'static str {
    let lower = log.to_ascii_lowercase();
    const NETWORK: [&str; 8] = [
        "could not resolve",
        "could not get resource",
        "could not download",
        "connection refused",
        "connection timed out",
        "unknownhost",
        "unknown host",
        "network is unreachable",
    ];
    if NETWORK.iter().any(|needle| lower.contains(needle)) {
        "network_unavailable"
    } else {
        "build_failed"
    }
}

/// Timeout resolution: `VINOA_VERIFY_TIMEOUT` override, 600 s default, 1800 s cap.
pub fn resolve_timeout(raw: Option<&str>) -> u64 {
    match raw.map(str::trim).and_then(|v| v.parse::<u64>().ok()) {
        Some(0) | None => DEFAULT_TIMEOUT_S,
        Some(v) => v.min(MAX_TIMEOUT_S),
    }
}

/// Last `n` non-empty lines (trailing newline ignored).
pub fn tail_lines(text: &str, n: usize) -> Vec<String> {
    let mut lines: Vec<String> = text
        .lines()
        .map(|l| l.trim_end().to_string())
        .filter(|l| !l.is_empty())
        .collect();
    if lines.len() > n {
        lines = lines.split_off(lines.len() - n);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Plan, PlannedAction};

    fn plan_at(root: PathBuf) -> Plan {
        Plan {
            root,
            files: Vec::new(),
            actions: vec![PlannedAction::Verify],
            warnings: Vec::new(),
            skipped: Vec::new(),
        }
    }

    #[test]
    fn offline_is_the_default_and_online_drops_the_flag() {
        let offline = build_command(false);
        assert!(offline.args.contains(&"--offline".to_string()));
        assert!(offline.args.contains(&"--no-daemon".to_string()));
        assert!(offline.args.contains(&"--console=plain".to_string()));
        assert!(offline.args.contains(&"--stacktrace".to_string()));
        assert_eq!(offline.args[0], "build");
        assert!(offline.display.contains("--offline"));

        let online = build_command(true);
        assert!(!online.args.contains(&"--offline".to_string()));
        if cfg!(windows) {
            assert_eq!(online.program, "cmd");
            assert_eq!(online.wrapper, "gradlew.bat");
        } else {
            assert_eq!(online.program, "./gradlew");
            assert_eq!(online.wrapper, "gradlew");
        }
    }

    #[test]
    fn timeout_resolution_honours_env_default_and_cap() {
        assert_eq!(resolve_timeout(None), 600);
        assert_eq!(resolve_timeout(Some("abc")), 600);
        assert_eq!(resolve_timeout(Some("0")), 600);
        assert_eq!(resolve_timeout(Some("120")), 120);
        assert_eq!(resolve_timeout(Some("99999")), MAX_TIMEOUT_S);
        assert_eq!(MAX_TIMEOUT_S, 1800);
    }

    #[test]
    fn options_clamp_tail_and_timeout() {
        let options = VerifyOptions { online: true, tail: 9999, timeout_s: 1 }.resolved();
        assert_eq!(options.tail, MAX_TAIL);
        assert_eq!(options.timeout_s, 1); // explicit value kept when env is absent
        let capped = VerifyOptions { timeout_s: 99_999, ..VerifyOptions::default() }.resolved();
        assert_eq!(capped.timeout_s, MAX_TIMEOUT_S);
        let floored = VerifyOptions { tail: 0, ..VerifyOptions::default() }.resolved();
        assert_eq!(floored.tail, 1);
    }

    #[test]
    fn tail_keeps_the_last_lines_only() {
        let text = "a\n\nb\nc\nd\n";
        assert_eq!(tail_lines(text, 2), vec!["c", "d"]);
        assert_eq!(tail_lines(text, 99).len(), 4);
        assert!(tail_lines("", 5).is_empty());
    }

    #[test]
    fn classification_splits_network_from_build_failures() {
        assert_eq!(classify("> Task :core:compileJava FAILED\ncannot find symbol"), "build_failed");
        assert_eq!(
            classify("Could not resolve org.spigotmc:spigot-api:1.8.8-R0.1-SNAPSHOT."),
            "network_unavailable"
        );
        assert_eq!(classify("Connection refused"), "network_unavailable");
        assert_eq!(classify(""), "build_failed");
    }

    #[test]
    fn not_run_json_matches_the_spec_shape() {
        let json = VerifyOutcome::not_run_json();
        assert_eq!(json["ran"], false);
        assert_eq!(json["ok"], serde_json::Value::Null);
        assert_eq!(json["reason"], serde_json::Value::Null);
    }

    #[test]
    fn failure_maps_to_exit_2_and_keeps_the_project() {
        let root = PathBuf::from("/tmp/my-plugin");
        let outcome = VerifyOutcome {
            ok: false,
            log_path: ".vinoa/verify/verify-20260925T102233Z.log".into(),
            reason: Some("build_failed".into()),
            exit_code: Some(1),
            command: "./gradlew build --offline --no-daemon".into(),
            timeout_s: 600,
            duration_ms: 91_234,
            tail: vec!["> Task :platforms:paper:compileJava FAILED".into()],
            log_abs: root.join(".vinoa/verify/x.log"),
        };
        let json = outcome.clone().to_json();
        assert_eq!(json["ran"], true);
        assert_eq!(json["reason"], "build_failed");
        assert_eq!(json["timeout_s"], 600);
        assert_eq!(json["tail"][0], "> Task :platforms:paper:compileJava FAILED");

        let err = outcome.into_error(&root).unwrap();
        assert_eq!(err.code, "verify_failed");
        assert_eq!(err.exit_code(), 2);
        assert!(err.hint.unwrap().contains("工程保留"));

        // A successful outcome maps to no error at all.
        let ok = VerifyOutcome { ok: true, reason: None, ..Default::default() };
        assert!(ok.into_error(&root).is_none());
    }

    #[test]
    fn missing_wrapper_is_a_build_failure_with_a_log() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("proj");
        std::fs::create_dir_all(&root).unwrap();
        let outcome = run_with(&plan_at(root.clone()), &VerifyOptions::default()).unwrap();
        assert!(!outcome.ok);
        assert_eq!(outcome.reason.as_deref(), Some("build_failed"));
        assert!(outcome.log_path.starts_with(".vinoa/verify/verify-"));
        assert!(outcome.log_abs.is_file(), "log must always be written");
        assert!(outcome.tail[0].contains("缺少构建包装器"));
        assert!(!outcome.into_error(&root.clone()).is_none());
    }

    #[cfg(unix)]
    fn write_fake_gradlew(root: &Path, body: &str) {
        use std::os::unix::fs::PermissionsExt;
        std::fs::create_dir_all(root).unwrap();
        let path = root.join("gradlew");
        std::fs::write(&path, body).unwrap();
        let mut perm = std::fs::metadata(&path).unwrap().permissions();
        perm.set_mode(0o755);
        std::fs::set_permissions(&path, perm).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn failing_build_is_reported_with_log_and_tail() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("proj");
        write_fake_gradlew(
            &root,
            "#!/bin/sh\necho 'FAILURE: Build failed'\necho '> Task :core:compileJava FAILED'\nexit 3\n",
        );
        let outcome = run_with(&plan_at(root.clone()), &VerifyOptions::default()).unwrap();
        assert!(!outcome.ok);
        assert_eq!(outcome.exit_code, Some(3));
        assert_eq!(outcome.reason.as_deref(), Some("build_failed"));
        assert!(outcome.tail.iter().any(|l| l.contains("compileJava FAILED")));
        assert!(
            std::fs::metadata(&outcome.log_abs).unwrap().len() > 0,
            "log must contain the build output"
        );
        // The project is kept: the wrapper still exists.
        assert!(root.join("gradlew").is_file());
        assert!(outcome.log_abs.starts_with(root.join(".vinoa")));
    }

    #[cfg(unix)]
    #[test]
    fn successful_build_reports_ok() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("proj");
        write_fake_gradlew(&root, "#!/bin/sh\necho BUILD SUCCESSFUL\nexit 0\n");
        let outcome = run_with(&plan_at(root.clone()), &VerifyOptions::default()).unwrap();
        assert!(outcome.ok);
        assert_eq!(outcome.reason, None);
        assert_eq!(outcome.exit_code, Some(0));
        assert!(outcome.into_error(&root).is_none());
    }

    #[cfg(unix)]
    #[test]
    fn timeout_kills_the_build_and_is_classified_as_timeout() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("proj");
        write_fake_gradlew(&root, "#!/bin/sh\necho starting\nsleep 30\necho never\n");
        let options = VerifyOptions { timeout_s: 1, ..VerifyOptions::default() };
        let started = Instant::now();
        let outcome = run_with(&plan_at(root.clone()), &options).unwrap();
        assert!(!outcome.ok);
        assert_eq!(outcome.reason.as_deref(), Some("timeout"));
        assert_eq!(outcome.exit_code, None);
        assert!(started.elapsed() < Duration::from_secs(25), "must not wait for sleep 30");
        let err = outcome.into_error(&root).unwrap();
        assert_eq!(err.exit_code(), 2);
        assert!(err.message.contains("超时"));
    }

    #[cfg(unix)]
    #[test]
    fn network_failure_is_classified_separately() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("proj");
        write_fake_gradlew(
            &root,
            "#!/bin/sh\necho 'Could not resolve org.spigotmc:spigot-api:1.8.8-R0.1-SNAPSHOT.'\nexit 1\n",
        );
        let outcome = run_with(&plan_at(root.clone()), &VerifyOptions::default()).unwrap();
        assert!(!outcome.ok);
        assert_eq!(outcome.reason.as_deref(), Some("network_unavailable"));
    }

    #[cfg(unix)]
    #[test]
    fn online_option_omits_the_offline_flag_in_the_recorded_command() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("proj");
        write_fake_gradlew(&root, "#!/bin/sh\necho ok\nexit 0\n");
        let options = VerifyOptions { online: true, ..VerifyOptions::default() };
        let outcome = run_with(&plan_at(root), &options).unwrap();
        assert!(!outcome.command.contains("--offline"));
        assert!(outcome.command.contains("--no-daemon"));
    }
}
