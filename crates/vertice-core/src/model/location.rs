//! `Location` and the search-root types it references.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// One place a `Component` was found. `path` is optional: a component
/// reported without a backing file (`origin: Embedded`) is still
/// representable and stays distinguishable from a file-backed one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../frontend/src/bindings/")]
pub struct Location {
    pub path: Option<PathBuf>,
    /// Reference into `ScanReport::roots_scanned`, not an embedded
    /// `SearchRoot` — the same root is never duplicated once per location.
    pub root: SearchRootId,
    pub origin: LocationOrigin,
}

/// How a `Location` was discovered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../frontend/src/bindings/")]
pub enum LocationOrigin {
    /// Backed by a file on disk (`Location::path` is `Some`).
    File,
    /// Reported by a client without a backing file (`Location::path` is
    /// `None`).
    Embedded,
}

/// Stable identifier for a `SearchRoot`, used by `Location::root` to avoid
/// embedding (and duplicating) the full `SearchRoot` per location.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../frontend/src/bindings/")]
pub struct SearchRootId(pub String);

/// A directory the scanner walked to produce zero or more components. The
/// scanner is modeled as "one root produces N components", not "one client
/// has N components".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../frontend/src/bindings/")]
pub struct SearchRoot {
    pub id: SearchRootId,
    pub path: PathBuf,
    pub kind: SearchRootKind,
}

/// What kind of component a `SearchRoot` is scanned for. Mirrors
/// `ComponentKind` because clients organize search roots per component kind
/// (e.g. a `skills/` directory and a separate `agents/` directory).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../frontend/src/bindings/")]
pub enum SearchRootKind {
    Skill,
    Agent,
}
