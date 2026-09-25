//! IDEA-style interactive wizard (spec §4). OWNER: cli-dev.
use crate::error::Result;
use crate::types::{ProjectSpec, Resolved};

/// Only called when stdin/stdout are a TTY and `--yes` was not given.
pub fn run(_partial: crate::cli::InitArgs, _resolved: Option<&Resolved>) -> Result<ProjectSpec> {
    todo!("cli-dev: inquire pages 1-4 with back navigation")
}
