//! Fixture-driven behaviour tests for `vertice_core::codex_agents::scan`,
//! over the synthetic-home fixture tree committed under
//! `crates/vertice-core/tests/fixtures/roots/codex-agents/`. One test (or
//! tight group) per `codex-agent-scanner` spec requirement; `design.md` §6.3
//! and §7.1 are the authority for every asserted shape.

use std::path::PathBuf;

use vertice_core::codex_agents::{self, CodexAgentDocument};
use vertice_core::model::{ComponentKind, IssueSeverity, LocationOrigin, Scope};

/// Build a path under
/// `crates/vertice-core/tests/fixtures/roots/codex-agents/<case>/` from
/// per-segment pushes — never a `"/"`-joined literal, so it stays
/// separator-correct on Windows.
fn fixture_home(case: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("fixtures");
    path.push("roots");
    path.push("codex-agents");
    path.push(case);
    path
}

/// Flat discovery of a direct `.toml`; a nested file is NOT discovered; a
/// non-`.toml` sibling file is ignored.
#[test]
fn flat_discovery_ignores_nested_files_and_non_toml_siblings() {
    let home = fixture_home("complete");

    let scan = codex_agents::scan(&home);

    assert!(scan.components.iter().any(|c| c.name == "reviewer"));
    assert!(scan.components.iter().any(|c| c.name == "planner"));
    assert!(
        !scan.components.iter().any(|c| c.name == "nested-agent"),
        "a .toml nested one level below the root must not be discovered"
    );
    assert_eq!(
        scan.components.len(),
        2,
        "notes.txt (non-.toml) must be silently ignored"
    );
    assert!(scan.issues.is_empty());
}

/// The multiline `developer_instructions` is parsed complete and byte-exact
/// **from the committed fixture file** — the seam's reason for existing, and
/// the literal wording of the `codex-agent-scanner` scenario ("GIVEN a
/// fixture Codex agent `.toml` file"). `toml_behavior.rs` pins the same
/// guarantee against an inline literal; this test pins it against real bytes
/// on disk, where a truncation at the first quote or the first newline would
/// actually surface.
///
/// It reads the DTO directly rather than going through `codex_agents::scan`
/// because `Component` carries no `developer_instructions` field — the value
/// is deliberately dropped during assembly (design §6.3), so the adapter's
/// output cannot express it.
///
/// The expected value starts at "You are" and ends with a trailing newline:
/// TOML trims the newline immediately following the opening `"""` delimiter
/// and nothing else. `.gitattributes` marks the fixture tree `-text`, so
/// these bytes are LF on every platform.
#[test]
fn codex_agent_with_multiline_developer_instructions_yields_the_complete_value() {
    let mut path = fixture_home("complete");
    path.push(".codex");
    path.push("agents");
    path.push("planner.toml");

    let raw = std::fs::read_to_string(&path).expect("planner.toml fixture must be readable");
    let document: CodexAgentDocument =
        vertice_core::toml::from_str(&raw).expect("the fixture must parse through the seam");

    assert_eq!(
        document.developer_instructions.as_deref(),
        Some(
            "You are a planning agent.\n\
             \n\
             Follow these steps:\n\
             \n\
             1. Read the request.\n\
             2. Draft a plan.\n"
        ),
        "the multiline value must survive the seam complete and byte-exact"
    );
    assert_eq!(document.name, "planner");
    assert_eq!(document.description.as_deref(), Some("Plans work."));
}

/// Deliberately NOT mapped onto `Component` (design §6.3): `Component`
/// carries no `developer_instructions` field at all. Here we assert the
/// component assembly shape instead.
#[test]
fn component_assembly_shape_is_agent_user_one_file_location() {
    let home = fixture_home("complete");

    let scan = codex_agents::scan(&home);

    let reviewer = scan
        .components
        .iter()
        .find(|c| c.name == "reviewer")
        .expect("reviewer agent must be discovered");
    assert_eq!(reviewer.kind, ComponentKind::Agent);
    assert_eq!(reviewer.scope, Scope::User);
    assert_eq!(reviewer.description.as_deref(), Some("Reviews code."));
    assert_eq!(reviewer.locations.len(), 1);
    assert!(reviewer.locations[0].path.is_some());
    assert_eq!(reviewer.locations[0].origin, LocationOrigin::File);
    assert_eq!(reviewer.locations[0].root.0, "codex-agents");
}

