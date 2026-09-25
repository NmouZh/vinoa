//! Matrix type + queries. OWNER: matrix-dev.
use crate::error::Result;
use crate::types::{PlatformPlan, Resolved};

#[derive(Debug, Clone)]
pub struct Matrix {
    pub generated_at: String,
}

impl Matrix {
    pub fn builtin() -> Result<Self> {
        todo!("matrix-dev: parse data/version-matrix.toml")
    }
    pub fn load_from_str(_toml: &str) -> Result<Self> {
        todo!("matrix-dev")
    }
    pub fn resolve(&self, _mc: &str, _platforms: &[String]) -> Result<Resolved> {
        todo!("matrix-dev")
    }
    pub fn available_platforms(&self, _mc: &str) -> Vec<String> {
        todo!("matrix-dev")
    }
    pub fn supported_versions(&self) -> Vec<String> {
        todo!("matrix-dev")
    }
    pub fn platform_plan(&self, _mc: &str, _platform: &str) -> Option<PlatformPlan> {
        todo!("matrix-dev")
    }
}
