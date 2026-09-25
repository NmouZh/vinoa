//! vinoa-template.toml schema + loader. OWNER: engine-dev.
use crate::error::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TemplateManifest {
    pub schema: u32,
    pub min_cli_version: String,
    pub name: String,
    #[serde(default)]
    pub vars: Vec<String>,
    #[serde(default)]
    pub entries: Vec<Entry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub path: String,
    #[serde(default)]
    pub render: Render,
    #[serde(default)]
    pub when: Option<String>,
    #[serde(default)]
    pub foreach: Option<String>,
    #[serde(default)]
    pub mode: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Render {
    #[default]
    Template,
    Copy,
}

pub fn load_builtin() -> Result<TemplateManifest> {
    load_from_str(include_str!("../../templates/vinoa-template.toml"))
}

pub fn load_from_str(_toml: &str) -> Result<TemplateManifest> {
    todo!("engine-dev")
}

pub fn load_from(_path: &std::path::Path) -> Result<TemplateManifest> {
    todo!("engine-dev")
}
