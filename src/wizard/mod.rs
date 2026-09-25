//! IDEA-style interactive wizard (spec §4). OWNER: cli-dev.
//!
//! Four pages — ① 工程 ② 构建 ③ 目标服务端 ④ 附加 — with back navigation. Only
//! reachable when stdin/stdout are a TTY and `--yes` was not given; the
//! orchestrator (`src/init.rs`) owns that gate and calls [`should_run`] first.
pub mod form;

use crate::cli::InitArgs;
use crate::error::{Error, Result, EXIT_INTERRUPT, EXIT_TEMPFAIL, EXIT_USAGE};
use crate::matrix::Matrix;
use crate::types::{ArtifactLanguage, MetadataFormat, ProjectSpec, UiLanguage};
use form::{
    label_for_metadata, metadata_from_label, platform_options, validate_package_name,
    validate_project_name, Nav, WizardForm, WIZARD_FEATURES,
};
use inquire::validator::Validation;
use inquire::{Confirm, CustomUserError, InquireError, MultiSelect, Select, Text};

/// TTY + no `--yes` (spec §3.2/§4): the only condition under which pages appear.
pub fn should_run(args: &InitArgs) -> bool {
    use std::io::IsTerminal;
    std::io::stdin().is_terminal() && std::io::stdout().is_terminal() && !args.yes
}

/// Run the four wizard pages and produce the frozen [`ProjectSpec`].
pub fn run(partial: InitArgs, matrix: &Matrix) -> Result<ProjectSpec> {
    let mut form = WizardForm::from_args(&partial, matrix)?;
    let ui = Ui::new(form.ui_language);
    // `page` is the page index; `Nav` decides forward/back within the loop.
    let mut page = 0usize;
    loop {
        match page {
            0 => match page_project(&ui, &mut form)? {
                Nav::Next => page = 1,
                Nav::Back => {}
            },
            1 => match page_build(&ui, &mut form)? {
                Nav::Next => page = 2,
                Nav::Back => page = 0,
            },
            2 => match page_target(&ui, &mut form, matrix)? {
                Nav::Next => page = 3,
                Nav::Back => page = 1,
            },
            _ => match page_extras(&ui, &mut form)? {
                Nav::Next => break,
                Nav::Back => page = 2,
            },
        }
    }
    form.build_spec(&partial)
}

// ── pages ───────────────────────────────────────────────────────────────────

/// ① 工程: name, package; location is derived and shown read-only.
fn page_project(ui: &Ui, form: &mut WizardForm) -> Result<Nav> {
    eprintln!("\n① {}", ui.t("工程", "Project"));

    let name = Text::new(ui.t("名称", "Name"))
        .with_default(&form.name)
        .with_help_message(ui.t(
            "须匹配 ^[A-Za-z][A-Za-z0-9_-]{0,63}$",
            "must match ^[A-Za-z][A-Za-z0-9_-]{0,63}$",
        ))
        .with_validator(|input: &str| match validate_project_name(input) {
            Ok(()) => Ok(Validation::Valid),
            Err(e) => Ok(Validation::Invalid(e.message.into())),
        })
        .prompt();
    form.name = map_inquire(name)?;

    let pkg_default = if form.package.is_empty() {
        format!("com.example.{}", crate::types::slug(&form.name))
    } else {
        form.package.clone()
    };
    let package = Text::new(ui.t("包名", "Package"))
        .with_default(&pkg_default)
        .with_help_message(ui.t(
            "每段 ^[a-z_][a-z0-9_]*$ 且非 Java 关键字",
            "each segment ^[a-z_][a-z0-9_]*$ and not a Java keyword",
        ))
        .with_validator(|input: &str| match validate_package_name(input) {
            Ok(()) => Ok(Validation::Valid),
            Err(e) => Ok(Validation::Invalid(e.message.into())),
        })
        .prompt();
    form.package = map_inquire(package)?;

    eprintln!("  {}: {}", ui.t("位置", "Location"), form.name);
    nav_prompt(ui)
}

