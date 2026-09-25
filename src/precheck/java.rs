//! JAVA_HOME -> PATH -> common install dirs. OWNER: cli-dev.
use crate::types::Resolved;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JavaCheck {
    pub required: u8,
    pub platform: String,
    pub found_path: Option<String>,
}

impl JavaCheck {
    pub fn satisfied(&self) -> bool {
        self.found_path.is_some()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct JavaReport {
    pub checks: Vec<JavaCheck>,
    pub gradle_version: Option<String>,
}

impl JavaReport {
    pub fn missing(&self) -> Vec<&JavaCheck> {
        self.checks.iter().filter(|c| !c.satisfied()).collect()
    }
    pub fn all_satisfied(&self) -> bool {
        self.missing().is_empty()
    }
}

pub fn detect(_resolved: &Resolved) -> JavaReport {
    todo!("cli-dev")
}
