//! Online refresh against `fill.papermc.io/v3` (spec §6.4; sources: [VM §1]).
//! OWNER: matrix-dev.
//!
//! Operational rules encoded here:
//! * every request carries a **non-generic `User-Agent`** identifying the software
//!   and carrying a contact URL ([VM §1.6]);
//! * `?channel=` is only honoured by `/builds` — `/builds/latest` silently ignores
//!   it, so the STABLE choice is made **client-side** ([VM §1.4], spec §6.4);
//! * `api.papermc.io/v2` is dead — this module only ever talks to Fill v3;
//! * a refresh failure is never fatal: the caller keeps the built-in matrix and
//!   warns (spec §11.1 / §12.4).
use std::collections::BTreeMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::query::{cmp_mc, Matrix};
use crate::error::{self, Result};

/// Fill v3 root. `api.papermc.io/v2` is gone ([VM §1.1]).
pub const API_BASE: &str = "https://fill.papermc.io/v3";
/// PaperMC requires a UA that is not a generic client and carries a contact ([VM §1.6]).
pub const USER_AGENT: &str = "vinoa/0.1.0 (+https://github.com/vinoa/vinoa)";
/// Fill projects vinoa consumes. There is **no** `bungeecord` project ([VM §1.1/§4.2]);
/// Waterfall is EOL and out of v1 scope (spec §14).
pub const PROJECTS: [&str; 3] = ["paper", "folia", "velocity"];

const TIMEOUT: Duration = Duration::from_secs(30);

// -------------------------------------------------------------- wire schema

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProjectInfo {
    pub id: String,
    #[serde(default)]
    pub name: String,
}

/// `GET /v3/projects/{project}` — the two-level `group → versions` map ([VM §1.2]).
#[derive(Debug, Clone, Deserialize)]
pub struct ProjectVersions {
    pub project: ProjectInfo,
    pub versions: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Download {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub checksums: BTreeMap<String, String>,
    #[serde(default)]
    pub size: Option<u64>,
}

impl Download {
    pub fn sha256(&self) -> Option<&str> {
        self.checksums.get("sha256").map(String::as_str)
    }
}

/// Build object ([VM §1.2]); `channel` ∈ `ALPHA | BETA | STABLE`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Build {
    pub id: i64,
    #[serde(default)]
    pub time: String,
    pub channel: String,
    #[serde(default)]
    pub downloads: BTreeMap<String, Download>,
}

impl Build {
    /// The server jar (`server:default`); there is no download-redirect endpoint ([VM §1.1]).
    pub fn server_download(&self) -> Option<&Download> {
        self.downloads.get("server:default")
    }

    pub fn sha256(&self) -> Option<&str> {
        self.server_download().and_then(Download::sha256)
    }
}

// ------------------------------------------------------------- report types

#[derive(Debug, Clone, Serialize)]
pub struct BuildRef {
    pub project: String,
    pub mc_version: String,
    pub build_id: i64,
    pub channel: String,
    pub url: String,
    pub sha256: Option<String>,
}

