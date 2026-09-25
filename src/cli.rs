//! Command line surface (spec §3). OWNER: cli-dev.
//!
//! Contract shapes consumed by the orchestrator (`src/init.rs`, lead-owned):
//! [`InitArgs`] / [`VersionsArgs`] / [`SchemaArgs`] / [`Command`]. Field names of
//! [`InitArgs`] are stable; new fields are only ever *appended*.
use crate::error::{self, Result, EXIT_CONFIG, EXIT_OK, EXIT_VERIFY_FAILED};
use crate::types::PLATFORMS;
use clap::{ArgAction, Args, CommandFactory, FromArgMatches, Parser, Subcommand};
use std::process::ExitCode;

/// `--features` values (spec §3.3): five add-on modules + three quality sub-items.
pub const FEATURE_NAMES: [&str; 8] = [
    "sqlite",
    "bstats",
    "update-check",
    "placeholderapi",
    "gui",
    "spotbugs",
    "coverage",
    "release-ci",
];

/// Metadata formats accepted on the command line (spec §3.1). The help text says
/// `plugin.yml | paper-plugin`; the `.yml` spellings are accepted keys for `-c`.
pub const METADATA_FORMATS: [&str; 3] = ["plugin.yml", "paper-plugin", "paper-plugin.yml"];

/// Frozen exit-code table (spec §11.2). Used by `vinoa schema` and tests.
pub const EXIT_TABLE: [(&str, u8, &str); 9] = [
    ("SUCCESS", crate::error::EXIT_OK, "生成成功；带 --verify 时构建也通过"),
    ("VERIFY_FAILED", EXIT_VERIFY_FAILED, "生成成功、验证失败（构建失败 / 超时 / 离线取不到依赖）"),
    ("USAGE", crate::error::EXIT_USAGE, "非法 flag / 缺必填参数且无法询问"),
    ("DATAERR", crate::error::EXIT_DATA, "名称/包名非法，或 (mc, platform) 组合不存在"),
    ("CANTCREAT", crate::error::EXIT_CANTCREAT, "目标目录非空且未给 --output，或无写权限"),
    ("IOERR", crate::error::EXIT_IO, "落盘中途失败，已回滚"),
    ("TEMPFAIL", crate::error::EXIT_TEMPFAIL, "JDK 缺失且不允许继续生成；或临时资源不可用"),
    ("CONFIG", EXIT_CONFIG, "内置模板/矩阵损坏，或 -c 配置文件语法错"),
    ("INTERRUPTED", crate::error::EXIT_INTERRUPT, "Ctrl-C / SIGINT：原子落盘已回滚"),
];

/// Default Fill API base for `versions --refresh` (spec §6.4).
pub const FILL_API_BASE: &str = "https://fill.papermc.io/v3";

#[derive(Debug)]
pub enum Command {
    Init(Box<InitArgs>),
    Versions(VersionsArgs),
    Schema(SchemaArgs),
    /// `--help` / `--version` / bare `vinoa`: clap already printed; exit 0.
    Help,
}

#[derive(Debug, Clone)]
pub struct InitArgs {
    pub name: Option<String>,
    pub package: Option<String>,
    pub mc: Option<String>,
    pub platforms: Vec<String>,
    pub features: Vec<String>,
    pub metadata: Option<String>,
    pub language: Option<String>,
    pub ui_language: Option<String>,
    pub license: Option<String>,
    pub author: Vec<String>,
    pub description: Option<String>,
    pub output: Option<String>,
    pub config: Option<String>,
    pub print_config: bool,
    pub bstats_id: Option<String>,
    pub yes: bool,
    pub dry_run: bool,
    pub json: bool,
    pub verify: bool,
    /// Effective git flag: `true` unless `--no-git` (spec §9.7 default on).
    pub git: bool,
    pub no_git: bool,
    pub no_example: bool,
    pub no_permissions: bool,
    pub no_quality: bool,
    pub download_jdk: Option<bool>,
    /// `--template <路径|git-url#ref>` (spec §7.9).
    pub template: Option<String>,
    /// `website` is `-c`-only (spec §3.1); kept for the config merge.
    pub website: Option<String>,
    /// Allow `--verify` to reach the network (spec §11.4).
    pub verify_online: bool,
    /// Log tail line count for `--verify` (default 20, cap 200).
    pub verify_tail: Option<usize>,
    /// Matrix source flags (spec §6.4): offline is the default.
    pub offline: bool,
    pub online: bool,
}

