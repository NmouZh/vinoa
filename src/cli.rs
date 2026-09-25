//! Command line surface (spec §3). OWNER: cli-dev.
use crate::error::Result;

#[derive(Debug)]
pub enum Command {
    Init(Box<InitArgs>),
    Versions(VersionsArgs),
    Schema(SchemaArgs),
}

#[derive(Debug, Default, Clone)]
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
    pub git: bool,
    pub no_git: bool,
    pub no_example: bool,
    pub no_permissions: bool,
    pub no_quality: bool,
    pub download_jdk: Option<bool>,
}

#[derive(Debug, Default, Clone)]
pub struct VersionsArgs {
    pub refresh: bool,
    pub json: bool,
}

#[derive(Debug, Default, Clone)]
pub struct SchemaArgs {
    pub command: Option<String>,
}

pub fn parse() -> Result<Command> {
    todo!("cli-dev: clap derive definitions per spec §3")
}

pub fn dispatch(command: Command) -> Result<std::process::ExitCode> {
    todo!("cli-dev: map subcommands to init/versions/schema runners")
}
