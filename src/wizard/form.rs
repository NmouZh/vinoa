//! Pure wizard state + validation (spec §4). OWNER: cli-dev.
//!
//! Everything here is deterministic and TTY-free so it can be unit tested; the
//! interactive `inquire` prompts live in `mod.rs`.
use crate::cli::InitArgs;
use crate::error::{Error, Result, EXIT_DATA};
use crate::matrix::Matrix;
use crate::types::{
    normalize_platforms, slug, ArtifactLanguage, FeatureSet, MetadataFormat, ProjectSpec,
    QualitySet, UiLanguage, PLATFORMS,
};
use std::path::PathBuf;

/// Add-on modules offered on wizard page ④ (spec §3.3): the quality sub-items are
/// CLI-only and never appear in the wizard.
pub const WIZARD_FEATURES: [&str; 5] =
    ["sqlite", "bstats", "update-check", "placeholderapi", "gui"];

/// Result of one wizard page: advance or go back one page (`Esc` is an error).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Nav {
    Next,
    Back,
}

/// Wizard answers, already validated piece-wise. `paper` → `bukkit` is applied by
/// [`normalize_platforms`] when the spec is built.
#[derive(Debug, Clone)]
pub struct WizardForm {
    pub name: String,
    pub package: String,
    pub mc: String,
    pub platforms: Vec<String>,
    pub metadata: MetadataFormat,
    pub example: bool,
    pub permissions: bool,
    pub quality: bool,
    pub git: bool,
    pub features: FeatureSet,
    pub language: ArtifactLanguage,
    pub ui_language: UiLanguage,
    pub bstats_id: Option<String>,
    pub download_jdk: bool,
}

impl WizardForm {
    /// Seed every field from the flags/defaults so backing up a page never loses
    /// an answer the user already typed on the command line.
    pub fn from_args(args: &InitArgs, matrix: &Matrix) -> Result<Self> {
        let name = match &args.name {
            Some(n) if !n.trim().is_empty() => n.clone(),
            _ => current_dir_name()?,
        };
        let package = args
            .package
            .clone()
            .unwrap_or_else(|| format!("com.example.{}", slug(&name)));
        let supported = matrix.supported_versions();
        let mc = match &args.mc {
            Some(m) => m.clone(),
            None => latest_supported(&supported).ok_or_else(|| {
                Error::new("matrix.empty", crate::error::EXIT_CONFIG, "内置矩阵没有任何受支持的 MC 版本")
            })?,
        };
        let language = match args.language.as_deref() {
            Some("zh") => ArtifactLanguage::Zh,
            Some("en") => ArtifactLanguage::En,
            Some("both") => ArtifactLanguage::Both,
            _ => system_language(),
        };
        let ui_language = match args.ui_language.as_deref() {
            Some("zh") => UiLanguage::Zh,
            Some("en") => UiLanguage::En,
            _ => system_ui_language(),
        };
        Ok(Self {
            name,
            package,
            mc,
            platforms: normalize_platforms(&args.platforms),
            metadata: metadata_from_arg(args.metadata.as_deref()),
            example: !args.no_example,
            permissions: !args.no_permissions,
            quality: !args.no_quality,
            git: args.git && !args.no_git,
            features: crate::init::parse_features(&args.features),
            language,
            ui_language,
            bstats_id: args.bstats_id.clone(),
            download_jdk: args.download_jdk.unwrap_or(false),
        })
    }

    /// Turn the collected answers into the frozen [`ProjectSpec`] contract.
    pub fn build_spec(&self, args: &InitArgs) -> Result<ProjectSpec> {
        validate_project_name(&self.name)?;
        validate_package_name(&self.package)?;
        let platforms = normalize_platforms(&self.platforms);
        if platforms.is_empty() {
            return Err(Error::new(
                "var.invalid_value",
                EXIT_DATA,
                "至少选择一个平台（平台 × 版本组合见 vinoa versions --matrix）",
            ));
        }
        if self.features.bstats && !is_valid_bstats_id(self.bstats_id.as_deref()) {
            return Err(Error::new(
                "var.invalid_value",
                EXIT_DATA,
                "勾选了 bStats 必须提供数字 plugin id；我们不生成假 id",
            )
            .with_hint("plugin id 在 bstats.org 新建插件后取得（--bstats-id <数字>）"));
        }
        Ok(ProjectSpec {
            target_dir: PathBuf::from(
                args.output.clone().unwrap_or_else(|| self.name.clone()),
            ),
            plugin_name: crate::types::to_pascal(&self.name),
            project_name: self.name.clone(),
            package_name: self.package.clone(),
            author: args.author.clone(),
            description: args.description.clone(),
            mc_version: self.mc.clone(),
            platforms,
            metadata: self.metadata,
            language: self.language,
            ui_language: self.ui_language,
            license: args
                .license
                .clone()
                .unwrap_or_else(|| "Apache-2.0".to_string()),
            features: self.features.clone(),
            quality: QualitySet {
                checkstyle: self.quality,
                unit_tests: self.quality,
                ci: self.quality,
            },
            git: self.git,
            download_jdk: self.download_jdk,
            bstats_id: self.bstats_id.clone(),
            example: self.example,
            permissions: self.permissions,
            website: args.website.clone(),
        })
    }
}