impl Default for InitArgs {
    fn default() -> Self {
        Self {
            name: None,
            package: None,
            mc: None,
            platforms: Vec::new(),
            features: Vec::new(),
            metadata: None,
            language: None,
            ui_language: None,
            license: None,
            author: Vec::new(),
            description: None,
            output: None,
            config: None,
            print_config: false,
            bstats_id: None,
            yes: false,
            dry_run: false,
            json: false,
            verify: false,
            git: true,
            no_git: false,
            no_example: false,
            no_permissions: false,
            no_quality: false,
            download_jdk: None,
            template: None,
            website: None,
            verify_online: false,
            verify_tail: None,
            offline: false,
            online: false,
        }
    }
}

impl InitArgs {
    /// Whether an add-on module / quality sub-item was selected via `--features`.
    pub fn feature_enabled(&self, name: &str) -> bool {
        self.features.iter().any(|f| f == name)
    }
}

#[derive(Debug, Default, Clone)]
pub struct VersionsArgs {
    pub refresh: bool,
    pub matrix: bool,
    pub mc: Option<String>,
    pub platform: Option<String>,
    pub json: bool,
}

#[derive(Debug, Default, Clone)]
pub struct SchemaArgs {
    pub json: bool,
}

/// Inner `[名称] / 常用 / 高级` surface of `vinoa init`, rendered with the
/// spec §3.1 headings.
#[derive(Args, Debug, Default, Clone)]
#[command(override_usage = "用法: vinoa init [名称] [选项]")]
#[command(help_template = "{usage}\n\n{all-args}{after-help}")]
#[command(after_help = "示例:\n  vinoa init my-plugin -p com.example.myplugin -m 1.21.11 --platform paper --features sqlite,bstats -y\n\n退出码: 0 成功 · 2 验证失败 · 64 用法 · 65 数据 · 73 无法创建 · 74 IO · 75 临时失败 · 78 配置 · 130 中断")]
struct InitCli {
    /// 工程名；省略时取当前目录名（就地初始化）
    #[arg(value_name = "名称", help_heading = "参数")]
    name: Option<String>,

    /// Java 包名（默认 com.example.<名称去连字符>）
    #[arg(short = 'p', long = "package", value_name = "包名", help_heading = "常用")]
    package: Option<String>,

    /// 目标 Minecraft 版本（如 1.21.11 / 26.2）
    #[arg(short = 'm', long = "mc", value_name = "版本", help_heading = "常用")]
    mc: Option<String>,

    /// 目标平台，可重复：paper,bukkit,velocity,bungeecord,folia,sponge,minestom
    #[arg(long = "platform", value_name = "平台", value_delimiter = ',', action = ArgAction::Append, help_heading = "常用")]
    platform: Vec<String>,

    /// 附加模块，逗号分隔（sqlite,bstats,update-check,placeholderapi,gui,spotbugs,coverage,release-ci）
    #[arg(long = "features", value_name = "列表", value_delimiter = ',', action = ArgAction::Append, help_heading = "常用")]
    features: Vec<String>,

    /// 输出到指定目录（目标目录非空时的首选处置）
    #[arg(short = 'o', long = "output", value_name = "目录", help_heading = "常用")]
    output: Option<String>,

    /// 全部用默认值，不询问
    #[arg(short = 'y', long = "yes", action = ArgAction::SetTrue, help_heading = "常用")]
    yes: bool,

    /// 只打印将要生成的内容（走同一条渲染路径）
    #[arg(long = "dry-run", action = ArgAction::SetTrue, help_heading = "常用")]
    dry_run: bool,

    /// 以 JSON 输出（给脚本 / agent 解析），恰好一个文档
    #[arg(long = "json", action = ArgAction::SetTrue, help_heading = "常用")]
    json: bool,

    /// 生成后跑一次构建验证（默认离线）
    #[arg(long = "verify", action = ArgAction::SetTrue, help_heading = "常用")]
    verify: bool,

    /// 从 TOML 读取全部答案（与向导等价，可版本化、可进仓库）
    #[arg(short = 'c', long = "config", value_name = "文件", help_heading = "高级")]
    config: Option<String>,