/// ② 构建: v1 has exactly one option each; JDK is derived from the matrix and
/// shown by the precheck (spec §4.1).
fn page_build(ui: &Ui, _form: &mut WizardForm) -> Result<Nav> {
    eprintln!("\n② {}", ui.t("构建", "Build"));
    eprintln!("  {}: Java", ui.t("语言", "Language"));
    eprintln!("  {}: Gradle", ui.t("构建系统", "Build system"));
    eprintln!("  Gradle DSL: Kotlin (build.gradle.kts)");
    eprintln!(
        "  JDK: {}",
        ui.t(
            "由所选 MC 版本经矩阵推导（阶段 3.5 预检）",
            "derived from the MC version via the matrix (precheck in phase 3.5)"
        )
    );
    nav_prompt(ui)
}

/// ③ 目标服务端: version first, then only that version's platforms, then metadata.
fn page_target(ui: &Ui, form: &mut WizardForm, matrix: &Matrix) -> Result<Nav> {
    eprintln!("\n③ {}", ui.t("目标服务端", "Target server"));

    let versions = matrix.supported_versions();
    if versions.is_empty() {
        return Err(Error::new(
            "matrix.empty",
            crate::error::EXIT_CONFIG,
            ui.t(
                "内置矩阵没有任何受支持的 MC 版本",
                "the builtin matrix has no supported MC versions",
            ),
        ));
    }
    let v_cursor = versions
        .iter()
        .position(|v| *v == form.mc)
        .unwrap_or(versions.len() - 1);
    let mc = Select::new(ui.t("MC 版本", "Minecraft version"), versions)
        .with_starting_cursor(v_cursor)
        .with_help_message(ui.t("只列受支持的版本", "only supported versions are listed"))
        .prompt();
    form.mc = map_inquire(mc)?;

    // Platforms are filtered by the *currently selected* version; proxies are
    // always present (matrix guarantees it, `platform_options` keeps them).
    let options = platform_options(&matrix.available_platforms(&form.mc));
    form.platforms.retain(|p| options.contains(p));
    loop {
        let defaults: Vec<usize> = options
            .iter()
            .enumerate()
            .filter(|(_, o)| form.platforms.contains(o))
            .map(|(i, _)| i)
            .collect();
        let picked = MultiSelect::new(
            ui.t(
                "平台（勾 paper 自动带上 bukkit）",
                "Platforms (paper implies bukkit)",
            ),
            options.clone(),
        )
        .with_default(&defaults)
        .prompt();
        form.platforms = map_inquire(picked)?;
        if !form.platforms.is_empty() {
            break;
        }
        eprintln!(
            "  ⚠ {}",
            ui.t("至少选择一个平台", "pick at least one platform")
        );
    }
    if form.platforms.iter().any(|p| p == "paper") {
        eprintln!(
            "  · {}",
            ui.t(
                "paper 会自动带上 bukkit 兼容模块",
                "paper implies the bukkit compatibility module"
            )
        );
    }

    let labels = vec![
        label_for_metadata(MetadataFormat::PluginYml).to_string(),
        label_for_metadata(MetadataFormat::PaperPluginYml).to_string(),
    ];
    let m_cursor = match form.metadata {
        MetadataFormat::PluginYml => 0,
        MetadataFormat::PaperPluginYml => 1,
    };
    let metadata = Select::new(ui.t("元数据格式", "Metadata format"), labels)
        .with_starting_cursor(m_cursor)
        .prompt();
    form.metadata = metadata_from_label(&map_inquire(metadata)?);

    nav_prompt(ui)
}