/// `vinoa init` in place: the current directory name is the project name.
pub fn current_dir_name() -> Result<String> {
    let cwd = std::env::current_dir().map_err(|e| {
        Error::new("io.failed", crate::error::EXIT_IO, format!("无法读取当前目录: {e}"))
    })?;
    cwd.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .filter(|n| !n.is_empty())
        .ok_or_else(|| {
            Error::new(
                "usage.invalid",
                crate::error::EXIT_USAGE,
                "无法从当前目录推导工程名；请显式给出 `vinoa init <名称>`",
            )
        })
}

/// `^[A-Za-z][A-Za-z0-9_-]{0,63}$` (spec §4.1).
pub fn validate_project_name(name: &str) -> Result<()> {
    let bytes = name.as_bytes();
    let ok = !bytes.is_empty()
        && bytes.len() <= 64
        && bytes[0].is_ascii_alphabetic()
        && bytes
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || *b == b'_' || *b == b'-');
    if ok {
        Ok(())
    } else {
        Err(Error::new(
            "var.invalid_value",
            EXIT_DATA,
            format!("工程名不合法: `{name}`（须匹配 ^[A-Za-z][A-Za-z0-9_-]{{0,63}}$）"),
        ))
    }
}

/// Each segment `^[a-z_][a-z0-9_]*$` and not a Java keyword (spec §4.1).
pub fn validate_package_name(package: &str) -> Result<()> {
    for segment in package.split('.') {
        let bytes = segment.as_bytes();
        let shape_ok = !bytes.is_empty()
            && (bytes[0].is_ascii_lowercase() || bytes[0] == b'_')
            && bytes
                .iter()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'_');
        if !shape_ok || is_java_keyword(segment) {
            return Err(Error::new(
                "var.invalid_value",
                EXIT_DATA,
                format!("包名不合法: `{package}`（段 `{segment}` 必须是 ^[a-z_][a-z0-9_]*$ 且非 Java 关键字）"),
            ));
        }
    }
    Ok(())
}

pub fn is_java_keyword(segment: &str) -> bool {
    const KEYWORDS: [&str; 53] = [
        "abstract", "assert", "boolean", "break", "byte", "case", "catch", "char", "class",
        "const", "continue", "default", "do", "double", "else", "enum", "extends", "final",
        "finally", "float", "for", "goto", "if", "implements", "import", "instanceof", "int",
        "interface", "long", "native", "new", "package", "private", "protected", "public",
        "return", "short", "static", "strictfp", "super", "switch", "synchronized", "this",
        "throw", "throws", "transient", "try", "void", "volatile", "while", "true", "false",
        "null",
    ];
    KEYWORDS.contains(&segment)
}

pub fn is_valid_bstats_id(id: Option<&str>) -> bool {
    matches!(id, Some(v) if !v.is_empty() && v.chars().all(|c| c.is_ascii_digit()))
}

/// Platform picker options for a MC version: only platforms the matrix knows for
/// that version (proxies are always available, spec §6.3.5), canonical order.
pub fn platform_options(available: &[String]) -> Vec<String> {
    PLATFORMS
        .iter()
        .filter(|p| {
            available.iter().any(|a| a == *p)
                || **p == "velocity"
                || **p == "bungeecord"
        })
        .map(|p| (*p).to_string())
        .collect()
}

/// Highest supported MC version (numeric-segment comparison handles both the
/// `1.x.y` and the date-style `26.x` axes).
pub fn latest_supported(versions: &[String]) -> Option<String> {
    versions.iter().max_by(|a, b| compare_mc(a, b)).cloned()
}