    /// 把本次解析出的答案导成 TOML
    #[arg(long = "print-config", action = ArgAction::SetTrue, help_heading = "高级")]
    print_config: bool,

    /// plugin.yml | paper-plugin（默认 plugin.yml）
    #[arg(long = "metadata", value_name = "格式", help_heading = "高级")]
    metadata: Option<String>,

    /// 默认 Apache-2.0；其它值报 template.license_unsupported
    #[arg(long = "license", value_name = "SPDX", help_heading = "高级")]
    license: Option<String>,

    /// 作者（可重复）
    #[arg(long = "author", value_name = "作者", action = ArgAction::Append, help_heading = "高级")]
    author: Vec<String>,

    /// 描述
    #[arg(long = "description", value_name = "描述", help_heading = "高级")]
    description: Option<String>,

    /// 生成物语言：zh | en | both（默认 zh）
    #[arg(long = "lang", value_name = "zh|en|both", help_heading = "高级")]
    lang: Option<String>,

    /// 界面语言：zh | en（默认跟随系统）
    #[arg(long = "ui-lang", value_name = "zh|en", help_heading = "高级")]
    ui_lang: Option<String>,

    /// 勾选 bStats 时必填；缺失硬报错，绝不生成假 id
    #[arg(long = "bstats-id", value_name = "数字", help_heading = "高级")]
    bstats_id: Option<String>,

    /// 整套替换模板集（本地路径 | git-url#ref）
    #[arg(long = "template", value_name = "路径|git-url#ref", help_heading = "高级")]
    template: Option<String>,

    /// 缺 Java 时让 Gradle 首次构建自动下载
    #[arg(long = "download-jdk", action = ArgAction::SetTrue, conflicts_with = "no_download_jdk", help_heading = "高级")]
    download_jdk: bool,

    /// 缺 Java 时不自动下载（--yes 的保守默认）
    #[arg(long = "no-download-jdk", action = ArgAction::SetTrue, conflicts_with = "download_jdk", help_heading = "高级")]
    no_download_jdk: bool,

    /// 允许 --verify 联网（默认离线）
    #[arg(long = "verify-online", action = ArgAction::SetTrue, help_heading = "高级")]
    verify_online: bool,

    /// 日志尾部行数（默认 20，上限 200）
    #[arg(long = "verify-tail", value_name = "N", help_heading = "高级")]
    verify_tail: Option<usize>,

    /// 矩阵取内置（默认）
    #[arg(long = "offline", action = ArgAction::SetTrue, conflicts_with = "online", help_heading = "高级")]
    offline: bool,

    /// 矩阵单次联网（§6.4）
    #[arg(long = "online", action = ArgAction::SetTrue, conflicts_with = "offline", help_heading = "高级")]
    online: bool,

    /// 关闭 git init + 首次提交
    #[arg(long = "no-git", action = ArgAction::SetTrue, help_heading = "高级")]
    no_git: bool,

    /// 关闭示例代码
    #[arg(long = "no-example", action = ArgAction::SetTrue, help_heading = "高级")]
    no_example: bool,

    /// 关闭权限声明
    #[arg(long = "no-permissions", action = ArgAction::SetTrue, help_heading = "高级")]
    no_permissions: bool,

    /// 关闭质量工程
    #[arg(long = "no-quality", action = ArgAction::SetTrue, help_heading = "高级")]
    no_quality: bool,
}

#[derive(Args, Debug, Default, Clone)]
#[command(override_usage = "用法: vinoa versions [--refresh] [--matrix] [--mc <版本>] [--platform <平台>] [--json]")]
#[command(help_template = "{usage}\n\n{all-args}{after-help}")]
struct VersionsCli {
    /// 从 fill.papermc.io/v3 拉取并写入用户缓存目录
    #[arg(long = "refresh", action = ArgAction::SetTrue)]
    refresh: bool,

    /// 打印内置矩阵全表
    #[arg(long = "matrix", action = ArgAction::SetTrue)]
    matrix: bool,

    /// 按 MC 版本过滤
    #[arg(long = "mc", value_name = "版本")]
    mc: Option<String>,

    /// 按平台过滤
    #[arg(long = "platform", value_name = "平台")]
    platform: Option<String>,

    /// 以 JSON 输出
    #[arg(long = "json", action = ArgAction::SetTrue)]
    json: bool,
}

