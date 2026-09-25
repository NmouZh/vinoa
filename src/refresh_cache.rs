//! User-cache for `vinoa versions --refresh` (spec §6.4). OWNER: cli-dev.
//!
//! The cache lives in the **user cache directory** — never in a generated project
//! and never in the repository. It only records upstream release/build data so
//! `versions` can label its provenance (`builtin` / `cache` / `online`) and warn
//! about upstream drift; it must never leak a timestamp or machine path into a
//! generated artifact (spec §5/§6.4 reproducibility rule).
//!
//! Declared from `cli.rs` via `#[path]` so the file lives at the task's expected
//! `src/refresh_cache.rs` while `lib.rs` (Lead-owned) stays untouched.
use crate::error::{self, Result};
use std::path::{Path, PathBuf};

/// File name inside the cache directory.
pub const CACHE_FILE_NAME: &str = "matrix-cache.json";
/// Envelope schema tag; a cache with any other value is ignored.
pub const CACHE_SCHEMA: &str = "vinoa.matrix-cache/v1";

/// Provenance of the version data used by `versions` / `init` (spec §6.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatrixSource {
    Builtin,
    Cache,
    Online,
}

impl MatrixSource {
    pub fn as_str(self) -> &'static str {
        match self {
            MatrixSource::Builtin => "builtin",
            MatrixSource::Cache => "cache",
            MatrixSource::Online => "online",
        }
    }
}

/// Result of looking for a usable cache file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheState {
    Missing,
    /// Parsed, schema-supported cache document.
    Valid(serde_json::Value),
    /// Present but unusable (bad JSON, wrong schema, missing envelope keys).
    Corrupt(String),
}

impl CacheState {
    pub fn is_valid(&self) -> bool {
        matches!(self, CacheState::Valid(_))
    }

    pub fn document(&self) -> Option<&serde_json::Value> {
        match self {
            CacheState::Valid(v) => Some(v),
            _ => None,
        }
    }

    pub fn changed(&self) -> Vec<String> {
        self.document()
            .and_then(|d| d.get("report"))
            .and_then(|r| r.get("changed"))
            .and_then(|c| c.as_array())
            .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
            .unwrap_or_default()
    }

    pub fn generated_at(&self) -> Option<&str> {
        self.document()?.get("generated_at")?.as_str()
    }
}

/// Cache directory for the current OS (spec §6.4).
pub fn cache_dir() -> Option<PathBuf> {
    cache_dir_for(
        if cfg!(windows) {
            "windows"
        } else if cfg!(target_os = "macos") {
            "macos"
        } else {
            "linux"
        },
        std::env::var("XDG_CACHE_HOME").ok().as_deref(),
        std::env::var("HOME").ok().as_deref(),
        std::env::var("LOCALAPPDATA").ok().as_deref(),
    )
}

/// Pure OS dispatch (`os` ∈ `windows` | `macos` | anything else ⇒ Unix/Linux).
pub fn cache_dir_for(
    os: &str,
    xdg: Option<&str>,
    home: Option<&str>,
    local_app_data: Option<&str>,
) -> Option<PathBuf> {
    match os {
        "windows" => local_app_data.map(|l| PathBuf::from(l).join("vinoa").join("cache")),
        "macos" => home.map(|h| PathBuf::from(h).join("Library/Caches/vinoa")),
        _ => xdg
            .map(PathBuf::from)
            .or_else(|| home.map(|h| PathBuf::from(h).join(".cache")))
            .map(|p| p.join("vinoa")),
    }
}

pub fn cache_file() -> Option<PathBuf> {
    cache_dir().map(|d| d.join(CACHE_FILE_NAME))
}

/// Read the cache in the current user cache directory.
pub fn load() -> CacheState {
    match cache_file() {
        Some(path) => load_at(&path),
        None => CacheState::Missing,
    }
}

