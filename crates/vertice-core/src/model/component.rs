//! `Component` and its closed enums (`ComponentKind`, `Scope`).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::identity::ComponentId;
use super::location::Location;

/// One logical AI component (a skill, an agent, or an MCP server),
/// aggregated across every location it was found in. Discovering the same
/// component under N search roots yields ONE `Component` with N `Location`
/// entries sharing one `id`, never N separate components.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../frontend/src/bindings/")]
pub struct Component {
    pub id: ComponentId,
    /// Raw display name, un-normalized. Identity is derived from a
    /// normalized copy (`identity::normalize_name`); this field preserves
    /// the original as the user or client would see it.
    pub name: String,
    pub kind: ComponentKind,
    pub description: Option<String>,
    pub scope: Scope,
    pub locations: Vec<Location>,
    /// Opaque display string describing provenance, absent when the adapter
    /// has nothing to report. MUST NOT be branched on to drive behavior — any
    /// machine-readable classification of a location's origin lives on
    /// `Location::origin` instead. `Option` rather than an empty `String`:
    /// "no hint" is a real state, not a sentinel value.
    pub provenance_hint: Option<String>,
}

/// Closed set of component kinds the PoC recognizes. No `#[non_exhaustive]`:
/// adding a variant is a breaking change that must be reviewed everywhere
/// this enum is matched.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../frontend/src/bindings/")]
pub enum ComponentKind {
    Skill,
    Agent,
    Mcp,
}

/// Closed set of scopes a component can be discovered at. The PoC only ever
/// constructs `Scope::User`; `Project` and `Local` are modeled now so the
/// field never needs a breaking shape change later.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../frontend/src/bindings/")]
pub enum Scope {
    User,
    Project,
    Local,
}