/// ④ 附加: optional modules, bStats id follow-up, defaults-on toggles, language.
fn page_extras(ui: &Ui, form: &mut WizardForm) -> Result<Nav> {
    eprintln!("\n④ {}", ui.t("附加", "Extras"));

    let labels = feature_labels(ui);
    let defaults: Vec<usize> = WIZARD_FEATURES
        .iter()
        .enumerate()
        .filter(|(_, name)| feature_on(&form.features, name))
        .map(|(i, _)| i)
        .collect();
    let picked = MultiSelect::new(ui.t("附加模块", "Add-on modules"), labels.clone())
        .with_default(&defaults)
        .prompt();
    let picked = map_inquire(picked)?;
    for (i, name) in WIZARD_FEATURES.iter().enumerate() {
        set_feature(&mut form.features, name, picked.contains(&labels[i]));
    }

    // bStats: the wizard asks for the numeric plugin id (spec §4.3).
    if form.features.bstats {
        let current = form.bstats_id.clone().unwrap_or_default();
        loop {
            let id = Text::new(ui.t("bStats 数字 plugin id", "bStats numeric plugin id"))
                .with_default(&current)
                .with_help_message(ui.t(
                    "在 bstats.org 新建插件后取得；缺失会硬报错",
                    "from your plugin page on bstats.org; a missing id is a hard error",
                ))
                .with_validator(numeric_validator)
                .prompt();
            let value = map_inquire(id)?;
            if form::is_valid_bstats_id(Some(&value)) {
                form.bstats_id = Some(value);
                break;
            }
        }
    }

    form.example = map_inquire(
        Confirm::new(ui.t(
            "示例代码（一条命令 + 一个监听器 + config）",
            "Example code (a command + a listener + config)",
        ))
        .with_default(form.example)
        .prompt(),
    )?;
    form.permissions = map_inquire(
        Confirm::new(ui.t(
            "权限声明（permissions + 权限常量类）",
            "Permissions (permissions block + constants class)",
        ))
        .with_default(form.permissions)
        .prompt(),
    )?;
    form.quality = map_inquire(
        Confirm::new(ui.t(
            "质量工程（checkstyle + JUnit + CI）",
            "Quality engineering (checkstyle + JUnit + CI)",
        ))
        .with_default(form.quality)
        .prompt(),
    )?;
    form.git = map_inquire(
        Confirm::new(ui.t("git init + 首次提交", "git init + first commit"))
            .with_default(form.git)
            .prompt(),
    )?;

    let lang_labels = vec![
        ui.t("中文", "Chinese").to_string(),
        "English".to_string(),
        ui.t("双语", "Both").to_string(),
    ];
    let lang_cursor = match form.language {
        ArtifactLanguage::Zh => 0,
        ArtifactLanguage::En => 1,
        ArtifactLanguage::Both => 2,
    };
    let lang = Select::new(ui.t("生成物语言", "Artifact language"), lang_labels)
        .with_starting_cursor(lang_cursor)
        .prompt();
    form.language = match map_inquire(lang)?.as_str() {
        "English" => ArtifactLanguage::En,
        "双语" | "Both" => ArtifactLanguage::Both,
        _ => ArtifactLanguage::Zh,
    };

    nav_prompt(ui)
}

// ── plumbing ────────────────────────────────────────────────────────────────

fn nav_prompt(ui: &Ui) -> Result<Nav> {
    let options = vec![
        ui.t("下一步", "Next").to_string(),
        ui.t("返回上一页", "Back").to_string(),
    ];
    let back = options[1].clone();
    let choice = Select::new(ui.t("继续？", "Continue?"), options).prompt();
    Ok(if map_inquire(choice)? == back { Nav::Back } else { Nav::Next })
}

fn feature_labels(ui: &Ui) -> Vec<String> {
    vec![
        ui.t("SQLite 持久化", "SQLite persistence").to_string(),
        ui.t("bStats 统计", "bStats metrics").to_string(),
        ui.t("更新检查", "Update check").to_string(),
        ui.t("PlaceholderAPI", "PlaceholderAPI").to_string(),
        ui.t("GUI 菜单骨架", "GUI menu skeleton").to_string(),
    ]
}

fn feature_on(features: &crate::types::FeatureSet, name: &str) -> bool {
    match name {
        "sqlite" => features.sqlite,
        "bstats" => features.bstats,
        "update-check" => features.update_check,
        "placeholderapi" => features.placeholderapi,
        "gui" => features.gui,
        _ => false,
    }
}