/// Read + validate a cache file. Corrupt or schema-mismatched cache is reported,
/// never fatal (spec §6.4 / task-6: fall back to builtin with a warning).
pub fn load_at(path: &Path) -> CacheState {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return CacheState::Missing,
        Err(e) => return CacheState::Corrupt(format!("无法读取 {}: {e}", path.display())),
    };
    let doc: serde_json::Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(e) => return CacheState::Corrupt(format!("缓存不是合法 JSON（{}）: {e}", path.display())),
    };
    let schema = doc.get("schema").and_then(|v| v.as_str()).unwrap_or("");
    if schema != CACHE_SCHEMA {
        return CacheState::Corrupt(format!(
            "缓存 schema 不匹配（期望 {CACHE_SCHEMA}，实际 `{schema}`）"
        ));
    }
    if !doc.get("report").map(|r| r.is_object()).unwrap_or(false) {
        return CacheState::Corrupt("缓存缺少 report 对象".to_string());
    }
    CacheState::Valid(doc)
}

/// Atomic write of a refresh result into the user cache directory.
pub fn write(report: &crate::matrix::RefreshReport) -> Result<PathBuf> {
    let dir = cache_dir().ok_or_else(|| {
        error::config("无法确定用户缓存目录（XDG_CACHE_HOME / HOME / LOCALAPPDATA 均不可用）")
    })?;
    write_at(&dir, report)
}

/// Atomic write into an explicit directory (testable).
pub fn write_at(dir: &Path, report: &crate::matrix::RefreshReport) -> Result<PathBuf> {
    std::fs::create_dir_all(dir)
        .map_err(|e| error::io(format!("无法创建缓存目录 {}: {e}", dir.display())))?;
    let doc = serde_json::json!({
        "schema": CACHE_SCHEMA,
        "vinoa": env!("CARGO_PKG_VERSION"),
        "generated_at": crate::report::utc_now_rfc3339(),
        "report": report,
    });
    let file = tempfile::Builder::new()
        .prefix(".vinoa-cache-")
        .tempfile_in(dir)
        .map_err(|e| error::io(format!("无法创建缓存临时文件: {e}")))?;
    serde_json::to_writer_pretty(&file, &doc)
        .map_err(|e| error::io(format!("无法序列化矩阵缓存: {e}")))?;
    let path = dir.join(CACHE_FILE_NAME);
    file.persist(&path)
        .map_err(|e| error::io(format!("无法写入缓存 {}: {}", path.display(), e.error)))?;
    Ok(path)
}

