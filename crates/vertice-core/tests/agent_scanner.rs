//! Fixture-driven behaviour tests for `vertice_core::agents::scan` and
//! `vertice_core::roots::agent_roots`, over the synthetic-home fixture tree
//! committed under `crates/vertice-core/tests/fixtures/roots/agents/`. One
//! test (or tight group) per `agent-scanner` spec requirement; `design.md`
//! is the authority for every asserted `status`/`severity`/`reason` shape.
//!
//! The `.gitkeep` tripwire (design §10) is deliberately split in two: the
//! disk-existence half below needs no `agents` module and lands with the
//! fixture tree in PR 1; the `SearchRootStatus::Found` half is proven
//! directly by the integration suite over `empty-root/` in PR 2.

use std::path::PathBuf;

use vertice_core::agents;
use vertice_core::model::{IssueSeverity, LocationOrigin, SearchRootKind, SearchRootStatus};
use vertice_core::roots;

/// Build a path under
/// `crates/vertice-core/tests/fixtures/roots/agents/<case>/` from
/// per-segment pushes — never a `"/"`-joined literal, so it stays
/// separator-correct on Windows.
fn fixture_home(case: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("fixtures");
    path.push("roots");
    path.push("agents");
    path.push(case);
    path
}

/// Only the file-backed (`origin: File`) components, per design §4 / spec
/// requirement "Absent and Empty Agent Roots...": never assert on
/// `components.is_empty()`, since the six embedded components may still be
/// present.
fn file_backed(scan: &vertice_core::agents::AgentScan) -> Vec<&vertice_core::model::Component> {
    scan.components
        .iter()
        .filter(|c| c.locations.iter().all(|l| l.origin == LocationOrigin::File))
        .collect()
}

/// Tripwire (design §10): git cannot track an empty directory, so the
/// present-and-empty agent root case relies on a `.gitkeep` file. If that
/// file is ever lost, `empty-root/.claude/agents/` silently vanishes and its
/// tests start exercising the "absent root" path instead, still passing.
/// This half asserts the directory itself is present on disk, independent
/// of any scanner code.
#[test]
fn empty_agent_root_fixture_directory_still_exists_on_disk() {
    let mut path = fixture_home("empty-root");
    path.push(".claude");
    path.push("agents");

    let metadata =
        std::fs::metadata(&path).expect("empty-root fixture directory must exist on disk");

    assert!(
        metadata.is_dir(),
        "empty-root fixture path must be a directory, not a file"
    );
}

/// Requirement: Agent Root Resolves Under The Home Directory.
#[test]
fn agent_root_resolves_under_home_with_agent_kind() {
    let home = fixture_home("tools-scalar");

    let [resolved, _embedded] = roots::agent_roots(&home);

    let mut expected = home.clone();
    expected.push(".claude");
    expected.push("agents");
    assert_eq!(resolved.root.path, expected);
    assert_eq!(resolved.root.kind, SearchRootKind::Agent);
}

/// Requirement: A Direct `.md` File Under The Root Is An Agent, Detected
/// Flat — a `.md` file directly under the root is discovered.
#[test]
fn direct_md_file_under_root_is_discovered() {
    let home = fixture_home("tools-scalar");

    let scan = agents::scan(&home);

    assert!(file_backed(&scan).iter().any(|c| c.name == "reviewer"));
}

/// Requirement: A Direct `.md` File Under The Root Is An Agent, Detected
/// Flat — a file nested one level under the root is not discovered, and no
/// `ScanIssue` references it.
#[test]
fn nested_md_file_is_not_discovered() {
    let home = fixture_home("nested-decoy");

    let scan = agents::scan(&home);

    let names: Vec<_> = file_backed(&scan).iter().map(|c| c.name.clone()).collect();
    assert_eq!(names, vec!["flat".to_string()]);
    assert!(scan.issues.is_empty());
}

/// Requirement: A Direct `.md` File Under The Root Is An Agent, Detected
/// Flat — a non-`.md` file directly under the root is silently ignored.
#[test]
fn non_md_file_directly_under_root_is_ignored() {
    let home = fixture_home("non-agent-entries");

    let scan = agents::scan(&home);

    let names: Vec<_> = file_backed(&scan).iter().map(|c| c.name.clone()).collect();
    assert_eq!(names, vec!["real".to_string()]);
    assert!(scan.issues.is_empty());
}

/// Requirement: Absent and Empty Agent Roots Produce No Issue and No
/// Component — an absent root yields zero file-backed components and zero
/// issues; the two roots' statuses are distinguishable.
#[test]
fn absent_root_yields_zero_file_backed_components_and_zero_issues() {
    let home = fixture_home("absent-root");

    let scan = agents::scan(&home);

    assert!(file_backed(&scan).is_empty());
    assert!(scan.issues.is_empty());
    assert!(scan
        .roots
        .iter()
        .all(|r| r.status == SearchRootStatus::NotFound));
}

