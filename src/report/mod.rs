//! Human and JSON reporting (spec §11). OWNER: cli-dev.
use crate::error::Error;
use crate::types::Plan;

pub fn error(_err: &Error) {
    todo!("cli-dev: human-readable error with stable code + hint")
}

pub fn plan_human(_plan: &Plan) {
    todo!("cli-dev")
}

pub fn plan_json(_plan: &Plan, _extra: &serde_json::Value) {
    todo!("cli-dev: exactly one JSON document on stdout; human output on stderr")
}
