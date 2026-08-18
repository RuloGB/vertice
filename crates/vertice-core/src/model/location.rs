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
    pub status: SearchRootStatus,
}

/// Whether a scan found this root's path on disk. Two-valued and
/// deliberately no wider: a root that exists but could not be read is
/// `Found` plus a `ScanIssue`, never a third status. `Found`/`NotFound`
/// rather than `Exists`/`Missing` because this is a statement about what one
/// scan observed, not a timeless fact about the path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../frontend/src/bindings/")]
pub enum SearchRootStatus {
    Found,
    NotFound,
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

#[cfg(test)]
mod tests {
    use super::*;

    /// domain-model spec, "An absent root is representable without a
    /// display label": a `SearchRoot` for a non-existent path is
    /// constructible with `status: NotFound`, and the type carries no
    /// client-label field to populate.
    #[test]
    fn absent_search_root_is_constructible_with_not_found_status() {
        let root = SearchRoot {
            id: SearchRootId("claude-skills".to_string()),
            path: PathBuf::from("/does/not/exist/.claude/skills"),
            kind: SearchRootKind::Skill,
            status: SearchRootStatus::NotFound,
        };

        assert_eq!(root.status, SearchRootStatus::NotFound);
    }

    /// domain-model spec, "Absent and present-and-empty are distinguishable
    /// values": two `SearchRoot`s differing only in `status` compare
    /// unequal, and each preserves its own found/not-found state.
    #[test]
    fn search_roots_differing_only_in_status_are_unequal() {
        let base = SearchRoot {
            id: SearchRootId("claude-skills".to_string()),
            path: PathBuf::from("/home/user/.claude/skills"),
            kind: SearchRootKind::Skill,
            status: SearchRootStatus::Found,
        };
        let absent = SearchRoot {
            status: SearchRootStatus::NotFound,
            ..base.clone()
        };

        assert_ne!(base, absent);
        assert_eq!(base.status, SearchRootStatus::Found);
        assert_eq!(absent.status, SearchRootStatus::NotFound);
    }

    /// domain-model spec, "Existing SearchRoot fields are unaffected": a
    /// `SearchRoot` for a root that produced components keeps `id`, `path`,
    /// and `kind` unchanged in type and value.
    #[test]
    fn existing_fields_are_unchanged_in_type_and_value() {
        let id = SearchRootId("agents-skills".to_string());
        let path = PathBuf::from("/home/user/.agents/skills");
        let kind = SearchRootKind::Agent;

        let root = SearchRoot {
            id: id.clone(),
            path: path.clone(),
            kind,
            status: SearchRootStatus::Found,
        };

        assert_eq!(root.id, id);
        assert_eq!(root.path, path);
        assert_eq!(root.kind, kind);
    }
}
