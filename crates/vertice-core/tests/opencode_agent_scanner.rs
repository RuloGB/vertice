//! Fixture-driven behaviour tests for `vertice_core::opencode_agents::scan`,
//! over the synthetic-home fixture tree committed under
//! `crates/vertice-core/tests/fixtures/roots/opencode-agents/`. One test (or
//! tight group) per `opencode-agent-scanner` spec requirement;
//! `openspec/changes/opencode-agent-adapter/design.md` is the authority for
//! every asserted `status`/`severity`/`reason` shape.
//!
//! `design.md` §10's `.gitkeep` tripwire is deliberately split in two: the
//! disk-existence half below needs no `opencode_agents` module and lands
//! with the fixture tree in commit group 1 (Phase 1); the `status ==
//! NotFound` half lives in `roots.rs`'s own unit tests, next to
//! `opencode_agent_root`.

use std::path::PathBuf;

use vertice_core::model::{ComponentId, ComponentKind, IssueSeverity, LocationOrigin, Scope};
use vertice_core::opencode_agents;
use vertice_core::roots;

/// Build a path under
/// `crates/vertice-core/tests/fixtures/roots/opencode-agents/<case>/` from
/// per-segment pushes — never a `"/"`-joined literal, so it stays
/// separator-correct on Windows (`tests/skill_scanner.rs:23-30`'s pattern).
fn fixture_home(case: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("fixtures");
    path.push("roots");
    path.push("opencode-agents");
    path.push(case);
    path
}

/// Tripwire (design §10, T4D §8/T5D §10 precedent): git cannot track an
/// empty directory, so `empty-config-dir/`'s "config dir present, neither
/// file present" case relies on a `.gitkeep` file. If that file is ever
/// lost, the directory silently vanishes and this fixture starts
/// exercising the "absent config directory entirely" path instead, still
/// passing with zero components and zero issues. This half asserts the
/// directory itself is present on disk, independent of any scanner code.
#[test]
fn empty_config_dir_fixture_still_exists_on_disk() {
    let mut path = fixture_home("empty-config-dir");
    path.push(".config");
    path.push("opencode");

    let metadata =
        std::fs::metadata(&path).expect("empty-config-dir fixture directory must exist on disk");

    assert!(
        metadata.is_dir(),
        "empty-config-dir fixture path must be a directory, not a file"
    );
}

/// Requirement: A key present in both files with a partial override yields
/// one component whose non-overridden field survives (design's primary
/// safeguard against a whole-object-replacement merge bug, tasks 2.1-2.3).
/// Written *first* among the integration tests, before `opencode_agents`
/// exists to compile against — this file's compile failure is expected and
/// acceptable specifically because 2.1's in-memory literal test is the one
/// that must fail on an assertion, not a compile error; the two together
/// are the safeguard.
#[test]
fn partial_override_fixture_merges_per_field_not_per_object() {
    let home = fixture_home("partial-override");

    let scan = opencode_agents::scan(&home);

    assert_eq!(
        scan.components.len(),
        1,
        "exactly one component for the shared key"
    );
    let component = &scan.components[0];
    assert_eq!(component.name, "reviewer");
    assert_eq!(
        component.description.as_deref(),
        Some("Reviews code for quality issues"),
        "the base's non-overridden `description` must survive the merge"
    );
}

/// Requirement: A key present in both files with a conflicting field takes
/// `opencode.jsonc`'s value — the fully-overriding sibling of
/// `partial-override/`'s partial case, on the same fixture.
#[test]
fn partial_override_fixture_fully_conflicting_field_takes_overlay_value() {
    let home = fixture_home("partial-override");

    let scan = opencode_agents::scan(&home);

    let component = &scan.components[0];
    // `permission.edit` is set by both files; the overlay's "allow" wins.
    // We only observe `description` on the `Component` (design §5.4 — no
    // other field is surfaced), so the conflicting-field resolution is
    // pinned at the merge-literal level (`opencode_agents.rs`'s own unit
    // tests) and re-asserted here through the two `Location`s this key
    // carries, proving both files declared it.
    assert_eq!(component.locations.len(), 2);
}

