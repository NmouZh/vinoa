//! Template engine: manifest, rendering, plan, assertions (spec §7).
//! OWNER: engine-dev.
pub mod manifest;
pub mod plan;
pub mod render;
pub mod vars;

pub use manifest::{TemplateManifest, load_builtin, load_from, load_from_str, load_from_str_with_source};
pub use plan::{build_plan, build_plan_with_vars};

#[cfg(test)]
mod tests;

use crate::error::Result;
use crate::types::{Plan, ProjectSpec, Resolved};

/// Build the full plan (the only source of truth for both `--dry-run` and
/// execution).
pub fn plan(manifest: &TemplateManifest, spec: &ProjectSpec, resolved: &Resolved) -> Result<Plan> {
    plan::build_plan(manifest, spec, resolved)
}

pub fn load_manifest() -> Result<TemplateManifest> {
    manifest::load_builtin()
}

pub fn load_manifest_from(path: &std::path::Path) -> Result<TemplateManifest> {
    manifest::load_from(path)
}

/// sha256 hex — used for plan metadata and reproducibility checks.
pub fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher.finalize().iter().map(|b| format!("{b:02x}")).collect()
}