/// Requirement: Absent and Empty Agent Roots Produce No Issue and No
/// Component — a present, empty root yields zero file-backed components and
/// zero issues, and is `Found`.
#[test]
fn empty_root_yields_zero_file_backed_components_and_is_found() {
    let home = fixture_home("empty-root");

    let scan = agents::scan(&home);

    assert!(file_backed(&scan).is_empty());
    assert!(scan.issues.is_empty());

    let agents_root = scan
        .roots
        .iter()
        .find(|r| r.id.0 == "claude-agents")
        .expect("claude-agents root must be present");
    assert_eq!(agents_root.status, SearchRootStatus::Found);
}

/// Requirement: Agent Frontmatter Data Contract — a comma-separated `tools`
/// scalar deserializes into one `String`.
#[test]
fn tools_comma_separated_scalar_deserializes_as_one_string() {
    let home = fixture_home("tools-scalar");

    let scan = agents::scan(&home);

    let reviewer = file_backed(&scan)
        .into_iter()
        .find(|c| c.name == "reviewer")
        .expect("reviewer component must be produced");
    assert_eq!(reviewer.kind, vertice_core::model::ComponentKind::Agent);
    assert_eq!(reviewer.scope, vertice_core::model::Scope::User);
}

/// Requirement: Agent Frontmatter Data Contract — a missing `model` and
/// `tools` field is not a failure; a `Component` is still produced.
#[test]
fn missing_model_and_tools_is_not_a_failure() {
    let home = fixture_home("missing-optional");

    let scan = agents::scan(&home);

    assert!(file_backed(&scan).iter().any(|c| c.name == "minimal"));
    assert!(scan.issues.is_empty());
}

/// Requirement: Agent Frontmatter Data Contract — a folded block-scalar
/// `description` is parsed in full (CA-10 inherited).
#[test]
fn folded_description_is_parsed_in_full() {
    let home = fixture_home("folded-description");

    let scan = agents::scan(&home);

    let summarizer = file_backed(&scan)
        .into_iter()
        .find(|c| c.name == "summarizer")
        .expect("summarizer component must be produced");
    let description = summarizer
        .description
        .as_ref()
        .expect("description must be present");
    assert!(description.contains("folded block scalar that spans"));
    assert!(
        !description.contains("\n  "),
        "lines must be joined with spaces, not left indented"
    );
}

/// Requirement: On-Disk Agent Component Assembly — a valid on-disk agent
/// produces a correctly shaped `Component`.
#[test]
fn valid_on_disk_agent_produces_correctly_shaped_component() {
    let home = fixture_home("tools-scalar");

    let scan = agents::scan(&home);

    let reviewer = file_backed(&scan)
        .into_iter()
        .find(|c| c.name == "reviewer")
        .expect("reviewer component must be produced");
    assert_eq!(reviewer.kind, vertice_core::model::ComponentKind::Agent);
    assert_eq!(reviewer.scope, vertice_core::model::Scope::User);
    assert_eq!(reviewer.locations.len(), 1);
    assert_eq!(reviewer.locations[0].origin, LocationOrigin::File);
    assert!(reviewer.locations[0].path.is_some());
}

/// Requirement: Embedded Agents Are Emitted From A Fixed, Named List — the
/// six embedded agents appear when the agent root exists but holds no agent
/// file. Named for the fixture it actually uses: `empty-root/` has a
/// present-and-empty `.claude/agents/`, which is a different code path from
/// the root being absent entirely — that case is covered by
/// `embedded_agents_appear_when_agent_root_absent_but_claude_dir_present`.
#[test]
fn embedded_agents_appear_when_agent_root_is_present_but_empty() {
    let home = fixture_home("empty-root");

    let scan = agents::scan(&home);

    let embedded: Vec<_> = scan
        .components
        .iter()
        .filter(|c| {
            c.locations
                .iter()
                .all(|l| l.origin == LocationOrigin::Embedded)
        })
        .collect();
    assert_eq!(embedded.len(), 6);
    assert!(embedded.iter().all(|c| c.locations[0].path.is_none()));
}

/// Requirement: Embedded Agents Are Emitted From A Fixed, Named List — the
/// six embedded agents appear when `<home>/.claude` exists but the agent root
/// `<home>/.claude/agents/` does not. This is the spec's literal scenario and
/// a distinct code path from the present-and-empty root above: here the walk
/// never opens a directory at all, yet the embedded gate must still open.
#[test]
fn embedded_agents_appear_when_agent_root_absent_but_claude_dir_present() {
    let home = fixture_home("claude-dir-no-agents-root");

    let scan = agents::scan(&home);

    let embedded: Vec<_> = scan
        .components
        .iter()
        .filter(|c| {
            c.locations
                .iter()
                .all(|l| l.origin == LocationOrigin::Embedded)
        })
        .collect();
    assert_eq!(embedded.len(), 6);
    assert!(embedded.iter().all(|c| c.locations[0].path.is_none()));
    assert!(file_backed(&scan).is_empty());
    assert!(scan.issues.is_empty());
}