/// Requirement: One File Produces N Components — `json-only/` yields a
/// component per key, `origin: File`.
#[test]
fn json_only_fixture_yields_a_component_per_key() {
    let home = fixture_home("json-only");

    let scan = opencode_agents::scan(&home);

    assert_eq!(scan.components.len(), 2);
    assert_eq!(scan.issues.len(), 0);
    for component in &scan.components {
        assert_eq!(component.locations.len(), 1);
        assert_eq!(component.locations[0].origin, LocationOrigin::File);
    }
    let mut names: Vec<&str> = scan.components.iter().map(|c| c.name.as_str()).collect();
    names.sort();
    assert_eq!(names, vec!["json-only-one", "json-only-two"]);
}

/// Requirement: A key present in only `opencode.jsonc` survives (CA-5) —
/// exercised in isolation, with no `opencode.json` present at all. This is
/// design §0/V4's non-negotiable coverage: the real reference machine's
/// `opencode.jsonc` has no `agent` key, so only this fixture proves the
/// `.jsonc`-agent path works at all.
#[test]
fn jsonc_only_fixture_yields_agents_with_no_json_present() {
    let home = fixture_home("jsonc-only");

    let scan = opencode_agents::scan(&home);

    assert_eq!(scan.components.len(), 2);
    assert_eq!(scan.issues.len(), 0);
    let mut names: Vec<&str> = scan.components.iter().map(|c| c.name.as_str()).collect();
    names.sort();
    assert_eq!(names, vec!["jsonc-only-one", "jsonc-only-two"]);
}

/// Requirement: A JSONC file with comments and a trailing comma parses
/// successfully — design §0/V4's non-negotiable coverage, unreachable from
/// the real reference machine's `opencode.jsonc`.
#[test]
fn jsonc_syntax_fixture_parses_comments_and_trailing_comma() {
    let home = fixture_home("jsonc-syntax");

    let scan = opencode_agents::scan(&home);

    assert_eq!(
        scan.issues.len(),
        0,
        "comments and a trailing comma must not be treated as malformed"
    );
    let mut names: Vec<&str> = scan.components.iter().map(|c| c.name.as_str()).collect();
    names.sort();
    assert_eq!(names, vec!["commented-one", "commented-two"]);
}

/// Requirement: A strict JSON file rejects a trailing comma — constructed
/// inline through the `jsonc` seam directly (design's per-file parsing
/// decision: both files go through the same lenient-by-design parser, so
/// this pins that a trailing comma in isolation is not, on its own, a
/// reason for `vertice_core::jsonc::parse` to accept malformed-shaped
/// input beyond what design §5.2 explicitly allows — see
/// `tests/jsonc_behavior.rs` for the seam-level pin of this exact rule).
#[test]
fn a_document_with_a_missing_value_after_trailing_comma_position_is_malformed() {
    let input = r#"{ "agent": { "a": 1, "b": } }"#;

    let result = vertice_core::jsonc::parse(input);

    assert!(
        result.is_err(),
        "a genuinely malformed document must be rejected"
    );
}

/// Requirement: Malformed `opencode.json` does not block `opencode.jsonc`'s
/// agents (CA-12).
#[test]
fn broken_json_fixture_isolates_to_that_file() {
    let home = fixture_home("broken-json");

    let scan = opencode_agents::scan(&home);

    assert_eq!(scan.issues.len(), 1);
    let issue = &scan.issues[0];
    assert_eq!(issue.severity, IssueSeverity::Error);
    let path = issue
        .path
        .as_ref()
        .expect("malformed-file issue must carry a path");
    assert!(path.ends_with("opencode.json"));

    let mut names: Vec<&str> = scan.components.iter().map(|c| c.name.as_str()).collect();
    names.sort();
    assert_eq!(names, vec!["healthy-one", "healthy-two"]);
}

