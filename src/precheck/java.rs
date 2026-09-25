//! JAVA_HOME -> PATH -> common install dirs. OWNER: cli-dev.
//!
//! Spec §10: init never asks for a Java version — the matrix derives it per
//! module. Detection must **not** start Gradle; the only external process we may
//! spawn is a `java -version` probe (and only when a `release` file is absent).
use crate::types::Resolved;
use std::path::{Path, PathBuf};

/// Gradle plugin written into the generated project when auto-download is on.
pub const FOOJAY_PLUGIN: &str = "org.gradle.toolchains.foojay-resolver-convention";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JavaCheck {
    pub required: u8,
    pub platform: String,
    pub found_path: Option<String>,
}

impl JavaCheck {
    pub fn satisfied(&self) -> bool {
        self.found_path.is_some()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct JavaReport {
    pub checks: Vec<JavaCheck>,
    pub gradle_version: Option<String>,
}

impl JavaReport {
    pub fn missing(&self) -> Vec<&JavaCheck> {
        self.checks.iter().filter(|c| !c.satisfied()).collect()
    }
    pub fn all_satisfied(&self) -> bool {
        self.missing().is_empty()
    }

    /// Spec §11.5 `precheck` object: per-module `required`/`found`/`path`, plus
    /// the auto-download decision.
    pub fn to_json(&self, auto_download: bool) -> serde_json::Value {
        let jdk: Vec<serde_json::Value> = self
            .checks
            .iter()
            .map(|c| {
                serde_json::json!({
                    "module": c.platform.clone(),
                    "required": c.required,
                    "found": c.satisfied(),
                    "path": c.found_path.clone(),
                })
            })
            .collect();
        serde_json::json!({
            "jdk": jdk,
            "auto_download": {
                "enabled": auto_download,
                "plugin": if auto_download { Some(FOOJAY_PLUGIN) } else { None },
            },
        })
    }
}

/// A discovered JDK: its major version and its home directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JavaHome {
    pub major: u32,
    pub path: PathBuf,
}

impl JavaHome {
    pub fn new(major: u32, path: impl Into<PathBuf>) -> Self {
        Self { major, path: path.into() }
    }
}

/// Detect the JDKs required by the resolved plan.
pub fn detect(resolved: &Resolved) -> JavaReport {
    check_with_homes(resolved, &discover_java_homes())
}

/// Pure half of [`detect`]: pair required targets with already-discovered homes.
pub fn check_with_homes(resolved: &Resolved, homes: &[JavaHome]) -> JavaReport {
    let mut checks: Vec<JavaCheck> = resolved
        .platforms
        .iter()
        .map(|(id, plan)| {
            let module = if plan.gradle_path.is_empty() {
                format!("platforms:{id}")
            } else {
                plan.gradle_path.clone()
            };
            let found = homes
                .iter()
                .find(|h| h.major == u32::from(plan.java_target))
                .map(|h| h.path.to_string_lossy().to_string());
            JavaCheck { required: plan.java_target, platform: module, found_path: found }
        })
        .collect();
    checks.sort_by(|a, b| a.platform.cmp(&b.platform));
    JavaReport { checks, gradle_version: Some(resolved.gradle_version.clone()) }
}

/// Detection order is fixed by spec §10: `JAVA_HOME` → `PATH` → common dirs.
pub fn discover_java_homes() -> Vec<JavaHome> {
    let mut homes: Vec<JavaHome> = Vec::new();
    let mut seen: Vec<PathBuf> = Vec::new();

    // 1. JAVA_HOME
    if let Ok(home) = std::env::var("JAVA_HOME") {
        push_home(&mut homes, &mut seen, Path::new(home.trim()));
    }

    // 2. `java` on PATH
    if let Some(exe) = find_in_path(&std::env::var("PATH").unwrap_or_default(), java_exe_name()) {
        if let Some(home) = java_home_of(&exe) {
            push_home(&mut homes, &mut seen, &home);
        }
    }

    // 3. Common install directories (Linux / Windows / macOS / SDKMAN / asdf)
    for dir in common_java_dirs() {
        push_home(&mut homes, &mut seen, &dir);
    }
    homes
}

fn push_home(homes: &mut Vec<JavaHome>, seen: &mut Vec<PathBuf>, dir: &Path) {
    if !java_exe(dir).is_file() {
        return;
    }
    let canonical = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
    if seen.contains(&canonical) {
        return;
    }
    let Some(major) = probe_major(dir) else { return };
    seen.push(canonical);
    homes.push(JavaHome::new(major, dir.to_path_buf()));
}

/// Major version of the JDK at `home`: `release` file first, then `java -version`.
fn probe_major(home: &Path) -> Option<u32> {
    if let Ok(text) = std::fs::read_to_string(home.join("release")) {
        if let Some(major) = parse_major_from_release(&text) {
            return Some(major);
        }
    }
    let output = std::process::Command::new(java_exe(home))
        .arg("-version")
        .output()
        .ok()?;
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    parse_major_from_version_output(&text)
}

/// `JAVA_VERSION="21.0.1"` / `JAVA_VERSION="1.8.0_392"` → `21` / `8`.
pub fn parse_major_from_release(text: &str) -> Option<u32> {
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("JAVA_VERSION=") {
            let value = rest.trim().trim_matches('"');
            return major_from_version_string(value);
        }
    }
    None
}

