//! Environment precheck — JDK detection (spec §10). OWNER: cli-dev.
pub mod java;
pub use java::{JavaCheck, JavaHome, JavaReport, FOOJAY_PLUGIN};

use crate::types::Resolved;

pub fn check_java(resolved: &Resolved) -> JavaReport {
    java::detect(resolved)
}
