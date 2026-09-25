//! git init + first commit (spec §9.7). OWNER: cli-dev.
//!
//! Runs **after** the atomic write succeeded. Never configures `user.name` /
//! `user.email`: when the identity is missing we skip the commit and warn instead
//! of touching global config. A missing `git` binary is likewise a warning, not a
//! scaffold failure — the generated tree is already on disk.
use crate::error::Result;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Outcome for the `--json` `git` object (spec §11.5).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GitOutcome {
    pub initialized: bool,
    pub committed: bool,
    pub branch: Option<String>,
    pub warnings: Vec<String>,
}

impl GitOutcome {
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "initialized": self.initialized,
            "committed": self.committed,
            "branch": self.branch,
        })
    }
}

/// `git init -b <branch>` + `git add -A` + one commit (spec §9.7).
///
/// Already inside a repository → no nested `git init`; only a warning.
pub fn init_and_commit(dir: &Path, branch: &str, message: &str) -> Result<()> {
    let outcome = init_and_commit_report(dir, branch, message)?;
    for warning in &outcome.warnings {
        crate::report::warn(warning);
    }
    Ok(())
}

/// Same as [`init_and_commit`] but returns what actually happened.
pub fn init_and_commit_report(dir: &Path, branch: &str, message: &str) -> Result<GitOutcome> {
    let mut outcome = GitOutcome { branch: Some(branch.to_string()), ..Default::default() };

    if inside_repo(dir) {
        outcome.warnings.push(format!(
            "已在 git 仓库内，跳过 git init（不重复初始化）: {}",
            dir.display()
        ));
        return Ok(outcome);
    }
    if !git_available() {
        outcome
            .warnings
            .push("未找到 git 可执行文件，跳过 git init + 首次提交".to_string());
        return Ok(outcome);
    }

    // `git init -b main`; older git falls back to init + symbolic-ref (spec §9.7).
    let init = run_git(dir, &["init", "-b", branch]);
    let init_ok = match init {
        Ok(_) => true,
        Err(err) => {
            let fallback = run_git(dir, &["init"]);
            match fallback {
                Ok(_) => {
                    let _ = run_git(dir, &["symbolic-ref", "HEAD", &format!("refs/heads/{branch}")]);
                    true
                }
                Err(_) => {
                    outcome.warnings.push(format!("git init 失败，跳过首次提交: {err}"));
                    false
                }
            }
        }
    };
    if !init_ok {
        return Ok(outcome);
    }
    outcome.initialized = true;
    // Fresh repos default to the branch we asked for; read it back for `--json`.
    if let Ok(out) = run_git(dir, &["symbolic-ref", "--short", "HEAD"]) {
        let name = out.trim().to_string();
        if !name.is_empty() {
            outcome.branch = Some(name);
        }
    }

    if let Err(err) = run_git(dir, &["add", "-A"]) {
        outcome.warnings.push(format!("git add -A 失败，跳过首次提交: {err}"));
        return Ok(outcome);
    }

    if !has_identity(dir) {
        outcome.warnings.push(
            "git 缺少 user.name / user.email，已跳过首次提交（不代配身份）；\
             配置后可在工程内执行 git commit -m \"chore: scaffold <project> with vinoa\""
                .to_string(),
        );
        return Ok(outcome);
    }

    match run_git(dir, &["commit", "-m", message]) {
        Ok(_) => outcome.committed = true,
        Err(err) => outcome.warnings.push(format!("git 首次提交失败: {err}")),
    }
    Ok(outcome)
}

/// True when `dir` or any ancestor already holds a `.git` entry.
pub fn inside_repo(dir: &Path) -> bool {
    let start = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
    let mut current: Option<&Path> = Some(start.as_path());
    while let Some(path) = current {
        if path.join(".git").exists() {
            return true;
        }
        current = path.parent();
    }
    false
}

/// `git config --get user.name` and `user.email` must both be non-empty.
pub fn has_identity(dir: &Path) -> bool {
    for key in ["user.name", "user.email"] {
        match run_git(dir, &["config", "--get", key]) {
            Ok(value) if !value.trim().is_empty() => {}
            _ => return false,
        }
    }
    true
}

/// The branch-selection argv for a given branch (kept pure for tests).
pub fn init_argv(branch: &str) -> Vec<String> {
    vec!["init".into(), "-b".into(), branch.to_string()]
}

fn git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn run_git(dir: &Path, args: &[&str]) -> std::result::Result<String, String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .map_err(|e| format!("git {}: {e}", args.join(" ")))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(if stderr.is_empty() {
            format!("git {} 退出码 {:?}", args.join(" "), output.status.code())
        } else {
            format!("git {}: {stderr}", args.join(" "))
        })
    }
}

/// First-commit message (spec §9.7).
pub fn commit_message(project: &str) -> String {
    format!("chore: scaffold {project} with vinoa")
}

/// Repository root of `dir`, if it is inside one (used by the orchestrator's
/// "already inside a repo" summary line).
pub fn repo_root(dir: &Path) -> Option<PathBuf> {
    let start = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
    let mut current: Option<&Path> = Some(start.as_path());
    while let Some(path) = current {
        if path.join(".git").exists() {
            return Some(path.to_path_buf());
        }
        current = path.parent();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inside_repo_detects_git_dir_and_ancestors() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(!inside_repo(tmp.path()));
        std::fs::create_dir_all(tmp.path().join("nested/deep")).unwrap();
        assert!(!inside_repo(&tmp.path().join("nested/deep")));
        std::fs::create_dir(tmp.path().join(".git")).unwrap();
        assert!(inside_repo(tmp.path()));
        assert!(inside_repo(&tmp.path().join("nested/deep")));
        assert_eq!(repo_root(&tmp.path().join("nested/deep")).as_deref(), Some(
            std::fs::canonicalize(tmp.path()).unwrap().as_path()
        ));
    }

    #[test]
    fn commit_message_matches_the_spec() {
        assert_eq!(commit_message("my-plugin"), "chore: scaffold my-plugin with vinoa");
    }

    #[test]
    fn init_argv_uses_b_flag_with_main() {
        assert_eq!(init_argv("main"), vec!["init", "-b", "main"]);
    }

    #[test]
    fn outcome_json_shape() {
        let outcome = GitOutcome {
            initialized: true,
            committed: true,
            branch: Some("main".into()),
            warnings: vec![],
        };
        let json = outcome.to_json();
        assert_eq!(json["initialized"], true);
        assert_eq!(json["branch"], "main");
    }
}
