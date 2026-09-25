//! Online refresh against fill.papermc.io/v3. OWNER: matrix-dev.
use crate::error::Result;

#[derive(Debug, Clone)]
pub struct RefreshReport {
    pub source: String,
    pub changed: Vec<String>,
}

pub fn refresh(_api_base: &str) -> Result<RefreshReport> {
    todo!("matrix-dev: GET fill v3 projects/versions; non-generic User-Agent; channel filter client-side")
}