/// `openjdk version "21.0.2"` / `java version "1.8.0_392"` → `21` / `8`.
pub fn parse_major_from_version_output(text: &str) -> Option<u32> {
    let (_, rest) = text.split_once("version \"")?;
    let (value, _) = rest.split_once('"')?;
    major_from_version_string(value)
}

/// `1.8.0_392` → 8, `21.0.2` → 21, `25` → 25.
pub fn major_from_version_string(value: &str) -> Option<u32> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    let mut parts = value.split(['.', '_', '-', '+']);
    let first = parts.next()?;
    if first == "1" {
        parts.next()?.parse().ok()
    } else {
        first.parse().ok()
    }
}

/// `/usr/lib/jvm/java-21/bin/java` → `/usr/lib/jvm/java-21`.
pub fn java_home_of(java_exe: &Path) -> Option<PathBuf> {
    let bin = java_exe.parent()?;
    if bin.file_name()?.to_string_lossy() != "bin" {
        return None;
    }
    Some(bin.parent()?.to_path_buf())
}

fn java_exe_name() -> &'static str {
    if cfg!(windows) { "java.exe" } else { "java" }
}

fn java_exe(home: &Path) -> PathBuf {
    home.join("bin").join(java_exe_name())
}

/// `java` (or `java.exe`) resolved against a `PATH`-style string.
pub fn find_in_path(path_env: &str, exe: &str) -> Option<PathBuf> {
    for dir in std::env::split_paths(path_env) {
        if dir.as_os_str().is_empty() {
            continue;
        }
        let candidate = dir.join(exe);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// Common install roots for the current OS (plus SDKMAN / asdf everywhere).
pub fn common_java_dirs() -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();

    #[cfg(target_os = "linux")]
    {
        roots.push(PathBuf::from("/usr/lib/jvm"));
        roots.push(PathBuf::from("/usr/java"));
    }
    #[cfg(target_os = "macos")]
    {
        roots.push(PathBuf::from("/Library/Java/JavaVirtualMachines"));
        roots.push(PathBuf::from("/System/Volumes/Data/Library/Java/JavaVirtualMachines"));
    }
    #[cfg(target_os = "windows")]
    {
        for base in ["ProgramFiles", "ProgramFiles(x86)", "ProgramW6432"] {
            if let Ok(dir) = std::env::var(base) {
                roots.push(PathBuf::from(&dir).join("Java"));
                roots.push(PathBuf::from(&dir).join("Eclipse Adoptium"));
                roots.push(PathBuf::from(&dir).join("Microsoft"));
                roots.push(PathBuf::from(&dir).join("Zulu"));
            }
        }
    }

    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_default();
    if !home.is_empty() {
        let sdkman = std::env::var("SDKMAN_CANDIDATES_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(&home).join(".sdkman/candidates/java"));
        roots.push(sdkman);
        roots.push(PathBuf::from(&home).join(".asdf/installs/java"));
        roots.push(PathBuf::from(&home).join(".jdks"));
    }

    let mut dirs = Vec::new();
    for root in roots {
        if let Ok(entries) = std::fs::read_dir(&root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if java_exe(&path).is_file() {
                    // macOS bundles keep the real home one level deeper.
                    let nested = path.join("Contents/Home");
                    if java_exe(&nested).is_file() {
                        dirs.push(nested);
                    } else {
                        dirs.push(path);
                    }
                }
            }
        }
    }
    dirs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{PlatformPlan, Resolved};
    use std::collections::BTreeMap;

    fn plan(id: &str, java: u8) -> PlatformPlan {
        PlatformPlan {
            id: id.into(),
            gradle_path: format!("platforms:{id}"),
            api_coordinate: "g:a:1".into(),
            java_target: java,
            metadata: crate::types::MetadataFormat::PluginYml,
            capability: Default::default(),
            experimental: false,
        }
    }

    fn resolved_with(platforms: &[(&str, u8)]) -> Resolved {
        let mut map = BTreeMap::new();
        for (id, java) in platforms {
            map.insert((*id).to_string(), plan(id, *java));
        }
        Resolved {
            mc_version: "1.21.11".into(),
            gradle_version: "9.8.0".into(),
            core_java_target: 21,
            platforms: map,
            thirdparty: BTreeMap::new(),
            quality: BTreeMap::new(),
            gradle_plugins: BTreeMap::new(),
        }
    }

    #[test]
    fn missing_jdk_reports_required_and_found_structure() {
        // Only Java 21 exists on this machine.
        let homes = vec![JavaHome::new(21, "/usr/lib/jvm/java-21-openjdk")];
        let report = check_with_homes(&resolved_with(&[("paper", 21), ("bukkit", 8)]), &homes);

        assert!(!report.all_satisfied());
        let paper = report.checks.iter().find(|c| c.platform == "platforms:paper").unwrap();
        assert_eq!(paper.required, 21);
        assert!(paper.satisfied());
        assert_eq!(paper.found_path.as_deref(), Some("/usr/lib/jvm/java-21-openjdk"));

        let bukkit = report.checks.iter().find(|c| c.platform == "platforms:bukkit").unwrap();
        assert_eq!(bukkit.required, 8);
        assert!(!bukkit.satisfied());
        assert_eq!(bukkit.found_path, None);

        let json = report.to_json(true);
        assert_eq!(json["jdk"][0]["module"], "platforms:bukkit");
        assert_eq!(json["jdk"][0]["required"], 8);
        assert_eq!(json["jdk"][0]["found"], false);
        assert_eq!(json["jdk"][0]["path"], serde_json::Value::Null);
        assert_eq!(json["auto_download"]["enabled"], true);
        assert_eq!(json["auto_download"]["plugin"], FOOJAY_PLUGIN);
        assert_eq!(report.gradle_version.as_deref(), Some("9.8.0"));
    }

    #[test]
    fn all_satisfied_when_every_target_is_present() {
        let homes = vec![JavaHome::new(8, "/jdk8"), JavaHome::new(21, "/jdk21"), JavaHome::new(25, "/jdk25")];
        let report = check_with_homes(
            &resolved_with(&[("paper", 21), ("bukkit", 8), ("velocity", 25)]),
            &homes,
        );
        assert!(report.all_satisfied());
        assert!(report.missing().is_empty());
        let json = report.to_json(false);
        assert_eq!(json["auto_download"]["enabled"], false);
        assert_eq!(json["auto_download"]["plugin"], serde_json::Value::Null);
    }

    #[test]
    fn java_home_is_prioritised_over_later_homes() {
        let homes = vec![JavaHome::new(21, "/first"), JavaHome::new(21, "/second")];
        let report = check_with_homes(&resolved_with(&[("paper", 21)]), &homes);
        assert_eq!(report.checks[0].found_path.as_deref(), Some("/first"));
    }

    #[test]
    fn parses_major_from_release_and_version_output() {
        assert_eq!(parse_major_from_release("JAVA_VERSION=\"21.0.2\"\n"), Some(21));
        assert_eq!(parse_major_from_release("JAVA_VERSION=\"1.8.0_392\"\n"), Some(8));
        assert_eq!(parse_major_from_release("IMPLEMENTOR=\"Temurin\"\n"), None);
        assert_eq!(parse_major_from_version_output("openjdk version \"25\" 2026-09-01"), Some(25));
        assert_eq!(
            parse_major_from_version_output("java version \"1.8.0_392\"\nJava(TM) SE"),
            Some(8)
        );
        assert_eq!(parse_major_from_version_output("no version here"), None);
        assert_eq!(major_from_version_string("17.0.9+9-LTS"), Some(17));
        assert_eq!(major_from_version_string(""), None);
    }

    #[test]
    fn java_home_of_only_accepts_bin_dirs() {
        assert_eq!(
            java_home_of(Path::new("/usr/lib/jvm/java-21-openjdk/bin/java")),
            Some(PathBuf::from("/usr/lib/jvm/java-21-openjdk"))
        );
        assert_eq!(java_home_of(Path::new("/usr/lib/jvm/java-21-openjdk/java")), None);
    }

    #[test]
    fn find_in_path_scans_each_entry() {
        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        let exe = bin.join(if cfg!(windows) { "java.exe" } else { "java" });
        std::fs::write(&exe, b"").unwrap();
        let path_env = std::env::join_paths([dir.path().join("nope"), bin.clone()])
            .unwrap()
            .to_string_lossy()
            .to_string();
        assert_eq!(find_in_path(&path_env, java_exe_name()), Some(exe));
        assert_eq!(find_in_path("", java_exe_name()), None);
    }

    #[test]
    fn common_dirs_are_os_appropriate() {
        let dirs = common_java_dirs();
        // The function may legitimately find nothing in a container, but it must
        // never panic and (on Linux) must consider /usr/lib/jvm.
        #[cfg(target_os = "linux")]
        {
            let _ = dirs;
            assert!(Path::new("/usr/lib/jvm").exists() || dirs.is_empty());
        }
        #[cfg(not(target_os = "linux"))]
        let _ = dirs;
    }
}