/// Requirement: Malformed `opencode.jsonc` does not block `opencode.json`'s
/// agents — the exact mirror of the case above.
#[test]
fn broken_jsonc_fixture_isolates_to_that_file() {
    let home = fixture_home("broken-jsonc");

    let scan = opencode_agents::scan(&home);

    assert_eq!(scan.issues.len(), 1);
    let issue = &scan.issues[0];
    assert_eq!(issue.severity, IssueSeverity::Error);
    let path = issue
        .path
        .as_ref()
        .expect("malformed-file issue must carry a path");
    assert!(path.ends_with("opencode.jsonc"));

    let mut names: Vec<&str> = scan.components.iter().map(|c| c.name.as_str()).collect();
    names.sort();
    assert_eq!(names, vec!["healthy-one", "healthy-two"]);
}

/// Requirement: Absent files, absent `agent` key, and empty `agent` object
/// each produce zero components and zero issues.
#[test]
fn absent_config_fixture_yields_nothing_no_issue_and_root_not_found() {
    let home = fixture_home("absent-config");

    let scan = opencode_agents::scan(&home);

    assert_eq!(scan.components.len(), 0);
    assert_eq!(scan.issues.len(), 0);
    assert_eq!(scan.roots.len(), 1);
    assert_eq!(
        scan.roots[0].status,
        vertice_core::model::SearchRootStatus::NotFound
    );
    // `SearchRoot.path` is still populated even though nothing was found —
    // "looked in the wrong place" must never be confusable with "found
    // nothing" (design §3).
    assert!(!scan.roots[0].path.as_os_str().is_empty());
}

#[test]
fn no_agent_key_fixture_yields_nothing_no_issue() {
    let home = fixture_home("no-agent-key");

    let scan = opencode_agents::scan(&home);

    assert_eq!(scan.components.len(), 0);
    assert_eq!(scan.issues.len(), 0);
    assert_eq!(
        scan.roots[0].status,
        vertice_core::model::SearchRootStatus::Found
    );
}

#[test]
fn empty_agent_fixture_yields_nothing_no_issue() {
    let home = fixture_home("empty-agent");

    let scan = opencode_agents::scan(&home);

    assert_eq!(scan.components.len(), 0);
    assert_eq!(scan.issues.len(), 0);
}

/// A `NotFound` root is distinguishable from a `Found` root with zero
/// components, even though both scans above report zero components and
/// zero issues.
#[test]
fn not_found_root_is_distinguishable_from_found_root_with_zero_components() {
    let absent = opencode_agents::scan(&fixture_home("absent-config"));
    let empty = opencode_agents::scan(&fixture_home("empty-agent"));

    assert_eq!(
        absent.roots[0].status,
        vertice_core::model::SearchRootStatus::NotFound
    );
    assert_eq!(
        empty.roots[0].status,
        vertice_core::model::SearchRootStatus::Found
    );
    assert_eq!(absent.components.len(), 0);
    assert_eq!(empty.components.len(), 0);
}

/// Requirement: Out-Of-Scope Top-Level Keys Produce No Component — the
/// `mcp`/`share`/`$schema` keys next to a well-formed `agent` key in
/// `no-agent-key/` produce nothing, and no issue references them.
#[test]
fn mcp_key_produces_no_component_and_no_issue() {
    let home = fixture_home("no-agent-key");

    let scan = opencode_agents::scan(&home);

    assert!(scan.components.is_empty());
    assert!(scan.issues.is_empty());
}

/// Requirement: Component Assembly For Every Merged Agent Key — every
/// emitted component carries `kind: Agent`, `scope: User`, and an id
/// derived from the merged key alone.
#[test]
fn every_component_is_agent_kind_user_scoped_with_a_derived_id() {
    let home = fixture_home("json-only");

    let scan = opencode_agents::scan(&home);

    for component in &scan.components {
        assert_eq!(component.kind, ComponentKind::Agent);
        assert_eq!(component.scope, Scope::User);
        assert_eq!(
            component.id,
            ComponentId::derive(ComponentKind::Agent, &component.name)
        );
    }
}

