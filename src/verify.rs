//! `--verify`: run the generated project's build. OWNER: cli-dev.
use crate::error::Result;
use crate::types::Plan;

pub struct VerifyOutcome {
    pub ok: bool,
    pub log_path: String,
    pub reason: Option<String>,
}

pub fn run(_plan: &Plan, _online: bool) -> Result<VerifyOutcome> {
    todo!("cli-dev: gradlew build (gradlew.bat on Windows), 600s default / 1800s cap, offline by default")
}