/// Decide where version data comes from, and whether to warn.
///
/// Precedence (spec §6.4):
/// 1. `--offline` never goes online; `--refresh` + `--offline` ⇒ warning, no network.
/// 2. `--refresh` / `--online` ⇒ one online fetch (still writes the cache).
/// 3. otherwise the cache wins when present, else the built-in matrix.
pub fn decide_source(
    refresh: bool,
    online: bool,
    offline: bool,
    cache: &CacheState,
) -> (MatrixSource, Option<String>) {
    if offline && (refresh || online) {
        let source = if cache.is_valid() { MatrixSource::Cache } else { MatrixSource::Builtin };
        return (
            source,
            Some("--offline 与 --refresh/--online 同时给出：不联网，改用缓存/内置矩阵".to_string()),
        );
    }
    if refresh || online {
        return (MatrixSource::Online, None);
    }
    if cache.is_valid() {
        (MatrixSource::Cache, None)
    } else {
        (MatrixSource::Builtin, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::matrix::RefreshReport;
    use std::collections::BTreeMap;

    fn report(changed: &[&str]) -> RefreshReport {
        RefreshReport {
            source: "online".into(),
            projects: BTreeMap::new(),
            changed: changed.iter().map(|s| (*s).to_string()).collect(),
        }
    }

    #[test]
    fn cache_dir_follows_each_platform_convention() {
        assert_eq!(
            cache_dir_for("linux", Some("/x/cache"), Some("/home/u"), None),
            Some(PathBuf::from("/x/cache/vinoa"))
        );
        assert_eq!(
            cache_dir_for("linux", None, Some("/home/u"), None),
            Some(PathBuf::from("/home/u/.cache/vinoa"))
        );
        assert_eq!(
            cache_dir_for("macos", None, Some("/Users/u"), None),
            Some(PathBuf::from("/Users/u/Library/Caches/vinoa"))
        );
        assert_eq!(
            cache_dir_for("windows", None, None, Some("C:\\Users\\u\\AppData\\Local")),
            Some(PathBuf::from("C:\\Users\\u\\AppData\\Local").join("vinoa").join("cache"))
        );
        assert_eq!(cache_dir_for("linux", None, None, None), None);
        assert_eq!(cache_dir_for("windows", None, Some("/home/u"), None), None);
    }

    #[test]
    fn write_then_load_round_trips_without_residue() {
        let tmp = tempfile::tempdir().unwrap();
        let path = write_at(tmp.path(), &report(&["paper: 上游新增 26.3"])).unwrap();
        assert_eq!(path.file_name().unwrap().to_string_lossy(), CACHE_FILE_NAME);

        let state = load_at(&path);
        assert!(state.is_valid());
        assert_eq!(state.changed(), vec!["paper: 上游新增 26.3"]);
        assert!(state.generated_at().unwrap().ends_with('Z'));
        assert_eq!(state.document().unwrap()["report"]["source"], "online");

        let leftovers: Vec<String> = std::fs::read_dir(tmp.path())
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|n| n.starts_with(".vinoa-cache-"))
            .collect();
        assert!(leftovers.is_empty(), "cache residue: {leftovers:?}");
    }

    #[test]
    fn missing_cache_is_missing_not_corrupt() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(load_at(&tmp.path().join(CACHE_FILE_NAME)), CacheState::Missing);
    }

    #[test]
    fn corrupt_or_wrong_schema_cache_is_ignored_not_fatal() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join(CACHE_FILE_NAME);

        std::fs::write(&path, b"{ not json").unwrap();
        assert!(matches!(load_at(&path), CacheState::Corrupt(_)));

        std::fs::write(&path, br#"{"schema":"vinoa.matrix-cache/v99","report":{}}"#).unwrap();
        let state = load_at(&path);
        assert!(matches!(state, CacheState::Corrupt(_)));
        match state {
            CacheState::Corrupt(reason) => assert!(reason.contains("schema 不匹配"), "{reason}"),
            _ => unreachable!(),
        }

        std::fs::write(&path, br#"{"schema":"vinoa.matrix-cache/v1"}"#).unwrap();
        assert!(matches!(load_at(&path), CacheState::Corrupt(_)));
    }

    #[test]
    fn source_precedence_never_networks_when_offline() {
        let no_cache = CacheState::Missing;
        let cache = CacheState::Valid(serde_json::json!({
            "schema": CACHE_SCHEMA,
            "generated_at": "2026-09-25T10:00:00Z",
            "report": {"source": "online", "changed": []}
        }));

        // Default, no cache → builtin.
        assert_eq!(decide_source(false, false, false, &no_cache), (MatrixSource::Builtin, None));
        // Default, cache present → cache.
        assert_eq!(decide_source(false, false, false, &cache), (MatrixSource::Cache, None));
        // --offline with cache → cache, no warning.
        assert_eq!(decide_source(false, false, true, &cache), (MatrixSource::Cache, None));
        // --refresh → online.
        assert_eq!(decide_source(true, false, false, &no_cache), (MatrixSource::Online, None));
        // --online → online.
        assert_eq!(decide_source(false, true, false, &no_cache), (MatrixSource::Online, None));
        // --offline + --refresh → no network, warning.
        let (source, warning) = decide_source(true, false, true, &cache);
        assert_eq!(source, MatrixSource::Cache);
        assert!(warning.unwrap().contains("不联网"));
        // --offline + --online without cache → builtin + warning.
        let (source, warning) = decide_source(false, true, true, &no_cache);
        assert_eq!(source, MatrixSource::Builtin);
        assert!(warning.is_some());
        // A corrupt cache never yields `cache`.
        let corrupt = CacheState::Corrupt("bad".into());
        assert_eq!(decide_source(false, false, false, &corrupt), (MatrixSource::Builtin, None));
    }

    #[test]
    fn source_labels_match_the_spec_strings() {
        assert_eq!(MatrixSource::Builtin.as_str(), "builtin");
        assert_eq!(MatrixSource::Cache.as_str(), "cache");
        assert_eq!(MatrixSource::Online.as_str(), "online");
    }
}