/// Requirement: Component And Issue Ordering Is Deterministic — two
/// consecutive `scan` calls over `reference/` yield byte-identical
/// component and issue vectors, asserted against a literal expected id
/// order (design §7).
#[test]
fn two_runs_over_the_reference_fixture_yield_identical_order() {
    let home = fixture_home("reference");

    let first = opencode_agents::scan(&home);
    let second = opencode_agents::scan(&home);

    assert_eq!(first.components, second.components);
    assert_eq!(first.issues, second.issues);

    let ids: Vec<String> = first
        .components
        .iter()
        .map(|c| c.id.as_str().to_string())
        .collect();
    let expected = [
        ComponentId::derive(ComponentKind::Agent, "alpha"),
        ComponentId::derive(ComponentKind::Agent, "beta"),
        ComponentId::derive(ComponentKind::Agent, "delta"),
        ComponentId::derive(ComponentKind::Agent, "epsilon"),
        ComponentId::derive(ComponentKind::Agent, "eta"),
        ComponentId::derive(ComponentKind::Agent, "gamma"),
        ComponentId::derive(ComponentKind::Agent, "zeta"),
    ];
    let expected_ids: Vec<String> = expected.iter().map(|id| id.as_str().to_string()).collect();
    assert_eq!(
        ids, expected_ids,
        "components must be emitted in sorted merged-key order"
    );
}

/// Requirement: A Normalization Collision Between Two Agent Keys Is
/// Reported, Not Silently Collapsed.
#[test]
fn normalize_collision_fixture_emits_both_components_sharing_one_id() {
    let home = fixture_home("normalize-collision");

    let scan = opencode_agents::scan(&home);

    assert_eq!(scan.components.len(), 2);
    assert_eq!(scan.issues.len(), 0);
    let expected_id = ComponentId::derive(ComponentKind::Agent, "reviewer");
    assert!(scan.components.iter().all(|c| c.id == expected_id));

    let mut names: Vec<&str> = scan.components.iter().map(|c| c.name.as_str()).collect();
    names.sort();
    assert_eq!(names, vec!["Reviewer", "reviewer"]);
}

/// Requirement: An Agent Entry's Body Can Never Prevent The Agent From
/// Being Reported — `malformed-entry/` still emits both components, with
/// `description: None` and a `Warning`-severity issue each (design §8's
/// severity rule: nothing is missing from the inventory).
#[test]
fn malformed_entry_fixture_still_emits_components_with_warnings() {
    let home = fixture_home("malformed-entry");

    let scan = opencode_agents::scan(&home);

    assert_eq!(scan.components.len(), 4, "every key becomes a component");

    let unreadable = ["string-valued", "bad-description"];
    assert!(scan
        .components
        .iter()
        .filter(|c| unreadable.contains(&c.name.as_str()))
        .all(|c| c.description.is_none()));

    // Only the two entries with unreadable metadata raise an issue; the
    // well-formed and the empty-bodied entries raise none (design §8).
    assert_eq!(scan.issues.len(), 2);
    assert!(scan
        .issues
        .iter()
        .all(|i| i.severity == IssueSeverity::Warning));
}

/// Requirement: An Agent Entry's Body Can Never Prevent The Agent From
/// Being Reported, scenario "An unexpected type in the body degrades the
/// field, never the component". `tools` as a **string** is the shape Claude
/// Code uses and the exact wrong-shape guess this capability refuses to
/// make (design §5.4). Because `tools` is never read, it must not disturb
/// the component or raise an issue.
#[test]
fn tools_typed_as_a_string_leaves_the_component_and_its_description_intact() {
    let home = fixture_home("malformed-entry");

    let scan = opencode_agents::scan(&home);

    let component = scan
        .components
        .iter()
        .find(|c| c.name == "tools-as-string")
        .expect("an entry whose `tools` is a string still produces a component");
    assert_eq!(
        component.description.as_deref(),
        Some("Tools typed as a string"),
        "a wrongly-typed field this capability never reads must not cost the description"
    );
    assert!(
        !scan
            .issues
            .iter()
            .any(|i| i.reason.contains("tools-as-string")),
        "an unread field of an unexpected type is not a defect worth reporting"
    );
}