#[derive(Args, Debug, Default, Clone)]
#[command(override_usage = "用法: vinoa schema [--json]")]
#[command(help_template = "{usage}\n\n{all-args}")]
struct SchemaCli {
    /// 以 JSON 输出命令面 / 退出码 / 错误码契约
    #[arg(long = "json", action = ArgAction::SetTrue)]
    json: bool,
}

#[derive(Parser, Debug)]
#[command(
    name = "vinoa",
    version,
    about = "Scaffold Minecraft server plugin projects",
    disable_help_subcommand = true
)]
struct Cli {
    #[command(subcommand)]
    command: Option<TopCommand>,
}

#[derive(Subcommand, Debug)]
enum TopCommand {
    /// 生成一个新的插件工程（spec §3.1）
    Init(InitCli),
    /// 打印版本矩阵 / 刷新缓存（spec §3.4）
    Versions(VersionsCli),
    /// 打印机器可读的命令契约
    Schema(SchemaCli),
}

/// Derived command with the auto `-h/--help` rows hidden: the spec §3.1 table
/// lists only the documented arguments, and `{all-args}` would otherwise print a
/// stray `Options:` group. Shared by [`parse`] and the help tests.
pub fn command() -> clap::Command {
    let mut cmd = Cli::command();
    cmd.build(); // materialise the default help args before mutating them
    cmd.mut_subcommand("init", |c| c.mut_arg("help", |a| a.hide(true)))
        .mut_subcommand("versions", |c| c.mut_arg("help", |a| a.hide(true)))
        .mut_subcommand("schema", |c| c.mut_arg("help", |a| a.hide(true)))
}

pub fn parse() -> Result<Command> {
    // Heuristic: even a clap-level failure must honour `--json` (spec §11.5).
    if std::env::args().any(|a| a == "--json") {
        crate::report::set_json_mode(true);
    }
    let argv: Vec<String> = std::env::args().collect();
    let cmd = command();
    let cli = match cmd.try_get_matches_from(&argv) {
        Ok(matches) => match Cli::from_arg_matches(&matches) {
            Ok(cli) => cli,
            Err(err) => return Err(error::usage(err.to_string())),
        },
        Err(err) => {
            use clap::error::ErrorKind;
            match err.kind() {
                ErrorKind::DisplayHelp
                | ErrorKind::DisplayVersion
                | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand => {
                    let _ = err.print();
                    return Ok(Command::Help);
                }
                _ => return Err(error::usage(err.to_string())),
            }
        }
    };
    match cli.command {
        Some(TopCommand::Init(init)) => {
            let args = init_args(init)?;
            if args.json {
                crate::report::set_json_mode(true);
            }
            Ok(Command::Init(Box::new(args)))
        }
        Some(TopCommand::Versions(v)) => {
            if v.json {
                crate::report::set_json_mode(true);
            }
            Ok(Command::Versions(VersionsArgs {
                refresh: v.refresh,
                matrix: v.matrix,
                mc: v.mc,
                platform: v.platform,
                json: v.json,
            }))
        }
        Some(TopCommand::Schema(s)) => {
            if s.json {
                crate::report::set_json_mode(true);
            }
            Ok(Command::Schema(SchemaArgs { json: s.json }))
        }
        None => {
            let mut cmd = Cli::command();
            let _ = cmd.print_help();
            println!();
            Ok(Command::Help)
        }
    }
}