impl BuildRef {
    fn new(project: &str, mc: &str, build: &Build) -> Self {
        let dl = build.server_download();
        Self {
            project: project.to_string(),
            mc_version: mc.to_string(),
            build_id: build.id,
            channel: build.channel.clone(),
            url: dl.map(|d| d.url.clone()).unwrap_or_default(),
            sha256: dl.and_then(|d| d.sha256().map(str::to_string)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct ProjectRefresh {
    /// Release versions only (pre-release ids containing `-` are dropped for display).
    pub versions: Vec<String>,
    /// MC version → newest STABLE build.
    pub stable: BTreeMap<String, BuildRef>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RefreshReport {
    /// Matrix provenance for `--json` (spec §6.4 / §11.5: `builtin` / `cache` / `online`).
    pub source: String,
    pub projects: BTreeMap<String, ProjectRefresh>,
    /// Upstream drift relative to the built-in matrix (R4 early-warning).
    pub changed: Vec<String>,
}

// -------------------------------------------------------------- URL helpers

/// `/builds` accepts `?channel=` ([VM §1.4]).
pub fn builds_url(base: &str, project: &str, mc: &str, channel: Option<&str>) -> String {
    let base = base.trim_end_matches('/');
    match channel {
        Some(ch) => format!("{base}/projects/{project}/versions/{mc}/builds?channel={ch}"),
        None => format!("{base}/projects/{project}/versions/{mc}/builds"),
    }
}

/// `/builds/latest` **ignores** `?channel=` — never pass one ([VM §1.4]).
pub fn latest_url(base: &str, project: &str, mc: &str) -> String {
    format!(
        "{}/projects/{project}/versions/{mc}/builds/latest",
        base.trim_end_matches('/')
    )
}

/// Client-side channel filter over the newest-first build list; the first match wins.
pub fn pick_stable(builds: &[Build]) -> Option<&Build> {
    builds.iter().find(|b| b.channel == "STABLE")
}

/// A Fill version id is a release when it carries no pre-release suffix (`-rc3`, `-pre2`).
pub fn is_release_version(v: &str) -> bool {
    !v.contains('-')
}

// -------------------------------------------------------------------- client

/// Minimal Fill v3 client. Offline by default: nothing here runs unless asked.
#[derive(Debug, Clone)]
pub struct FillClient {
    base: String,
    agent: ureq::Agent,
}

impl Default for FillClient {
    fn default() -> Self {
        Self::new()
    }
}

impl FillClient {
    pub fn new() -> Self {
        Self::with_base(API_BASE)
    }

    pub fn with_base(base: impl Into<String>) -> Self {
        let agent = ureq::Agent::config_builder()
            .user_agent(USER_AGENT)
            .timeout_global(Some(TIMEOUT))
            .build()
            .new_agent();
        Self {
            base: base.into().trim_end_matches('/').to_string(),
            agent,
        }
    }

    pub fn base(&self) -> &str {
        &self.base
    }

    fn get_json<T: serde::de::DeserializeOwned>(&self, url: &str) -> Result<T> {
        let mut response = self.agent.get(url).call().map_err(|e| net_err(url, e))?;
        // Deliberately not `Body::read_json`: that helper is behind ureq's `json`
        // feature, and the matrix crate already depends on `serde_json`.
        let text = response
            .body_mut()
            .read_to_string()
            .map_err(|e| net_err(url, e))?;
        serde_json::from_str(&text)
            .map_err(|e| net_err(url, format!("响应不是合法 JSON: {e}")))
    }

    /// All version ids of a project, flattened from the `group → versions` map.
    pub fn project_versions(&self, project: &str) -> Result<Vec<String>> {
        let url = format!("{}/projects/{project}", self.base);
        let body: ProjectVersions = self.get_json(&url)?;
        let mut out: Vec<String> = body.versions.into_values().flatten().collect();
        out.sort_by(|a, b| cmp_mc(a, b));
        out.dedup();
        Ok(out)
    }

    /// `/builds` (newest first). `channel` is only legal here ([VM §1.4]).
    pub fn builds(&self, project: &str, mc: &str, channel: Option<&str>) -> Result<Vec<Build>> {
        let url = builds_url(&self.base, project, mc, channel);
        self.get_json(&url)
    }

    /// `/builds/latest` — the newest build **regardless of channel** ([VM §1.4]).
    pub fn latest_build(&self, project: &str, mc: &str) -> Result<Build> {
        let url = latest_url(&self.base, project, mc);
        self.get_json(&url)
    }

    /// Newest **STABLE** build, chosen client-side from `/builds`. Never uses
    /// `/builds/latest`, which ignores `?channel=` (spec §6.4).
    pub fn first_stable(&self, project: &str, mc: &str) -> Result<Option<Build>> {
        let builds: Vec<Build> = self.get_json(&builds_url(&self.base, project, mc, None))?;
        Ok(pick_stable(&builds).cloned())
    }
}

fn net_err(url: &str, e: impl std::fmt::Display) -> error::Error {
    error::io(format!("矩阵刷新失败（{url}）: {e}"))
        .with_hint("继续使用内置矩阵，不阻塞生成（spec §6.4）")
}

// ------------------------------------------------------------------- refresh

/// Fetch Fill v3, collect newest STABLE builds and diff against the built-in
/// matrix. The caller writes the result to the user cache directory and falls
/// back to the built-in matrix on any error (spec §6.4 / §12.4).
pub fn refresh(api_base: &str) -> Result<RefreshReport> {
    let client = FillClient::with_base(api_base);
    let builtin = Matrix::builtin().ok();
    let mut projects = BTreeMap::new();
    let mut changed = Vec::new();

    for project in PROJECTS {
        let versions: Vec<String> = client
            .project_versions(project)?
            .into_iter()
            .filter(|v| is_release_version(v))
            .collect();
        let known = builtin
            .as_ref()
            .map(|m| m.platform_versions(project))
            .unwrap_or_default();
        if !known.is_empty() {
            for v in &versions {
                if !known.contains(v) {
                    changed.push(format!("{project}: 上游新增 {v}"));
                }
            }
            for v in &known {
                if !versions.contains(v) {
                    changed.push(format!("{project}: 上游已无 {v}"));
                }
            }
        }
        let mut stable = BTreeMap::new();
        for mc in &versions {
            if let Some(build) = client.first_stable(project, mc)? {
                stable.insert(mc.clone(), BuildRef::new(project, mc, &build));
            }
        }
        projects.insert(
            project.to_string(),
            ProjectRefresh {
                versions,
                stable,
            },
        );
    }

    Ok(RefreshReport {
        source: "online".to_string(),
        projects,
        changed,
    })
}

/// `refresh` against the production Fill endpoint.
pub fn refresh_builtin() -> Result<RefreshReport> {
    refresh(API_BASE)
}
