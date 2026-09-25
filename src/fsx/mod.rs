//! Atomic write + git init (spec §5 phases 6-7). OWNER: cli-dev.
//!
//! Phase 6 contract: the whole plan is staged in a sibling temp directory and
//! moved into place with a single `rename`; any failure leaves **no residue**.
pub mod git;

use crate::error::{Error, Result, EXIT_CANTCREAT, EXIT_IO};
use crate::types::Plan;
use std::path::{Path, PathBuf};

/// Refuse to write when the target directory exists and is not empty (spec §11.1).
///
/// The message must say what happened, what to do next and how to reproduce it.
pub fn ensure_target_writable(dir: &Path) -> Result<()> {
    if dir.exists() {
        if !dir.is_dir() {
            return Err(Error::new(
                "fs.cannot_create",
                EXIT_CANTCREAT,
                format!("目标路径已存在且不是目录: {}", dir.display()),
            )
            .with_hint(format!(
                "处置: --output <other-dir> 换一个位置 | 复现: vinoa init {} --output <other-dir> -y",
                dir.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()
            )));
        }
        let conflicts = dir_conflicts(dir);
        if !conflicts.is_empty() {
            let shown: Vec<String> = conflicts.iter().take(10).cloned().collect();
            let more = if conflicts.len() > shown.len() {
                format!(" …（共 {} 项）", conflicts.len())
            } else {
                String::new()
            };
            return Err(Error::new(
                "fs.cannot_create",
                EXIT_CANTCREAT,
                format!(
                    "目标目录已存在且非空: {}\n  冲突项（前 10 条）: {}{more}",
                    dir.display(),
                    shown.join(", ")
                ),
            )
            .with_hint(format!(
                "处置: --output <other-dir> 换空目录 | cd <dir> && vinoa init 确认就地初始化（仍逐文件询问）\n  复现: vinoa init <名称> --output {} -y",
                dir.with_extension("2").display()
            )));
        }
        probe_writable(dir)?;
    } else {
        let parent = writable_parent(dir);
        if !parent.is_dir() {
            return Err(Error::new(
                "fs.cannot_create",
                EXIT_CANTCREAT,
                format!("上级目录不存在: {}", parent.display()),
            )
            .with_hint("处置: 先创建上级目录或改用 --output <dir>"));
        }
        probe_writable(&parent)?;
    }
    Ok(())
}

/// Directory entries that would conflict, sorted, directories marked with `/`.
pub fn dir_conflicts(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = match std::fs::read_dir(dir) {
        Ok(entries) => entries
            .flatten()
            .map(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
                if is_dir { format!("{name}/") } else { name }
            })
            .collect(),
        Err(_) => Vec::new(),
    };
    names.sort();
    names
}

/// Write the whole plan atomically: temp dir next to the target, then rename.
///
/// Steps: re-validate the target → stage every file in a sibling temp dir →
/// `rename` into place. Any error drops the temp dir, so the workspace keeps no
/// half-written project (spec §5 phase 6).
pub fn write_atomic(plan: &Plan) -> Result<()> {
    ensure_target_writable(&plan.root)?;

    let root = &plan.root;
    let parent = writable_parent(root);
    if !parent.is_dir() {
        std::fs::create_dir_all(&parent)
            .map_err(|e| io_err(&format!("无法创建上级目录 {}", parent.display()), e))?;
    }

    let staging = tempfile::Builder::new()
        .prefix(".vinoa-staging-")
        .tempdir_in(parent)
        .map_err(|e| io_err("无法创建临时目录", e))?;

    for file in &plan.files {
        let dest = join_checked(staging.path(), &file.path)?;
        if let Some(dir) = dest.parent() {
            std::fs::create_dir_all(dir)
                .map_err(|e| io_err(&format!("无法创建目录 {}", dir.display()), e))?;
        }
        std::fs::write(&dest, &file.content)
            .map_err(|e| io_err(&format!("无法写入 {}", file.path), e))?;
        if file.executable {
            set_executable(&dest)?;
        }
    }

    // The target may exist as an *empty* directory (spec §11.3 only rejects
    // non-empty ones). POSIX `rename` can replace an empty dir; Windows cannot,
    // so remove it first.
    if root.exists() {
        std::fs::remove_dir(root)
            .map_err(|e| io_err(&format!("无法清理空目录 {}", root.display()), e))?;
    }

    let staged = staging.keep();
    if let Err(e) = std::fs::rename(&staged, root) {
        // Roll back: never leave the staged tree behind.
        let _ = std::fs::remove_dir_all(&staged);
        return Err(io_err(
            &format!("落盘失败，已回滚（{} 未创建）", root.display()),
            e,
        ));
    }
    Ok(())
}

