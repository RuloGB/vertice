//! `ScanError` — orchestration-level scan failure.
//!
//! Reserved for failures where the scan cannot proceed at all. Per-item,
//! recoverable problems (unparseable frontmatter, an unreadable file, an
//! absent root, a non-UTF-8 path) are `ScanIssue`s accumulated into
//! `Ok(ScanReport)`, never `ScanError`. See `design.md` §6-§7 for the full
//! taxonomy.
//!
//! **Invariant**: every variant payload here MUST be owned, serializable
//! data (`String`, `PathBuf`, a model enum) — never a foreign error type.
//! Foreign errors (e.g. the YAML crate's error type, `std::io::Error`) are
//! not `Serialize`; they are converted with `.to_string()` at the boundary
//! before reaching a `ScanError` variant, not wrapped with `#[from]`.
//! `Deserialize` is deliberately not derived: the frontend never constructs
//! a `ScanError`, only receives one.

use serde::Serialize;
use thiserror::Error;
use ts_rs::TS;

/// Orchestration-level failure: the scan could not proceed at all.
#[derive(Debug, Error, Serialize, TS)]
#[serde(tag = "kind", content = "detail", rename_all = "camelCase")]
#[ts(export, export_to = "../../../frontend/src/bindings/")]
pub enum ScanError {
    /// No search roots were configured to scan.
    #[error("no search roots configured")]
    NoRootsConfigured,
    /// The scan could not proceed because of an unexpected internal
    /// failure. `reason` is already-owned, human-readable text — never a
    /// foreign error type.
    #[error("scan failed: {reason}")]
    Internal { reason: String },
}