/// Compare two MC version strings segment by segment.
pub fn compare_mc(a: &str, b: &str) -> std::cmp::Ordering {
    let pa: Vec<u64> = a.split('.').map(|s| s.parse().unwrap_or(0)).collect();
    let pb: Vec<u64> = b.split('.').map(|s| s.parse().unwrap_or(0)).collect();
    for i in 0..pa.len().max(pb.len()) {
        let x = pa.get(i).copied().unwrap_or(0);
        let y = pb.get(i).copied().unwrap_or(0);
        match x.cmp(&y) {
            std::cmp::Ordering::Equal => continue,
            other => return other,
        }
    }
    std::cmp::Ordering::Equal
}

pub const METADATA_LABELS: [&str; 2] = ["plugin.yml", "paper-plugin"];

pub fn metadata_from_label(label: &str) -> MetadataFormat {
    if label.starts_with("paper") {
        MetadataFormat::PaperPluginYml
    } else {
        MetadataFormat::PluginYml
    }
}

pub fn label_for_metadata(m: MetadataFormat) -> &'static str {
    match m {
        MetadataFormat::PluginYml => "plugin.yml",
        MetadataFormat::PaperPluginYml => "paper-plugin",
    }
}

fn metadata_from_arg(value: Option<&str>) -> MetadataFormat {
    match value.map(|v| v.to_ascii_lowercase()).as_deref() {
        Some("paper-plugin") | Some("paper-plugin.yml") | Some("paper-plugin-yml") => {
            MetadataFormat::PaperPluginYml
        }
        _ => MetadataFormat::PluginYml,
    }
}

/// Artifact language default: Chinese system → `zh`, otherwise `en` (spec §4.3).
pub fn system_language() -> ArtifactLanguage {
    if system_is_chinese() {
        ArtifactLanguage::Zh
    } else {
        ArtifactLanguage::En
    }
}

pub fn system_ui_language() -> UiLanguage {
    if system_is_chinese() {
        UiLanguage::Zh
    } else {
        UiLanguage::En
    }
}

