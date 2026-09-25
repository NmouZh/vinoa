//! Atomic write + git init (spec §5 phases 6-7). OWNER: cli-dev.
pub mod git;

use crate::error::Result;
use crate::types::Plan;

/// Write the whole plan atomically: temp dir next to target, then rename.
pub fn write_atomic(_plan: &Plan) -> Result<()> {
    todo!("cli-dev: tempdir -> rename; rollback on failure; refuse non-empty target")
}

/// Refuse to write when the target directory exists and is not empty.
pub fn ensure_target_writable(_dir: &std::path::Path) -> Result<()> {
    todo!("cli-dev")
}
