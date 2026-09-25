//! Condition evaluation + minijinja rendering. OWNER: engine-dev.
use crate::error::Result;

/// Restricted condition language: pure variables plus `cap_*` / `has_*` / `is_*` switches.
pub fn eval_condition(_expr: &str, _ctx: &serde_json::Value) -> Result<bool> {
    todo!("engine-dev")
}

pub fn render_str(_template: &str, _ctx: &serde_json::Value) -> Result<String> {
    todo!("engine-dev")
}