fn system_is_chinese() -> bool {
    for key in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(v) = std::env::var(key) {
            let v = v.to_ascii_lowercase();
            if v.starts_with("zh") {
                return true;
            }
            if !v.is_empty() && v != "c" && v != "posix" {
                return false;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args_with(name: &str) -> InitArgs {
        InitArgs { name: Some(name.to_string()), ..InitArgs::default() }
    }

    fn form(name: &str, package: &str) -> WizardForm {
        WizardForm {
            name: name.into(),
            package: package.into(),
            mc: "1.21.11".into(),
            platforms: vec!["paper".into()],
            metadata: MetadataFormat::PluginYml,
            example: true,
            permissions: true,
            quality: true,
            git: true,
            features: FeatureSet::default(),
            language: ArtifactLanguage::Zh,
            ui_language: UiLanguage::Zh,
            bstats_id: None,
            download_jdk: false,
        }
    }

    #[test]
    fn project_name_rules() {
        assert!(validate_project_name("my-plugin").is_ok());
        assert!(validate_project_name("A1_b").is_ok());
        assert!(validate_project_name("").is_err());
        assert!(validate_project_name("1plugin").is_err());
        assert!(validate_project_name("my plugin").is_err());
        assert!(validate_project_name(&"a".repeat(65)).is_err());
        assert_eq!(validate_project_name("0bad").unwrap_err().code, "var.invalid_value");
        assert_eq!(validate_project_name("0bad").unwrap_err().exit_code(), EXIT_DATA);
    }

    #[test]
    fn package_name_rules() {
        assert!(validate_package_name("com.example.myplugin").is_ok());
        assert!(validate_package_name("com.example.my_plugin2").is_ok());
        assert!(validate_package_name("Com.example").is_err());
        assert!(validate_package_name("com..x").is_err());
        assert!(validate_package_name("com.class.x").is_err());
        assert!(validate_package_name("com.1x").is_err());
        assert!(is_java_keyword("class") && is_java_keyword("true") && !is_java_keyword("core"));
    }

    #[test]
    fn paper_implies_bukkit_and_orders_canonically() {
        let spec = form("my-plugin", "com.example.myplugin").build_spec(&args_with("my-plugin")).unwrap();
        assert_eq!(spec.platforms, vec!["paper", "bukkit"]);
        let spec2 = form("my-plugin", "com.example.myplugin")
            .build_spec(&args_with("my-plugin"))
            .unwrap();
        assert_eq!(spec2.plugin_name, "MyPlugin");
        assert_eq!(spec2.package_name, "com.example.myplugin");
    }

    #[test]
    fn paper_implies_bukkit_deduplicates() {
        let mut f = form("p", "com.example.p");
        f.platforms = vec!["bukkit".into(), "paper".into(), "bukkit".into()];
        let spec = f.build_spec(&args_with("p")).unwrap();
        assert_eq!(spec.platforms, vec!["paper", "bukkit"]);
    }

    #[test]
    fn bstats_requires_numeric_id() {
        let mut f = form("p", "com.example.p");
        f.features.bstats = true;
        let err = f.build_spec(&args_with("p")).unwrap_err();
        assert_eq!(err.code, "var.invalid_value");
        f.bstats_id = Some("abc".into());
        assert!(f.build_spec(&args_with("p")).is_err());
        f.bstats_id = Some("12345".into());
        assert!(f.build_spec(&args_with("p")).is_ok());
        assert!(is_valid_bstats_id(Some("0")));
        assert!(!is_valid_bstats_id(None));
    }

    #[test]
    fn empty_platform_selection_is_rejected() {
        let mut f = form("p", "com.example.p");
        f.platforms.clear();
        let err = f.build_spec(&args_with("p")).unwrap_err();
        assert_eq!(err.code, "var.invalid_value");
        assert_eq!(err.exit_code(), EXIT_DATA);
    }

    #[test]
    fn target_dir_honours_output_flag() {
        let f = form("p", "com.example.p");
        let mut args = args_with("p");
        args.output = Some("/tmp/elsewhere".into());
        let spec = f.build_spec(&args).unwrap();
        assert_eq!(spec.target_dir, PathBuf::from("/tmp/elsewhere"));
        let spec2 = f.build_spec(&args_with("p")).unwrap();
        assert_eq!(spec2.target_dir, PathBuf::from("p"));
    }

    #[test]
    fn platform_options_keep_proxies_and_canonical_order() {
        let opts = platform_options(&["bukkit".into(), "paper".into(), "folia".into()]);
        assert_eq!(opts, vec!["paper", "bukkit", "velocity", "bungeecord", "folia"]);
        let opts2 = platform_options(&[]);
        assert_eq!(opts2, vec!["velocity", "bungeecord"]);
    }

    #[test]
    fn latest_version_is_the_numeric_max() {
        let versions = vec!["1.8.9".to_string(), "1.21.11".into(), "26.2".into(), "1.12.2".into()];
        assert_eq!(latest_supported(&versions).as_deref(), Some("26.2"));
        assert_eq!(compare_mc("1.21.11", "1.21.9"), std::cmp::Ordering::Greater);
        assert_eq!(compare_mc("26.2", "1.21.11"), std::cmp::Ordering::Greater);
        assert_eq!(compare_mc("1.16.5", "1.16.5"), std::cmp::Ordering::Equal);
    }

    #[test]
    fn metadata_labels_round_trip() {
        for label in METADATA_LABELS {
            assert_eq!(label_for_metadata(metadata_from_label(label)), label);
        }
        assert_eq!(metadata_from_arg(None), MetadataFormat::PluginYml);
        assert_eq!(metadata_from_arg(Some("paper-plugin")), MetadataFormat::PaperPluginYml);
    }

    #[test]
    fn quality_and_negative_flags_map_to_the_spec() {
        let mut args = args_with("my-plugin");
        args.no_example = true;
        args.no_permissions = true;
        args.no_quality = true;
        args.no_git = true;
        args.git = false;
        args.features = vec!["sqlite".into(), "spotbugs".into()];
        let f = WizardForm {
            name: "my-plugin".into(),
            package: "com.example.myplugin".into(),
            mc: "1.21.11".into(),
            platforms: vec!["paper".into()],
            metadata: MetadataFormat::PluginYml,
            example: !args.no_example,
            permissions: !args.no_permissions,
            quality: !args.no_quality,
            git: false,
            features: crate::init::parse_features(&args.features),
            language: ArtifactLanguage::En,
            ui_language: UiLanguage::En,
            bstats_id: None,
            download_jdk: false,
        };
        let spec = f.build_spec(&args).unwrap();
        assert!(!spec.quality.checkstyle && !spec.quality.unit_tests && !spec.quality.ci);
        assert!(!spec.git);
        assert!(spec.features.sqlite && spec.features.spotbugs);
        assert_eq!(spec.language, ArtifactLanguage::En);
    }
}
