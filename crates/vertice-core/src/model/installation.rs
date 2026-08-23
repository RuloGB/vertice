//! `ClientInstallation` and the closed set of clients the PoC recognizes.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// One detected installation of a supported client. Each installation is
/// counted separately: a client installed twice (e.g. two Claude Code
/// installs) is reported as two `ClientInstallation` values, never merged.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../frontend/src/bindings/")]
pub struct ClientInstallation {
    pub client: ClientKind,
    pub version: String,
    pub path: PathBuf,
}

/// Closed set of clients the PoC scans. Deliberately minimal: only the
/// clients the PoC ships adapters for (Claude Code, OpenCode, Codex).
/// Growing this set is expected as later adapter phases land, but it stays a
/// closed enum, never `#[non_exhaustive]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../frontend/src/bindings/")]
pub enum ClientKind {
    ClaudeCode,
    OpenCode,
    Codex,
}
