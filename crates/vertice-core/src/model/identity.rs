//! Component identity derivation — deterministic, content-free.
//!
//! See `design.md` §3 for the full decision record. `ComponentId` is a
//! newtype over a human-readable string, never a hash: `"{kind}:{normalized
//! name}"`. Identity depends on `(kind, name)` alone — never on `Location`
//! data or file content.

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use unicode_normalization::UnicodeNormalization;

use super::component::ComponentKind;

/// Deterministic, human-readable identity for a `Component`: `"{kind
/// prefix}:{normalized name}"`. Stable across runs, processes, and
/// platforms — no RNG, no clock, no hashing.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../frontend/src/bindings/")]
pub struct ComponentId(String);

impl ComponentId {
    /// Derive an id from `(kind, name)` alone. Never incorporates
    /// `Location` data or file content — content divergence across
    /// locations is surfaced downstream as a duplicate to review, not as
    /// two distinct identities.
    pub fn derive(kind: ComponentKind, name: &str) -> Self {
        let normalized = normalize_name(name);
        Self(format!("{}:{normalized}", kind.identity_prefix()))
    }

    /// Borrow the underlying id string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl ComponentKind {
    /// Kind segment used as the identity prefix. `ComponentKind`'s
    /// serialized forms contain no `:`, so a name containing `:` can never
    /// alias another kind's id.
    fn identity_prefix(self) -> &'static str {
        match self {
            ComponentKind::Skill => "skill",
            ComponentKind::Agent => "agent",
        }
    }
}

/// Normalize a raw component name for identity purposes: trim Unicode
/// whitespace, apply NFC normalization (macOS surfaces NFD, Linux/Windows
/// surface NFC — without this step the same name would derive two ids
/// depending on source platform), then lowercase with full Unicode case
/// conversion (not case folding — `ß`/`ẞ` do not unify; PoC component names
/// are kebab-case ASCII, so this is an accepted limitation).
pub(crate) fn normalize_name(name: &str) -> String {
    name.trim().nfc().collect::<String>().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::super::component::ComponentKind;
    use super::ComponentId;

    #[test]
    fn case_variants_collapse_to_one_identity() {
        let upper = ComponentId::derive(ComponentKind::Skill, "Issue-Creation");
        let lower = ComponentId::derive(ComponentKind::Skill, "issue-creation");

        assert_eq!(upper, lower);
    }

    #[test]
    fn same_kind_and_name_always_yield_equal_ids() {
        let first = ComponentId::derive(ComponentKind::Agent, "triage");
        let second = ComponentId::derive(ComponentKind::Agent, "triage");

        assert_eq!(first, second);
    }

    #[test]
    fn different_kind_same_name_yields_different_identity() {
        let skill = ComponentId::derive(ComponentKind::Skill, "triage");
        let agent = ComponentId::derive(ComponentKind::Agent, "triage");

        assert_ne!(skill, agent);
    }

    #[test]
    fn delimiter_in_name_cannot_forge_a_different_kind_prefix() {
        // A name containing `:` must not let its suffix be mistaken for a
        // different kind's id: the prefix before the FIRST `:` is always
        // exactly the kind, because `ComponentKind`'s serialized forms
        // (`skill`, `agent`) contain no `:` themselves.
        let id = ComponentId::derive(ComponentKind::Skill, "agent:fake-name");

        assert_eq!(id.as_str(), "skill:agent:fake-name");
        assert_eq!(id.as_str().split_once(':').unwrap().0, "skill");
    }

    #[test]
    fn nfc_and_nfd_encodings_of_the_same_name_collapse_to_one_identity() {
        // "é" as a single NFC codepoint (U+00E9) vs. "e" + combining acute
        // accent (U+0065 U+0301, NFD) — macOS surfaces NFD, Linux/Windows
        // surface NFC. Both must derive the same id or a synced/mac-sourced
        // root would silently double a component (the exact duplication
        // acceptance criterion 2 forbids).
        let nfc_name = "revisi\u{00e9}n";
        let nfd_name = "revisi\u{0065}\u{0301}n";

        let nfc_id = ComponentId::derive(ComponentKind::Skill, nfc_name);
        let nfd_id = ComponentId::derive(ComponentKind::Skill, nfd_name);

        assert_eq!(nfc_id, nfd_id);
    }
}
