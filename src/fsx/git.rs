//! git init + first commit. OWNER: cli-dev.
use crate::error::Result;
use std::path::Path;

pub fn init_and_commit(_dir: &Path, _branch: &str, _message: &str) -> Result<()> {
    todo!("cli-dev: git init -b <branch>; git add -A; git commit")
}
