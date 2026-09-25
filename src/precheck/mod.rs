//! Environment precheck — JDK detection (spec §10). OWNER: cli-dev.
pub mod java;
pub use java::{JavaCheck, JavaReport};

use crate::types::Resolved;

pub fn check_java(_resolved: &Resolved) -> JavaReport {
    java::detect(_resolved)
}
