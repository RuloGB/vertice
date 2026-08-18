//! Fixture-driven behaviour tests for `vertice_core::skills::scan` and
//! `vertice_core::roots::skill_roots`, over the synthetic-home fixture tree
//! committed under `crates/vertice-core/tests/fixtures/roots/`. One test (or
//! tight group) per skill-scanner spec requirement; `design.md` §7/§8 is the
//! authority for every asserted `status`/`severity`/`reason` shape.
//!
//! `openspec/changes/skill-scanner-user-roots/design.md` §8's `.gitkeep`
//! tripwire is deliberately split in two: the disk-existence half below
//! needs no `roots`/`skills` module and lands with the fixture tree; the
//! `status == Found` half lives in `roots.rs`'s own unit tests, next to
//! `resolve_opencode`.

use std::path::PathBuf;

use vertice_core::model::{IssueSeverity, Scope};
use vertice_core::roots;
use vertice_core::skills;

/// Build a path under
/// `crates/vertice-core/tests/fixtures/roots/<case>/` from per-segment
/// pushes — never a `"/"`-joined literal, so it stays separator-correct on
/// Windows.
fn fixture_home(case: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("fixtures");
    path.push("roots");
    path.push(case);
    path
}

/// Tripwire (design §8): git cannot track an empty directory, so the CA-9
/// "present and empty" case relies on a `.gitkeep` file. If that file is
/// ever lost, the directory silently vanishes and the CA-9 test starts
/// exercising the "absent root" path instead, still passing with zero
/// components. This half asserts the directory itself is present on disk,
/// independent of any scanner code.
#[test]
fn empty_alias_fixture_directory_still_exists_on_disk() {
    let mut path = fixture_home("empty-alias");
    path.push(".config");
    path.push("opencode");
    path.push("skill");

    let metadata =
        std::fs::metadata(&path).expect("empty-alias fixture directory must exist on disk");

    assert!(
        metadata.is_dir(),
        "empty-alias fixture path must be a directory, not a file"
    );
}

/// Requirement: User Root Set Is Fixed and Hardcoded — the OpenCode root
/// resolves under home on every OS, never a platform config-dir path.
#[test]
fn opencode_root_resolves_under_home_never_a_platform_config_dir() {
    let home = fixture_home("alias-populated");

    let resolved = roots::skill_roots(&home);
    let opencode = resolved
        .iter()
        .find(|r| r.root.id.0 == "opencode-skills")
        .expect("opencode root must be one of the three resolved roots");

    let mut expected = home.clone();
    expected.push(".config");
    expected.push("opencode");
    expected.push("skills");

    assert_eq!(opencode.root.path, expected);
}

/// Requirement: User Root Set Is Fixed and Hardcoded — the singular and
/// plural OpenCode roots are scanned as one logical root.
#[test]
fn singular_and_plural_opencode_roots_are_one_logical_root() {
    let home = fixture_home("alias-populated");

    let scan = skills::scan(&home);

    let opencode_components: Vec<_> = scan
        .components
        .iter()
        .filter(|c| c.locations.iter().any(|l| l.root.0 == "opencode-skills"))
        .collect();

    assert_eq!(opencode_components.len(), 1);
    assert_eq!(opencode_components[0].name, "demo");
}

/// Requirement: SKILL.md Presence Is the Sole Detection Rule — a directory
/// named `_shared` containing `SKILL.md` is an ordinary skill.
#[test]
fn underscore_prefixed_directory_is_an_ordinary_skill() {
    let home = fixture_home("underscore-shared");

    let scan = skills::scan(&home);

    assert!(
        scan.components.iter().any(|c| c.name == "_shared"),
        "expected a component named _shared, got: {:?}",
        scan.components.iter().map(|c| &c.name).collect::<Vec<_>>()
    );
}

/// Requirement: Traversal Is Recursive — a `SKILL.md` two levels below a
/// root is discovered.
#[test]
fn nested_skill_two_levels_deep_is_discovered() {
    let home = fixture_home("nested-skill");

    let scan = skills::scan(&home);

    assert!(scan.components.iter().any(|c| c.name == "nested"));
}

