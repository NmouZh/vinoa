//! Version matrix (spec §6). OWNER: matrix-dev.
pub mod builtin;
pub mod query;
pub mod refresh;

#[cfg(test)]
mod tests;

pub use query::{
    cmp_mc, mc_key, sort_versions, thirdparty_keys_for_features, JavaRequirement, Matrix,
    MatrixData, PlatformData, ThirdPartyLib, ThirdPartyScope, PAPERWEIGHT_FLOOR,
    PLATFORM_ORDER, PROXY_PLATFORMS, REOBF_UNTIL_EXCLUSIVE, SCHEMA,
};
pub use refresh::{FillClient, RefreshReport, API_BASE, USER_AGENT};

use crate::error::Result;
use crate::types::{PlatformPlan, Resolved};

/// Look up a concrete plan for (mc_version, platforms).
pub fn resolve(matrix: &Matrix, mc: &str, platforms: &[String]) -> Result<Resolved> {
    matrix.resolve(mc, platforms)
}

/// `resolve` + the third-party Java-floor check (`init` uses this one).
pub fn resolve_with_features(
    matrix: &Matrix,
    mc: &str,
    platforms: &[String],
    features: &[String],
) -> Result<Resolved> {
    matrix.resolve_with_features(mc, platforms, features)
}

/// Hard error when a requested third-party library needs a newer Java than the
/// module that consumes it provides (`matrix.unsupported_combination`).
pub fn check_thirdparty(
    matrix: &Matrix,
    keys: &[String],
    core_java_target: u8,
    platform_java_target: u8,
) -> Result<()> {
    matrix.check_thirdparty(keys, core_java_target, platform_java_target)
}

/// Normalise a requested platform list: validate, `paper → bukkit`, dedupe,
/// fixed priority order (§7.1).
pub fn normalize_platforms(matrix: &Matrix, requested: &[String]) -> Result<Vec<String>> {
    matrix.normalize_platforms(requested)
}

/// Platforms that exist for a given MC version (for the wizard picker; proxies
/// are always present, §6.3.5). Server-side only: [`Matrix::gated_platforms`].
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
