//! Post-scan duplicate consolidation: merge components discovered under
//! different search roots into one entry per identity.
//!
//! Pure and total: no filesystem read, no environment read, no clock read,
//! and no panicking path. See `design.md` (duplicate-consolidation) for the
//! full decision record — the canonical root order (`ROOT_ORDER`), the
//! sort-then-fold grouping strategy, and the first-non-empty field
//! precedence rule are all pinned there.

use std::path::Path;

use crate::model::{Component, Location};

/// Canonical search-root order, pinned to `roots::skill_roots` ++
/// `roots::agent_roots` ++ `roots::opencode_agent_root` ++
/// `roots::codex_agent_root` by a test below (design §6.2) rather than by a
/// call — those functions require a `home` and touch the filesystem, which
/// this module must never do. `codex-skills` lands at index 3 (inside
/// `skill_roots`), not last overall — see
/// `openspec/changes/2026-08-23-add-codex-client-support/design.md` §0/§6.2.
const ROOT_ORDER: [&str; 8] = [
    "claude-skills",
    "agents-skills",
    "opencode-skills",
    "codex-skills",
    "claude-agents",
    "claude-embedded-agents",
    "opencode-agents",
    "codex-agents",
];

/// Rank of a root id in `ROOT_ORDER`; an unknown id (a future root not yet
/// added here) falls to the end, deterministically and without panicking
/// (design §4).
fn root_rank(root_id: &str) -> usize {
    ROOT_ORDER
        .iter()
        .position(|&id| id == root_id)
        .unwrap_or(ROOT_ORDER.len())
}

/// Total, deterministic sort key for a `Location` (design §4): rank first,
/// then root id (several ids can share the fallback rank), then path.
/// `Option<&Path>` orders `None` before `Some`, so an embedded
/// pseudo-location always sorts ahead of its file-backed siblings under the
/// same root — and two locations legitimately sharing one root id (the
/// OpenCode plural/singular alias) still sort deterministically by path.
fn location_key(location: &Location) -> (usize, &str, Option<&Path>) {
    (
        root_rank(&location.root.0),
        location.root.0.as_str(),
        location.path.as_deref(),
    )
}

/// Member sort key within an identity group (design §5): the member's own
/// sorted location keys, then its raw `name`. `Vec<T: Ord>` is `Ord`, so
/// this is total; grouping order falls out of this same sort, so the
/// precedence walk needs no second ordering pass and never consults the
/// input's arrival index.
fn member_key(component: &Component) -> (Vec<(usize, String, Option<std::path::PathBuf>)>, String) {
    let mut keys: Vec<_> = component
        .locations
        .iter()
        .map(|loc| {
            let (rank, root_id, path) = location_key(loc);
            (rank, root_id.to_string(), path.map(Path::to_path_buf))
        })
        .collect();
    keys.sort();
    (keys, component.name.clone())
}

/// `true` when a display string carries no visible content — whitespace
/// only counts as empty, since the frontmatter seam performs no trimming
/// and a folded block scalar keeps its trailing newline (design §6, V8).
fn is_blank_str(value: &str) -> bool {
    value.trim().is_empty()
}

/// Same rule as [`is_blank_str`] for an optional field: absent counts as
/// blank too.
fn is_blank_opt(value: &Option<String>) -> bool {
    value.as_deref().map(is_blank_str).unwrap_or(true)
}

/// Fold `other` into `target`, both already belonging to the same identity
/// group. `target` is always the accumulator built from members walked in
/// canonical-order-of-arrival (member-key order), so "first present and
/// non-empty wins" only needs a one-way check per field (design §6).
fn merge_into(target: &mut Component, mut other: Component) {
    if is_blank_str(&target.name) && !is_blank_str(&other.name) {
        target.name = std::mem::take(&mut other.name);
    }
    if is_blank_opt(&target.description) && !is_blank_opt(&other.description) {
        target.description = other.description.take();
    }
    if is_blank_opt(&target.provenance_hint) && !is_blank_opt(&other.provenance_hint) {
        target.provenance_hint = other.provenance_hint.take();
    }
    // `scope` is not optional and not a string: precedence degenerates to
    // "first member wins", so `target.scope` (already the first member's,
    // or an earlier merge's untouched value) is left alone (design §6).
    target.locations.extend(other.locations);
}

