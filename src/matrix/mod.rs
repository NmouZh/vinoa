//! Version matrix (spec §6). OWNER: matrix-dev.
pub mod builtin;
pub mod query;
pub mod refresh;

pub use query::Matrix;

use crate::error::Result;
use crate::types::{PlatformPlan, Resolved};

/// Look up a concrete plan for (mc_version, platforms).
pub fn resolve(matrix: &Matrix, mc: &str, platforms: &[String]) -> Result<Resolved> {
    matrix.resolve(mc, platforms)
}

/// Platforms that exist for a given MC version (for the wizard picker).
pub fn available_platforms(matrix: &Matrix, mc: &str) -> Vec<String> {
    matrix.available_platforms(mc)
}

/// MC versions the CLI supports, in display order.
pub fn supported_versions(matrix: &Matrix) -> Vec<String> {
    matrix.supported_versions()
}

pub fn platform_plan(matrix: &Matrix, mc: &str, platform: &str) -> Option<PlatformPlan> {
    matrix.platform_plan(mc, platform)
}