/// Join a plan-relative file path onto `base`, refusing escapes (defence in depth;
/// the engine already validates `template.bad_target`).
pub fn join_checked(base: &Path, rel: &str) -> Result<PathBuf> {
    if rel.is_empty() || Path::new(rel).is_absolute() || rel.starts_with('/') || rel.starts_with('\\')
    {
        return Err(Error::new(
            "write.io",
            EXIT_IO,
            format!("计划中的文件路径非法或越界: {rel}"),
        ));
    }
    let mut out = base.to_path_buf();
    for segment in rel.split(['/', '\\']) {
        if segment.is_empty() {
            continue;
        }
        if segment == "." || segment == ".." || segment.contains(':') {
            return Err(Error::new(
                "write.io",
                EXIT_IO,
                format!("计划中的文件路径非法或越界: {rel}"),
            ));
        }
        out.push(segment);
    }
    Ok(out)
}

/// Parent directory that the staged tree is created in; `.` for bare names.
pub fn writable_parent(dir: &Path) -> PathBuf {
    match dir.parent() {
        Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
        _ => PathBuf::from("."),
    }
}

fn probe_writable(dir: &Path) -> Result<()> {
    tempfile::Builder::new()
        .prefix(".vinoa-probe-")
        .tempfile_in(dir)
        .map(|_| ())
        .map_err(|e| {
            Error::new(
                "fs.cannot_create",
                EXIT_CANTCREAT,
                format!("目录不可写: {}（{e}）", dir.display()),
            )
            .with_hint("处置: 换一个可写目录（--output <dir>）或用就地初始化")
        })
}

fn set_executable(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perm = std::fs::metadata(path)
            .map_err(|e| io_err(&format!("无法读取权限 {}", path.display()), e))?
            .permissions();
        perm.set_mode(0o755);
        std::fs::set_permissions(path, perm)
            .map_err(|e| io_err(&format!("无法设置可执行位 {}", path.display()), e))?;
    }
    #[cfg(not(unix))]
    {
        let _ = path; // Windows ignores the executable bit.
    }
    Ok(())
}