/// Merge components discovered under different search roots into one entry
/// per identity. Pure: no I/O, no clock, no ambient environment. Total: it
/// cannot fail and emits no `ScanIssue` (design §8).
#[must_use]
pub fn consolidate(mut components: Vec<Component>) -> Vec<Component> {
    components.sort_by(|a, b| {
        a.id.as_str()
            .cmp(b.id.as_str())
            .then_with(|| member_key(a).cmp(&member_key(b)))
    });

    let mut out: Vec<Component> = Vec::new();
    for component in components {
        match out.last_mut() {
            Some(last) if last.id == component.id => merge_into(last, component),
            _ => out.push(component),
        }
    }

    for component in &mut out {
        component
            .locations
            .sort_by(|a, b| location_key(a).cmp(&location_key(b)));
    }

    out.sort_by(|a, b| {
        a.name
            .cmp(&b.name)
            .then_with(|| a.id.as_str().cmp(b.id.as_str()))
    });

    out
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::model::{
        Component, ComponentId, ComponentKind, Location, LocationOrigin, Scope, SearchRootId,
    };

    fn location(root_id: &str, path: Option<&str>) -> Location {
        Location {
            path: path.map(PathBuf::from),
            root: SearchRootId(root_id.to_string()),
            origin: if path.is_some() {
                LocationOrigin::File
            } else {
                LocationOrigin::Embedded
            },
        }
    }

    fn skill(name: &str, description: Option<&str>, locations: Vec<Location>) -> Component {
        Component {
            id: ComponentId::derive(ComponentKind::Skill, name),
            name: name.to_string(),
            kind: ComponentKind::Skill,
            description: description.map(str::to_string),
            scope: Scope::User,
            locations,
            provenance_hint: None,
        }
    }

    fn agent(name: &str, locations: Vec<Location>) -> Component {
        Component {
            id: ComponentId::derive(ComponentKind::Agent, name),
            name: name.to_string(),
            kind: ComponentKind::Agent,
            description: None,
            scope: Scope::User,
            locations,
            provenance_hint: None,
        }
    }

    /// Design §4: the local `ROOT_ORDER` constant is pinned to `roots.rs` by
    /// this test, never by a call — `skill_roots`/`agent_roots`/
    /// `opencode_agent_root` require a `home` and touch the filesystem.
    #[test]
    fn root_order_matches_the_roots_module_in_order() {
        let home = PathBuf::from("/definitely/does/not/exist/vertice-consolidate-pin");

        let mut expected: Vec<String> = Vec::new();
        for resolved in crate::roots::skill_roots(&home) {
            expected.push(resolved.root.id.0.clone());
        }
        for resolved in crate::roots::agent_roots(&home) {
            expected.push(resolved.root.id.0.clone());
        }
        expected.push(crate::roots::opencode_agent_root(&home).root.id.0.clone());
        expected.push(crate::roots::codex_agent_root(&home).root.id.0.clone());

        let actual: Vec<String> = ROOT_ORDER.iter().map(|s| s.to_string()).collect();

        assert_eq!(actual, expected);
    }

    #[test]
    fn unknown_root_id_ranks_last_deterministically() {
        let known_location = location("claude-skills", Some("/a"));
        let unknown_location = location("unknown-root", Some("/b"));
        let known = location_key(&known_location);
        let unknown = location_key(&unknown_location);

        assert!(known.0 < unknown.0);
        assert_eq!(unknown.0, ROOT_ORDER.len());
    }

    #[test]
    fn none_path_sorts_before_some_path_under_the_same_root() {
        let embedded_location = location("claude-embedded-agents", None);
        let file_location = location("claude-embedded-agents", Some("/a"));
        let embedded = location_key(&embedded_location);
        let file = location_key(&file_location);

        assert!(embedded < file);
    }

    #[test]
    fn later_roots_non_empty_description_wins_over_earlier_roots_empty_one() {
        let earlier = skill(
            "triage",
            None,
            vec![location("claude-skills", Some("/a/SKILL.md"))],
        );
        let later = skill(
            "triage",
            Some("Real description."),
            vec![location("agents-skills", Some("/b/SKILL.md"))],
        );

        let result = consolidate(vec![earlier, later]);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].description, Some("Real description.".to_string()));
    }

    #[test]
    fn whitespace_only_description_does_not_win_precedence() {
        let earlier = skill(
            "triage",
            Some("\n"),
            vec![location("claude-skills", Some("/a/SKILL.md"))],
        );
        let later = skill(
            "triage",
            Some("Real description."),
            vec![location("agents-skills", Some("/b/SKILL.md"))],
        );

        let result = consolidate(vec![earlier, later]);

        assert_eq!(result[0].description, Some("Real description.".to_string()));
    }

    #[test]
    fn all_blank_descriptions_preserve_the_first_members_value_verbatim() {
        let earlier = skill(
            "triage",
            Some(""),
            vec![location("claude-skills", Some("/a/SKILL.md"))],
        );
        let later = skill(
            "triage",
            None,
            vec![location("agents-skills", Some("/b/SKILL.md"))],
        );

        let result = consolidate(vec![earlier, later]);

        assert_eq!(result[0].description, Some(String::new()));
    }

    #[test]
    fn precedence_is_independent_of_input_arrival_order() {
        let a = skill(
            "triage",
            None,
            vec![location("claude-skills", Some("/a/SKILL.md"))],
        );
        let b = skill(
            "triage",
            Some("Real description."),
            vec![location("agents-skills", Some("/b/SKILL.md"))],
        );
        let c = skill(
            "triage",
            Some("Another description."),
            vec![location("opencode-skills", Some("/c/SKILL.md"))],
        );

        let forward = consolidate(vec![a.clone(), b.clone(), c.clone()]);
        let shuffled = consolidate(vec![c, a, b]);

        assert_eq!(forward, shuffled);
    }

    #[test]
    fn a_skill_and_an_agent_sharing_a_name_are_not_merged() {
        let skill_c = skill(
            "triage",
            None,
            vec![location("claude-skills", Some("/a/SKILL.md"))],
        );
        let agent_c = agent(
            "triage",
            vec![location("claude-agents", Some("/b/AGENT.md"))],
        );

        let result = consolidate(vec![skill_c, agent_c]);

        assert_eq!(result.len(), 2);
    }

    #[test]
    fn case_and_nfc_nfd_name_variants_collapse_to_one_component_with_two_locations() {
        let upper = skill(
            "Issue-Creation",
            None,
            vec![location("claude-skills", Some("/a/SKILL.md"))],
        );
        let lower = skill(
            "issue-creation",
            None,
            vec![location("agents-skills", Some("/b/SKILL.md"))],
        );

        let result = consolidate(vec![upper, lower]);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].locations.len(), 2);
    }

    #[test]
    fn empty_input_yields_empty_output() {
        assert!(consolidate(Vec::new()).is_empty());
    }

    #[test]
    fn single_component_input_is_passed_through_with_one_location() {
        let only = skill(
            "solo",
            Some("desc"),
            vec![location("claude-skills", Some("/a/SKILL.md"))],
        );

        let result = consolidate(vec![only]);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].locations.len(), 1);
    }

    #[test]
    fn two_components_sharing_a_display_name_are_ordered_by_identity() {
        let agent_c = agent(
            "triage",
            vec![location("claude-agents", Some("/a/AGENT.md"))],
        );
        let skill_c = skill(
            "triage",
            None,
            vec![location("claude-skills", Some("/b/SKILL.md"))],
        );

        let result = consolidate(vec![agent_c.clone(), skill_c.clone()]);

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].id, agent_c.id);
        assert_eq!(result[1].id, skill_c.id);
    }
}