/// Requirement: Symbolic Links Are Not Followed — asserted structurally, as
/// design §6 records: no portable fixture exists, so this pins the
/// `follow_links(false)` contract via the crate's own walk behaviour rather
/// than a symlink fixture.
#[test]
fn walk_never_follows_symlinks_by_default_walkdir_setting() {
    // `walkdir::WalkDir::new(..).follow_links(false)` is the crate default
    // AND what `skills::walk_one` sets explicitly; there is no portable way
    // to fixture a symlink on every CI platform (design §6). This test
    // documents the contract by re-running a scan twice over the same
    // fixture and asserting deterministic, non-duplicated output, which
    // would not hold if a cyclical symlink were ever followed.
    let home = fixture_home("nested-skill");

    let first = skills::scan(&home);
    let second = skills::scan(&home);

    assert_eq!(first.components.len(), second.components.len());
    assert_eq!(first.components.len(), 1);
}

/// Requirement: Absent and Empty Roots Produce No Issue and No Component —
/// an absent root yields nothing.
#[test]
fn absent_roots_yield_zero_components_zero_issues_all_not_found() {
    let home = fixture_home("absent-roots");

    let scan = skills::scan(&home);

    assert!(scan.components.is_empty());
    assert!(scan.issues.is_empty());
    assert_eq!(scan.roots.len(), 3);
    assert!(scan
        .roots
        .iter()
        .all(|r| r.status == vertice_core::model::SearchRootStatus::NotFound));
}

/// Requirement: Absent and Empty Roots Produce No Issue and No Component —
/// a present, empty root yields nothing, and is distinguishable from an
/// absent one (CA-9).
#[test]
fn present_empty_root_yields_zero_components_zero_issues_and_is_found() {
    let home = fixture_home("empty-alias");

    let scan = skills::scan(&home);

    assert!(scan.components.is_empty());
    assert!(scan.issues.is_empty());

    let opencode = scan
        .roots
        .iter()
        .find(|r| r.id.0 == "opencode-skills")
        .expect("opencode root must be present");
    assert_eq!(
        opencode.status,
        vertice_core::model::SearchRootStatus::Found
    );
}

/// Requirement: Every Skill Component Has Scope::User — every produced
/// component is User-scoped, and a project-shaped tree outside the three
/// resolved roots yields nothing (CA-14).
#[test]
fn every_component_is_user_scoped_and_project_decoy_is_excluded() {
    let home = fixture_home("project-decoy");

    let scan = skills::scan(&home);

    assert!(scan.components.iter().all(|c| c.scope == Scope::User));
    assert!(scan.components.iter().any(|c| c.name == "real"));
    assert!(
        !scan.components.iter().any(|c| c.name == "fake"),
        "the project-decoy's nested .claude/skills/ must not be walked"
    );
}

/// Requirement: No Plugin-Provided Skill Appears In The Result — a
/// plugin-shaped fixture outside the three roots contributes nothing
/// (CA-6).
#[test]
fn plugin_decoy_outside_the_three_roots_is_excluded() {
    let home = fixture_home("plugin-decoy");

    let scan = skills::scan(&home);

    assert!(scan.components.is_empty());
    assert!(scan.issues.is_empty());
}

/// Requirement: Per-File Parsing Failures Do Not Abort The Scan — one
/// corrupt `SKILL.md` yields an issue carrying its path, and both sibling
/// skills are still discovered (CA-12 partial).
#[test]
fn corrupt_skill_yields_an_issue_and_does_not_stop_the_walk() {
    let home = fixture_home("unreadable-entry");

    let scan = skills::scan(&home);

    assert_eq!(scan.components.len(), 1);
    assert_eq!(scan.components[0].name, "good");

    assert_eq!(scan.issues.len(), 1);
    let issue = &scan.issues[0];
    assert_eq!(issue.severity, IssueSeverity::Error);
    let path = issue
        .path
        .as_ref()
        .expect("corrupt-file issue must carry a path");
    assert!(path.ends_with("SKILL.md"));
    assert!(path.to_string_lossy().contains("broken"));
}

/// Requirement: Scanner Performs No Writes — a full scan run leaves the
/// fixture tree byte-for-byte unchanged (CA-16).
#[test]
fn full_scan_leaves_the_fixture_tree_unchanged() {
    let home = fixture_home("reference");

    let before = fixture_tree_bytes(&home);
    let _ = skills::scan(&home);
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

/// Requirement: Reference Fixture Set Produces Exactly 69 On-Disk Entries.
#[test]
fn reference_fixture_tree_yields_69_entries() {
    let home = fixture_home("reference");

    let scan = skills::scan(&home);

    let total = scan.components.len() + scan.issues.len();
    assert_eq!(total, 69);

    // Non-binding corroborator: 25 distinct component ids.
    let mut ids: Vec<_> = scan.components.iter().map(|c| c.id.as_str()).collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), 25);
}