/// Absent root -> zero components, zero issues (CA-11).
#[test]
fn absent_root_yields_zero_components_and_zero_issues() {
    let home = PathBuf::from("/definitely/does/not/exist/vertice-codex-agents-absent");

    let scan = codex_agents::scan(&home);

    assert!(scan.components.is_empty());
    assert!(scan.issues.is_empty());
}

/// Empty root -> zero components, zero issues.
#[test]
fn empty_root_yields_zero_components_and_zero_issues() {
    let home = fixture_home("empty");

    let scan = codex_agents::scan(&home);

    assert!(scan.components.is_empty());
    assert!(scan.issues.is_empty());
}

/// Extra/unmodelled keys, including a nested table, are ignored, not an
/// error (design §8's TOML analogue).
#[test]
fn extra_unmodelled_keys_including_a_nested_table_are_ignored() {
    let home = fixture_home("extra-keys");

    let scan = codex_agents::scan(&home);

    assert_eq!(scan.components.len(), 1);
    assert_eq!(scan.components[0].name, "extra");
    assert_eq!(
        scan.components[0].description.as_deref(),
        Some("Has unmodelled keys.")
    );
    assert!(scan.issues.is_empty());
}

/// A source-inspection test: no regex is used to parse `.toml` content
/// (AGENTS.md's frontmatter-parsing prohibition, extended to this adapter).
#[test]
fn source_does_not_use_regex_to_parse_toml_content() {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("src");
    path.push("codex_agents.rs");

    let source = std::fs::read_to_string(&path).expect("codex_agents.rs must be readable");

    assert!(
        !source.contains("regex") && !source.contains("Regex"),
        "codex_agents.rs must never reference a regex crate or type"
    );
}

/// Per-file isolation (CA-12): one malformed `.toml` and one missing-`name`
/// `.toml` each yield one `Error` `ScanIssue` with its own path, and both
/// well-formed siblings are still discovered.
#[test]
fn malformed_and_missing_name_files_are_isolated_from_valid_siblings() {
    let home = fixture_home("corrupt");

    let scan = codex_agents::scan(&home);

    assert_eq!(scan.components.len(), 2, "the two valid siblings survive");
    assert!(scan.components.iter().any(|c| c.name == "good-sibling-one"));
    assert!(scan.components.iter().any(|c| c.name == "good-sibling-two"));

    assert_eq!(scan.issues.len(), 2, "one Error per broken file");
    for issue in &scan.issues {
        assert_eq!(issue.severity, IssueSeverity::Error);
        assert!(issue.path.is_some(), "every issue here must carry a path");
    }
    let broken_path_present = scan
        .issues
        .iter()
        .any(|i| i.path.as_ref().is_some_and(|p| p.ends_with("broken.toml")));
    let no_name_path_present = scan
        .issues
        .iter()
        .any(|i| i.path.as_ref().is_some_and(|p| p.ends_with("no-name.toml")));
    assert!(broken_path_present);
    assert!(no_name_path_present);
}

/// A root-shape error: `.codex/agents` exists but is a file, not a
/// directory.
#[test]
fn root_that_is_not_a_directory_yields_one_error() {
    let home = fixture_home("not-a-directory");

    let scan = codex_agents::scan(&home);

    assert!(scan.components.is_empty());
    assert_eq!(scan.issues.len(), 1);
    assert_eq!(scan.issues[0].severity, IssueSeverity::Error);
    assert!(scan.issues[0].reason.contains("not a directory"));
}

/// CA-16: a full scan over `complete/` leaves the fixture tree byte-for-byte
/// unchanged.
#[test]
fn full_scan_leaves_the_fixture_tree_unchanged() {
    let home = fixture_home("complete");

    let before = fixture_tree_bytes(&home);
    let _ = codex_agents::scan(&home);
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