fn io_err(context: &str, err: std::io::Error) -> Error {
    Error::new("write.io", EXIT_IO, format!("{context}: {err}"))
        .with_hint("已回滚，未留下半个工程；修好后重跑同一命令")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{PlannedFile, Plan};

    fn plan_at(root: PathBuf, files: &[(&str, &str)]) -> Plan {
        Plan {
            root,
            files: files
                .iter()
                .map(|(p, c)| PlannedFile {
                    path: (*p).to_string(),
                    content: c.as_bytes().to_vec(),
                    executable: p.ends_with("gradlew"),
                })
                .collect(),
            actions: Vec::new(),
            warnings: Vec::new(),
            skipped: Vec::new(),
        }
    }

    /// Acceptance: non-empty target → `fs.cannot_create` / exit 73.
    #[test]
    fn non_empty_directory_is_refused_with_conflicts() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("my-plugin");
        std::fs::create_dir_all(target.join("src")).unwrap();
        std::fs::write(target.join("pom.xml"), b"<project/>").unwrap();

        let err = ensure_target_writable(&target).unwrap_err();
        assert_eq!(err.code, "fs.cannot_create");
        assert_eq!(err.exit_code(), EXIT_CANTCREAT);
        assert_eq!(err.exit_code(), 73);
        assert!(err.message.contains("目标目录已存在且非空"), "{}", err.message);
        assert!(err.message.contains("pom.xml"), "{}", err.message);
        assert!(err.message.contains("src/"), "{}", err.message);
        let hint = err.hint.unwrap();
        assert!(hint.contains("--output"), "{hint}");

        // Nothing may be written when the target is refused.
        let plan = plan_at(target.clone(), &[("settings.gradle.kts", "x")]);
        let err = write_atomic(&plan).unwrap_err();
        assert_eq!(err.exit_code(), 73);
        assert!(!target.join("settings.gradle.kts").exists());
    }

    #[test]
    fn empty_directory_is_accepted_and_probed() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("my-plugin");
        std::fs::create_dir(&target).unwrap();
        ensure_target_writable(&target).unwrap();
    }

    #[test]
    fn missing_target_is_accepted_when_parent_exists() {
        let tmp = tempfile::tempdir().unwrap();
        ensure_target_writable(&tmp.path().join("new-project")).unwrap();
    }

    #[test]
    fn missing_parent_is_a_cant_create_error() {
        let tmp = tempfile::tempdir().unwrap();
        let err = ensure_target_writable(&tmp.path().join("a/b/c")).unwrap_err();
        assert_eq!(err.code, "fs.cannot_create");
        assert_eq!(err.exit_code(), EXIT_CANTCREAT);
    }

    #[test]
    fn file_where_directory_is_expected_is_refused() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("not-a-dir");
        std::fs::write(&file, b"x").unwrap();
        let err = ensure_target_writable(&file).unwrap_err();
        assert_eq!(err.code, "fs.cannot_create");
    }

    #[test]
    fn write_atomic_creates_the_whole_tree_without_residue() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("my-plugin");
        let plan = plan_at(
            target.clone(),
            &[
                ("settings.gradle.kts", "rootProject.name = \"my-plugin\"\n"),
                ("core/src/main/java/A.java", "class A {}\n"),
                ("gradlew", "#!/bin/sh\n"),
            ],
        );
        write_atomic(&plan).unwrap();
        assert!(target.join("settings.gradle.kts").is_file());
        assert!(target.join("core/src/main/java/A.java").is_file());
        assert!(target.join("gradlew").is_file());

        // No staging directory left next to the target.
        let leftovers: Vec<String> = std::fs::read_dir(tmp.path())
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|n| n.starts_with(".vinoa-staging-"))
            .collect();
        assert!(leftovers.is_empty(), "staging residue: {leftovers:?}");
    }

    #[cfg(unix)]
    #[test]
    fn write_atomic_sets_the_executable_bit_on_unix() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("p");
        write_atomic(&plan_at(target.clone(), &[("gradlew", "#!/bin/sh\n"), ("README.md", "x")])).unwrap();
        let mode = std::fs::metadata(target.join("gradlew")).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o755);
        let other = std::fs::metadata(target.join("README.md")).unwrap().permissions().mode();
        assert_ne!(other & 0o111, 0o111);
    }

    #[test]
    fn write_atomic_replaces_an_empty_target_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("p");
        std::fs::create_dir(&target).unwrap();
        write_atomic(&plan_at(target.clone(), &[("README.md", "hi")])).unwrap();
        assert_eq!(std::fs::read_to_string(target.join("README.md")).unwrap(), "hi");
    }

    #[test]
    fn failed_write_leaves_no_residue() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("p");
        // `..` is a path escape: staged files are rejected before any partial tree
        // is exposed at the target.
        let bad = plan_at(target.clone(), &[("ok.txt", "a"), ("../escape.txt", "b")]);
        let err = write_atomic(&bad).unwrap_err();
        assert_eq!(err.code, "write.io");
        assert!(!target.exists());
        let leftovers: Vec<String> = std::fs::read_dir(tmp.path())
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|n| n.starts_with(".vinoa-staging-"))
            .collect();
        assert!(leftovers.is_empty(), "staging residue: {leftovers:?}");
    }

    #[test]
    fn join_checked_rejects_escapes_and_absolute_paths() {
        let base = Path::new("/tmp/base");
        assert_eq!(join_checked(base, "a/b.txt").unwrap(), PathBuf::from("/tmp/base/a/b.txt"));
        for bad in ["", ".", "..", "a/../../x", "/etc/passwd", "C:\\x"] {
            assert!(join_checked(base, bad).is_err(), "{bad} should be rejected");
        }
    }
}