/// Requirement: An Agent Entry's Body Can Never Prevent The Agent From
/// Being Reported, scenario "An agent entry with an empty body still
/// produces a component". A bare `{}` yields a component with no
/// description and **no** `ScanIssue`: an absent field is absence, not
/// unreadable metadata (design §8's severity rule).
#[test]
fn an_entry_with_an_empty_body_produces_a_component_and_no_issue() {
    let home = fixture_home("malformed-entry");

    let scan = opencode_agents::scan(&home);

    let component = scan
        .components
        .iter()
        .find(|c| c.name == "empty-body")
        .expect("an entry with an empty body still produces a component");
    assert!(component.description.is_none());
    assert!(
        !scan.issues.iter().any(|i| i.reason.contains("empty-body")),
        "an absent description is absence, not unreadable metadata"
    );
}

/// CA-5 PIN: the `reference/` fixture pins 7 components, 7 distinct ids,
/// at least one sourced only from `.jsonc`, and at least one carrying two
/// `Location`s.
#[test]
fn reference_fixture_pins_seven_components_seven_ids_and_cross_file_provenance() {
    let home = fixture_home("reference");

    let scan = opencode_agents::scan(&home);

    assert_eq!(scan.components.len(), 7);

    let mut ids: Vec<String> = scan
        .components
        .iter()
        .map(|c| c.id.as_str().to_string())
        .collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), 7, "all 7 ids must be distinct");

    let jsonc_only = ["zeta", "eta"];
    let jsonc_only_count = scan
        .components
        .iter()
        .filter(|c| jsonc_only.contains(&c.name.as_str()))
        .count();
    assert!(
        jsonc_only_count >= 1,
        "at least one component must be sourced only from .jsonc"
    );
    for name in jsonc_only {
        let component = scan
            .components
            .iter()
            .find(|c| c.name == name)
            .unwrap_or_else(|| panic!("expected a component named {name}"));
        assert_eq!(component.locations.len(), 1);
    }

    let epsilon = scan
        .components
        .iter()
        .find(|c| c.name == "epsilon")
        .expect("epsilon must be declared in both files");
    assert_eq!(
        epsilon.locations.len(),
        2,
        "epsilon must carry two Locations"
    );
    assert_eq!(
        epsilon.description.as_deref(),
        Some("Epsilon agent, overridden by opencode.jsonc")
    );
}

/// Requirement: Scanner Performs No Writes — a full scan leaves the
/// fixture tree byte-for-byte unchanged (CA-16).
#[test]
fn full_scan_leaves_the_reference_fixture_tree_unchanged() {
    let home = fixture_home("reference");

    let before = fixture_tree_bytes(&home);
    let _ = opencode_agents::scan(&home);
    let after = fixture_tree_bytes(&home);

    assert_eq!(before, after);
}

fn fixture_tree_bytes(root: &std::path::Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut out = Vec::new();
    for entry in walkdir::WalkDir::new(root)
        .follow_links(false)
        .sort_by_file_name()
    {
        let entry = entry.expect("fixture tree must be fully readable");
        if entry.file_type().is_file() {
            let bytes = std::fs::read(entry.path()).expect("fixture file must be readable");
            out.push((entry.into_path(), bytes));
        }
    }
    out
}

/// Contract: `roots.len() == 1` for every home, regardless of scan
/// outcome (design §3).
#[test]
fn roots_len_is_always_exactly_one() {
    for case in ["absent-config", "reference", "broken-json", "empty-agent"] {
        let scan = opencode_agents::scan(&fixture_home(case));
        assert_eq!(
            scan.roots.len(),
            1,
            "case {case} must resolve exactly one root"
        );
    }
}

/// Structural sanity: `roots::opencode_agent_root` is reachable from the
/// crate root the same way `opencode_agents::scan` is (guards against an
/// accidental private-only regression to either symbol).
#[test]
fn opencode_agent_root_is_reachable_from_the_crate_root() {
    let resolved = roots::opencode_agent_root(&fixture_home("reference"));

    assert!(resolved.root.path.ends_with("opencode.json"));
}