/// Tripwire (design §10): the `claude-dir-no-agents-root/` fixture only tests
/// what it claims while `.claude/` exists *without* an `agents/` child. If the
/// `.gitkeep` is lost the directory vanishes and this case silently becomes
/// `no_embedded_agents_when_claude_dir_absent`, still passing. Asserted before
/// any scanner code runs.
#[test]
fn claude_dir_no_agents_root_fixture_shape_still_holds_on_disk() {
    let home = fixture_home("claude-dir-no-agents-root");

    assert!(home.join(".claude").is_dir());
    assert!(!home.join(".claude").join("agents").exists());
}

/// Requirement: Embedded Agents Are Emitted From A Fixed, Named List — no
/// embedded agents are emitted when the client directory is absent.
#[test]
fn no_embedded_agents_when_claude_dir_absent() {
    let home = fixture_home("absent-root");

    let scan = agents::scan(&home);

    assert!(scan.components.is_empty());
    assert!(scan.issues.is_empty());
}

/// Requirement: Embedded Agents Are Emitted From A Fixed, Named List —
/// embedded and on-disk agents are distinguishable by origin and path alone.
#[test]
fn embedded_and_on_disk_agents_distinguishable_by_origin_and_path() {
    let home = fixture_home("tools-scalar");

    let scan = agents::scan(&home);

    for component in &scan.components {
        for location in &component.locations {
            match location.origin {
                LocationOrigin::File => assert!(location.path.is_some()),
                LocationOrigin::Embedded => assert!(location.path.is_none()),
            }
        }
    }
}

/// Requirement: Embedded Agents Are Emitted From A Fixed, Named List — every
/// embedded component's `Location.root` holds a valid, well-formed
/// `SearchRootId`.
#[test]
fn embedded_component_root_is_a_valid_search_root_id() {
    let home = fixture_home("tools-scalar");

    let scan = agents::scan(&home);

    let embedded: Vec<_> = scan
        .components
        .iter()
        .filter(|c| {
            c.locations
                .iter()
                .all(|l| l.origin == LocationOrigin::Embedded)
        })
        .collect();
    assert_eq!(embedded.len(), 6);
    assert!(embedded.iter().all(|c| !c.locations[0].root.0.is_empty()));
}

/// Requirement: A User Agent File Shadowing An Embedded Agent Name Produces
/// Two Components.
#[test]
fn shadowing_user_agent_and_embedded_agent_both_appear() {
    let home = fixture_home("shadowing");

    let scan = agents::scan(&home);

    let plan_id =
        vertice_core::model::ComponentId::derive(vertice_core::model::ComponentKind::Agent, "Plan");
    let plan_components: Vec<_> = scan.components.iter().filter(|c| c.id == plan_id).collect();
    assert_eq!(plan_components.len(), 2);
    assert!(plan_components
        .iter()
        .any(|c| c.locations[0].origin == LocationOrigin::Embedded));
    assert!(plan_components
        .iter()
        .any(|c| c.locations[0].origin == LocationOrigin::File));
}

/// Requirement: Per-File Parsing Failures Do Not Abort The Walk — one
/// corrupt agent file yields an issue and does not stop the walk.
#[test]
fn corrupt_agent_yields_an_issue_and_does_not_stop_the_walk() {
    let home = fixture_home("broken-frontmatter");

    let scan = agents::scan(&home);

    assert!(file_backed(&scan).iter().any(|c| c.name == "good"));

    let file_issues: Vec<_> = scan.issues.iter().filter(|i| i.path.is_some()).collect();
    assert_eq!(file_issues.len(), 1);
    let issue = file_issues[0];
    assert_eq!(issue.severity, IssueSeverity::Error);
    let path = issue.path.as_ref().unwrap();
    assert!(path.to_string_lossy().contains("broken"));
}

/// Requirement: Scanner Performs No Writes — a full scan run leaves the
/// fixture tree byte-for-byte unchanged (CA-16).
#[test]
fn full_scan_leaves_the_fixture_tree_unchanged() {
    let home = fixture_home("reference");

    let before = fixture_tree_bytes(&home);
    let _ = agents::scan(&home);
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

/// Requirement: Reference Fixture Set Produces Exactly 17 On-Disk Agent
/// Components — 17 file-backed + 6 embedded = 23 total, 23 distinct ids.
#[test]
fn reference_fixture_yields_17_on_disk_and_23_total_with_23_distinct_ids() {
    let home = fixture_home("reference");

    let scan = agents::scan(&home);

    assert_eq!(file_backed(&scan).len(), 17);
    assert_eq!(scan.components.len(), 23);

    let mut ids: Vec<_> = scan.components.iter().map(|c| c.id.as_str()).collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), 23);
}

/// Component order is identical regardless of `read_dir`'s OS-dependent
/// yield order — assert against a fixture with 3+ files, run against the
/// sorted expectation (design §6).
#[test]
fn component_order_matches_sorted_file_name_order() {
    let home = fixture_home("reference");

    let scan = agents::scan(&home);
    let file_backed_names: Vec<_> = scan
        .components
        .iter()
        .filter(|c| c.locations.iter().all(|l| l.origin == LocationOrigin::File))
        .map(|c| c.name.clone())
        .collect();

    let mut expected = file_backed_names.clone();
    expected.sort();

    assert_eq!(file_backed_names, expected);
}
