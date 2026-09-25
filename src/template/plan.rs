//! Plan construction: render tree -> in-memory file set. OWNER: engine-dev.
use crate::error::Result;
use crate::types::{Plan, ProjectSpec, Resolved};
use super::manifest::TemplateManifest;

pub fn build_plan(_m: &TemplateManifest, _spec: &ProjectSpec, _resolved: &Resolved) -> Result<Plan> {
    todo!("engine-dev: render + rename rules + conditional files + assertions A1-A5")
}