fn set_feature(features: &mut crate::types::FeatureSet, name: &str, on: bool) {
    match name {
        "sqlite" => features.sqlite = on,
        "bstats" => features.bstats = on,
        "update-check" => features.update_check = on,
        "placeholderapi" => features.placeholderapi = on,
        "gui" => features.gui = on,
        _ => {}
    }
}

fn numeric_validator(input: &str) -> std::result::Result<Validation, CustomUserError> {
    if input.chars().all(|c| c.is_ascii_digit()) && !input.is_empty() {
        Ok(Validation::Valid)
    } else {
        Ok(Validation::Invalid("必须是数字 plugin id（例如 12345）".into()))
    }
}

/// `Esc` cancels (exit 130), Ctrl-C interrupts, non-TTY is a usage error.
fn map_inquire<T>(result: std::result::Result<T, InquireError>) -> Result<T> {
    result.map_err(|e| match e {
        InquireError::OperationCanceled => {
            Error::new("interrupted", EXIT_INTERRUPT, "已取消（Esc）：未写入任何文件")
                .with_hint("复现: vinoa init <名称> -m <版本> --platform <平台> -y")
        }
        InquireError::OperationInterrupted => {
            Error::new("interrupted", EXIT_INTERRUPT, "已中断（Ctrl-C）：未写入任何文件")
        }
        InquireError::NotTTY => Error::new(
            "usage.invalid",
            EXIT_USAGE,
            "向导需要 TTY；非交互请用 --yes 或显式参数",
        ),
        other => Error::new("input.failed", EXIT_TEMPFAIL, other.to_string()),
    })
}

/// Which language the wizard chrome speaks; independent of the artifact language.
struct Ui {
    lang: UiLanguage,
}

impl Ui {
    fn new(lang: UiLanguage) -> Self {
        Self { lang }
    }

    fn t(&self, zh: &'static str, en: &'static str) -> &'static str {
        match self.lang {
            UiLanguage::Zh => zh,
            UiLanguage::En => en,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_run_requires_tty_and_no_yes() {
        // Under `cargo test` stdin/stdout are not a TTY, so this is false; `--yes`
        // must keep it false regardless.
        let mut args = InitArgs::default();
        assert!(!should_run(&args));
        args.yes = true;
        assert!(!should_run(&args));
    }

    #[test]
    fn ui_strings_follow_ui_language() {
        let zh = Ui::new(UiLanguage::Zh);
        let en = Ui::new(UiLanguage::En);
        assert_eq!(zh.t("工程", "Project"), "工程");
        assert_eq!(en.t("工程", "Project"), "Project");
    }

    #[test]
    fn feature_labels_align_with_canonical_names() {
        let labels = feature_labels(&Ui::new(UiLanguage::En));
        assert_eq!(labels.len(), WIZARD_FEATURES.len());
        assert_eq!(labels[0], "SQLite persistence");
        assert_eq!(labels[4], "GUI menu skeleton");
        let mut f = crate::types::FeatureSet::default();
        set_feature(&mut f, "bstats", true);
        set_feature(&mut f, "gui", true);
        assert!(feature_on(&f, "bstats") && feature_on(&f, "gui"));
        assert!(!feature_on(&f, "sqlite"));
    }

    #[test]
    fn numeric_validator_accepts_only_digits() {
        assert!(matches!(numeric_validator("12345"), Ok(Validation::Valid)));
        assert!(matches!(numeric_validator(""), Ok(Validation::Invalid(_))));
        assert!(matches!(numeric_validator("12a"), Ok(Validation::Invalid(_))));
    }

    #[test]
    fn cancelled_prompts_map_to_exit_130() {
        let err = map_inquire::<()>(Err(InquireError::OperationCanceled)).unwrap_err();
        assert_eq!(err.exit_code(), EXIT_INTERRUPT);
        assert_eq!(err.code, "interrupted");
        let err = map_inquire::<()>(Err(InquireError::NotTTY)).unwrap_err();
        assert_eq!(err.exit_code(), EXIT_USAGE);
    }
}