fn init_args(cli: InitCli) -> Result<InitArgs> {
    for p in &cli.platform {
        if !PLATFORMS.contains(&p.as_str()) {
            return Err(error::usage(format!(
                "未知平台 `{p}`；可用平台: {}",
                PLATFORMS.join(", ")
            ))
            .with_hint("复现: vinoa init --help"));
        }
    }
    for f in &cli.features {
        if !FEATURE_NAMES.contains(&f.as_str()) {
            return Err(error::usage(format!(
                "未知 --features 取值 `{f}`；可用值: {}",
                FEATURE_NAMES.join(", ")
            ))
            .with_hint("复现: vinoa init --help"));
        }
    }
    if let Some(m) = &cli.metadata {
        if !METADATA_FORMATS.contains(&m.as_str()) {
            return Err(error::usage(format!(
                "未知 --metadata 取值 `{m}`；可用值: {}",
                METADATA_FORMATS.join(", ")
            )));
        }
    }
    if let Some(l) = &cli.lang {
        if !["zh", "en", "both"].contains(&l.as_str()) {
            return Err(error::usage(format!(
                "未知 --lang 取值 `{l}`；可用值: zh, en, both"
            )));
        }
    }
    if let Some(l) = &cli.ui_lang {
        if !["zh", "en"].contains(&l.as_str()) {
            return Err(error::usage(format!(
                "未知 --ui-lang 取值 `{l}`；可用值: zh, en"
            )));
        }
    }
    if let Some(id) = &cli.bstats_id {
        if id.is_empty() || !id.chars().all(|c| c.is_ascii_digit()) {
            return Err(
                error::usage(format!("--bstats-id 必须是数字 plugin id，得到 `{id}`"))
                    .with_hint("在 bstats.org 插件页取得数字 id"),
            );
        }
    }
    let download_jdk = if cli.download_jdk {
        Some(true)
    } else if cli.no_download_jdk {
        Some(false)
    } else {
        None
    };
    let mut platforms = cli.platform.clone();
    platforms.dedup();
    // Canonicalise for downstream consumers: `--metadata paper-plugin` and
    // `paper-plugin.yml` are the same format.
    let metadata = cli.metadata.map(|m| {
        if m.starts_with("paper") {
            "paper-plugin.yml".to_string()
        } else {
            m
        }
    });
    Ok(InitArgs {
        name: cli.name,
        package: cli.package,
        mc: cli.mc,
        platforms,
        features: cli.features,
        metadata,
        language: cli.lang,
        ui_language: cli.ui_lang,
        license: cli.license,
        author: cli.author,
        description: cli.description,
        output: cli.output,
        config: cli.config,
        print_config: cli.print_config,
        bstats_id: cli.bstats_id,
        yes: cli.yes,
        dry_run: cli.dry_run,
        json: cli.json,
        verify: cli.verify,
        git: !cli.no_git,
        no_git: cli.no_git,
        no_example: cli.no_example,
        no_permissions: cli.no_permissions,
        no_quality: cli.no_quality,
        download_jdk,
        template: cli.template,
        website: None,
        verify_online: cli.verify_online,
        verify_tail: cli.verify_tail.map(|n| n.min(200)),
        offline: cli.offline || !cli.online,
        online: cli.online,
    })
}

pub fn dispatch(command: Command) -> Result<ExitCode> {
    match command {
        Command::Init(args) => crate::init::run(*args),
        Command::Versions(args) => run_versions(args),
        Command::Schema(args) => run_schema(args),
        Command::Help => Ok(ExitCode::SUCCESS),
    }
}

fn run_versions(args: VersionsArgs) -> Result<ExitCode> {
    let matrix = crate::matrix::Matrix::builtin()?;
    let mut source = "builtin".to_string();
    let mut warnings: Vec<String> = Vec::new();

    if args.refresh {
        match crate::matrix::refresh::refresh(FILL_API_BASE) {
            Ok(report) => source = report.source,
            Err(err) => {
                warnings.push(format!("矩阵联网刷新失败，回退内置矩阵（不阻塞 init）: {err}"));
                source = "builtin".to_string();
            }
        }
    }

    let mut versions = matrix.supported_versions();
    if let Some(mc) = &args.mc {
        versions.retain(|v| v == mc);
    }
    if let Some(p) = &args.platform {
        if !PLATFORMS.contains(&p.as_str()) {
            return Err(error::usage(format!(
                "未知平台 `{p}`；可用平台: {}",
                PLATFORMS.join(", ")
            )));
        }
        versions.retain(|mc| matrix.platform_plan(mc, p).is_some());
    }

    if args.json {
        let rows: Vec<serde_json::Value> = versions
            .iter()
            .map(|mc| {
                let mut platforms = serde_json::Map::new();
                if args.platform.is_none() {
                    for p in crate::matrix::available_platforms(&matrix, mc) {
                        if let Some(plan) = matrix.platform_plan(mc, &p) {
                            platforms.insert(
                                p,
                                serde_json::json!({
                                    "api": plan.api_coordinate,
                                    "java": plan.java_target,
                                    "experimental": plan.experimental,
                                }),
                            );
                        }
                    }
                }
                serde_json::json!({ "mc": mc, "platforms": platforms })
            })
            .collect();
        crate::report::emit_json(&serde_json::json!({
            "schema": "vinoa.versions/v1",
            "vinoa": env!("CARGO_PKG_VERSION"),
            "ok": true,
            "command": "versions",
            "status": "ok",
            "exit_code": EXIT_OK,
            "source": source,
            "generated_at": matrix.generated_at(),
            "versions": rows,
            "warnings": warnings,
        }));
        return Ok(ExitCode::from(EXIT_OK));
    }

    let mut out = String::new();
    out.push_str(&format!(
        "矩阵来源: {source}（generated_at {}）\n",
        matrix.generated_at()
    ));
    if args.matrix {
        for mc in &versions {
            out.push_str(&format!("{mc}\n"));
            for p in PLATFORMS {
                if let Some(plan) = matrix.platform_plan(mc, p) {
                    out.push_str(&format!(
                        "  {p:<11} java {}  {}{}\n",
                        plan.java_target,
                        plan.api_coordinate,
                        if plan.experimental { "  (experimental)" } else { "" }
                    ));
                }
            }
        }
    } else {
        out.push_str(&format!("受支持版本: {}\n", versions.join(", ")));
    }
    crate::report::info(&out);
    for w in warnings {
        crate::report::warn(&w);
    }
    Ok(ExitCode::from(EXIT_OK))
}

