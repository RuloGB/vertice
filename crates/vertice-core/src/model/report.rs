//! `ScanReport`, `ScanIssue`, and `IssueSeverity` — the top-level scan
//! result and its non-aborting issue-accumulation contract.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::component::Component;
use super::installation::ClientInstallation;
use super::location::SearchRoot;

/// The complete result of one scan. Empty collections are a legitimate,
/// non-error value — `Err` is reserved for orchestration-level failure
/// where the scan could not run at all, never for "the scan ran and found
/// nothing".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../frontend/src/bindings/")]
pub struct ScanReport {
    pub components: Vec<Component>,
    pub installations: Vec<ClientInstallation>,
    pub roots_scanned: Vec<SearchRoot>,
    pub issues: Vec<ScanIssue>,
    /// Value passed in by the caller, never measured here — this module
    /// performs zero clock reads.
    pub duration_ms: u32,
}

/// One recoverable, per-item problem accumulated during a scan. Neither
/// severity level aborts the scan; every issue, of either severity, ends up
/// in `ScanReport::issues`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../frontend/src/bindings/")]
pub struct ScanIssue {
    pub severity: IssueSeverity,
    /// `None` when the issue has no meaningful file path (an absent root,
    /// or an embedded component).
    pub path: Option<PathBuf>,
    pub reason: String,
}

/// Two non-aborting severity levels. Severity is a display/triage signal
/// for later UI phases, not control flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../frontend/src/bindings/")]
pub enum IssueSeverity {
    Warning,
    Error,
}