fn run_schema(_args: SchemaArgs) -> Result<ExitCode> {
    let exit_codes: Vec<serde_json::Value> = EXIT_TABLE
        .iter()
        .map(|(name, code, when)| serde_json::json!({ "name": name, "code": code, "when": when }))
        .collect();
    let doc = serde_json::json!({
        "schema": "vinoa.schema/v1",
        "vinoa": env!("CARGO_PKG_VERSION"),
        "commands": {
            "init": {
                "usage": "vinoa init [名称] [选项]",
                "positional": ["名称"],
                "common": ["-p/--package", "-m/--mc", "--platform", "--features", "-o/--output", "-y/--yes", "--dry-run", "--json", "--verify"],
                "advanced": ["-c/--config", "--print-config", "--metadata", "--license", "--author", "--description", "--lang", "--ui-lang", "--bstats-id", "--template", "--download-jdk", "--no-download-jdk", "--verify-online", "--verify-tail", "--offline", "--online", "--no-git", "--no-example", "--no-permissions", "--no-quality"],
                "platforms": PLATFORMS,
                "features": FEATURE_NAMES,
                "metadata": METADATA_FORMATS,
            },
            "versions": { "usage": "vinoa versions [--refresh] [--matrix] [--mc <版本>] [--platform <平台>] [--json]" },
            "schema": { "usage": "vinoa schema [--json]" }
        },
        "exit_codes": exit_codes,
        "error_codes": [
            "template.not_found", "template.manifest_invalid", "template.manifest_unknown_key",
            "template.version_too_old", "template.undefined_variable", "template.variable_mismatch",
            "template.bad_path_var", "template.bad_target", "template.i18n_missing_key",
            "template.license_unsupported", "template.too_large", "var.invalid_value",
            "render.syntax_error", "render.leftover_placeholder", "render.stale_default",
            "render.main_class_mismatch", "render.module_set_mismatch", "render.assertion_failed",
            "write.exists", "write.io", "verify_failed"
        ],
        "json_documents": ["vinoa.init/v1", "vinoa.versions/v1", "vinoa.schema/v1"],
    });
    crate::report::emit_json(&doc);
    Ok(ExitCode::from(EXIT_OK))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init_help() -> String {
        let mut cmd = command();
        cmd.find_subcommand_mut("init").unwrap().render_help().to_string()
    }

    #[test]
    fn help_has_spec_sections_and_flags() {
        let h = init_help();
        assert!(
            h.starts_with("用法: vinoa init [名称] [选项]"),
            "usage line missing:\n{h}"
        );
        // No clap default "Options:" leftovers in the rendered table.
        assert!(!h.contains("Options:"), "clap default heading leaked:\n{h}");
        assert!(!h.contains("Print help"), "clap default help row leaked:\n{h}");
        for section in ["参数:", "常用:", "高级:"] {
            assert!(h.contains(section), "missing section {section}:\n{h}");
        }
        // Spec §3.1 flags, in order.
        let ordered = [
            "-p, --package <包名>",
            "-m, --mc <版本>",
            "--platform <平台>",
            "--features <列表>",
            "-o, --output <目录>",
            "-y, --yes",
            "--dry-run",
            "--json",
            "--verify",
            "-c, --config <文件>",
            "--print-config",
            "--metadata <格式>",
            "--license <SPDX>",
            "--author <作者>",
            "--description <描述>",
            "--lang <zh|en|both>",
            "--ui-lang <zh|en>",
            "--bstats-id <数字>",
            "--template <路径|git-url#ref>",
            "--download-jdk",
            "--no-download-jdk",
            "--verify-online",
            "--verify-tail <N>",
            "--offline",
            "--online",
            "--no-git",
            "--no-example",
            "--no-permissions",
            "--no-quality",
        ];
        let mut last = 0usize;
        for token in ordered {
            let at = h.find(token).unwrap_or_else(|| panic!("missing in help: {token}\n{h}"));
            assert!(at >= last, "help order drifted at {token}\n{h}");
            last = at;
        }
    }

    #[test]
    fn parses_repeatable_flags() {
        let cmd = Cli::try_parse_from([
            "vinoa",
            "init",
            "my-plugin",
            "--platform",
            "paper",
            "--platform",
            "velocity",
            "--features",
            "sqlite,bstats",
            "-y",
            "--json",
            "--no-git",
            "--no-download-jdk",
            "--verify-tail",
            "500",
        ])
        .unwrap();
        let Some(TopCommand::Init(c)) = cmd.command else { panic!("expected init") };
        let args = init_args(c).unwrap();
        assert_eq!(args.name.as_deref(), Some("my-plugin"));
        assert_eq!(args.platforms, vec!["paper", "velocity"]);
        assert_eq!(args.features, vec!["sqlite", "bstats"]);
        assert!(args.yes && args.json && args.no_git && !args.git);
        assert_eq!(args.download_jdk, Some(false));
        assert_eq!(args.verify_tail, Some(200)); // clamped to the spec cap
        assert!(args.offline && !args.online);
    }

    #[test]
    fn rejects_unknown_values_with_usage_exit() {
        for argv in [
            vec!["vinoa", "init", "--platform", "nukkit"],
            vec!["vinoa", "init", "--features", "nope"],
            vec!["vinoa", "init", "--metadata", "nope"],
            vec!["vinoa", "init", "--lang", "fr"],
            vec!["vinoa", "init", "--bstats-id", "abc"],
        ] {
            let cmd = Cli::try_parse_from(&argv).unwrap();
            let Some(TopCommand::Init(c)) = cmd.command else { panic!() };
            let err = init_args(c).unwrap_err();
            assert_eq!(err.exit_code(), crate::error::EXIT_USAGE, "{argv:?}");
        }
    }

    #[test]
    fn conflicting_flags_are_usage_errors() {
        assert!(
            Cli::try_parse_from(["vinoa", "init", "--download-jdk", "--no-download-jdk"]).is_err()
        );
        assert!(Cli::try_parse_from(["vinoa", "init", "--offline", "--online"]).is_err());
    }

    #[test]
    fn exit_code_table_matches_spec() {
        let expected = [
            ("SUCCESS", 0u8),
            ("VERIFY_FAILED", 2),
            ("USAGE", 64),
            ("DATAERR", 65),
            ("CANTCREAT", 73),
            ("IOERR", 74),
            ("TEMPFAIL", 75),
            ("CONFIG", 78),
            ("INTERRUPTED", 130),
        ];
        assert_eq!(EXIT_TABLE.len(), expected.len());
        for ((name, code, _), (ename, ecode)) in EXIT_TABLE.iter().zip(expected) {
            assert_eq!((*name, *code), (ename, ecode));
        }
        assert_eq!(EXIT_OK, crate::error::EXIT_OK);
        assert_eq!(EXIT_VERIFY_FAILED, crate::error::EXIT_VERIFY_FAILED);
        assert_eq!(EXIT_CONFIG, crate::error::EXIT_CONFIG);
    }

    #[test]
    fn versions_help_lists_all_options() {
        let mut cmd = command();
        let h = cmd.find_subcommand_mut("versions").unwrap().render_help().to_string();
        for token in ["--refresh", "--matrix", "--mc <版本>", "--platform <平台>", "--json"] {
            assert!(h.contains(token), "missing {token} in:\n{h}");
        }
    }
}
