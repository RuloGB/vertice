//! Fixture-driven behaviour tests for `vertice_core::installations::{scan,
//! scan_for}`, over the synthetic-home fixture tree committed under
//! `crates/vertice-core/tests/fixtures/installations/` (design §10). One
//! test (or tight group) per `client-installation-detector` spec
//! requirement; `openspec/changes/2026-08-19-client-installation-detection/design.md`
//! is the authority for every asserted `severity`/`reason` shape.
//!
//! `two-claude/` (task 1.8, CA-7 pin) is written FIRST, before any other
//! test in this file, per design §10: it "must exist and FAIL before the
//! assembly code is written."

use std::path::PathBuf;

use vertice_core::installations::{self, HostPlatform};
use vertice_core::model::{ClientInstallation, ClientKind, IssueSeverity};

/// Build a path under
/// `crates/vertice-core/tests/fixtures/installations/<case>/` from
/// per-segment pushes — never a `"/"`-joined literal, so it stays
/// separator-correct on Windows (`tests/skill_scanner.rs:23-30`'s pattern).
fn fixture_home(case: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("fixtures");
    path.push("installations");
    path.push(case);
    path
}

/// Requirement: Claude Code npm And Desktop Are Never Merged (CA-7).
/// **Primary safeguard for this change** — written first, before any other
/// test in this file (design §10, task 2.3).
#[test]
fn two_claude_fixture_yields_two_never_merged_claude_installations() {
    let home = fixture_home("two-claude");

    let scan = installations::scan_for(&home, HostPlatform::Windows);

    let claude_code: Vec<_> = scan
        .installations
        .iter()
        .filter(|i| i.client == ClientKind::ClaudeCode)
        .collect();
    assert_eq!(
        claude_code.len(),
        2,
        "exactly two ClaudeCode installations, never merged"
    );

    let mut versions: Vec<&str> = claude_code.iter().map(|i| i.version.as_str()).collect();
    versions.sort();
    assert_eq!(versions, vec!["1.0.100", "2.5.3"]);

    let mut paths: Vec<&std::path::Path> = claude_code.iter().map(|i| i.path.as_path()).collect();
    paths.sort();
    paths.dedup();
    assert_eq!(paths.len(), 2, "the two installations have distinct paths");

    assert_eq!(scan.issues.len(), 0, "the fixture is fully healthy");
}

/// Requirement: An Absent Slot Is Reported As An Explicit "Not Detected"
/// Signal (CA-11 pin, task 1.7) — every slot is absent, no `Error`.
#[test]
fn nothing_fixture_yields_zero_installations_and_three_warnings_never_an_error() {
    let home = fixture_home("nothing");

    let scan = installations::scan_for(&home, HostPlatform::Windows);

    assert_eq!(scan.installations.len(), 0);
    assert_eq!(scan.issues.len(), 3);
    for issue in &scan.issues {
        assert_eq!(issue.severity, IssueSeverity::Warning);
        assert!(
            issue.path.is_some(),
            "each not-detected warning must carry its probe path"
        );
        assert!(issue.reason.ends_with("not detected"));
    }
    assert!(
        !scan
            .issues
            .iter()
            .any(|i| i.severity == IssueSeverity::Error),
        "an absent client must never be reported as an Error"
    );
}

/// Requirement: A present OpenCode npm install is reported.
#[test]
fn opencode_npm_fixture_yields_one_opencode_installation() {
    let home = fixture_home("opencode-npm");

    let scan = installations::scan_for(&home, HostPlatform::Windows);

    let opencode: Vec<_> = scan
        .installations
        .iter()
        .filter(|i| i.client == ClientKind::OpenCode)
        .collect();
    assert_eq!(opencode.len(), 1);
    assert_eq!(opencode[0].version, "0.4.2");
}

/// Requirement: Each Slot Fails Independently (task 1.9, NON-NEGOTIABLE).
#[test]
fn isolation_fixture_isolates_one_malformed_slot_from_the_other_two() {
    let home = fixture_home("isolation");

    let scan = installations::scan_for(&home, HostPlatform::Windows);

    assert_eq!(
        scan.issues.len(),
        1,
        "exactly one Error on the malformed slot"
    );
    let issue = &scan.issues[0];
    assert_eq!(issue.severity, IssueSeverity::Error);
    let path = issue
        .path
        .as_ref()
        .expect("malformed slot issue must carry a path");
    assert!(path.ends_with("package.json"));

    assert_eq!(
        scan.installations.len(),
        2,
        "the other two installations are still detected"
    );
    assert!(scan
        .installations
        .iter()
        .any(|i| i.client == ClientKind::ClaudeCode && i.version == "1.1.1"));
    assert!(scan
        .installations
        .iter()
        .any(|i| i.client == ClientKind::OpenCode && i.version == "2.2.2"));
}

/// Requirement: A Malformed Or Unreadable package.json Produces An Error,
/// Never A Phantom Installation — a `package.json` with no `"version"` key.
#[test]
fn no_version_key_fixture_yields_no_phantom_installation_and_one_error() {
    let home = fixture_home("no-version-key");

    let scan = installations::scan_for(&home, HostPlatform::Windows);

    assert!(!scan
        .installations
        .iter()
        .any(|i| i.client == ClientKind::OpenCode));
    let errors: Vec<_> = scan
        .issues
        .iter()
        .filter(|i| i.severity == IssueSeverity::Error)
        .collect();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].reason, "package.json has no \"version\" string");
}

/// `"version"` present but not a string — same collapsed reason as the
/// missing-key case (design §8's collapsed row).
#[test]
fn version_not_a_string_fixture_yields_the_same_collapsed_reason() {
    let home = fixture_home("version-not-a-string");

    let scan = installations::scan_for(&home, HostPlatform::Windows);

    assert!(!scan
        .installations
        .iter()
        .any(|i| i.client == ClientKind::OpenCode));
    let errors: Vec<_> = scan
        .issues
        .iter()
        .filter(|i| i.severity == IssueSeverity::Error)
        .collect();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].reason, "package.json has no \"version\" string");
}

/// A zero-byte `package.json` parses to an empty object (V5's empty-input
/// edge) and lands on the same collapsed branch as a missing `"version"`
/// key.
#[test]
fn package_json_empty_fixture_yields_the_same_reason_as_no_version_key() {
    let home = fixture_home("package-json-empty");

    let scan = installations::scan_for(&home, HostPlatform::Windows);

    assert!(!scan
        .installations
        .iter()
        .any(|i| i.client == ClientKind::OpenCode));
    let errors: Vec<_> = scan
        .issues
        .iter()
        .filter(|i| i.severity == IssueSeverity::Error)
        .collect();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].reason, "package.json has no \"version\" string");
}

/// A non-UTF-8 `package.json` fails at `read_to_string`, before
/// `jsonc::parse` ever runs — distinct reason prefix from the parse-failure
/// case in `isolation/`, and zero `Warning` (the slot is present, just
/// broken).
#[test]
fn package_json_unreadable_fixture_yields_one_error_and_zero_warning() {
    let home = fixture_home("package-json-unreadable");

    let scan = installations::scan_for(&home, HostPlatform::Windows);

    assert!(
        !scan
            .issues
            .iter()
            .any(|i| i.severity == IssueSeverity::Warning),
        "a present-but-broken slot must never read as absent"
    );
    let errors: Vec<_> = scan
        .issues
        .iter()
        .filter(|i| i.severity == IssueSeverity::Error)
        .collect();
    assert_eq!(errors.len(), 1);
    assert!(errors[0].reason.starts_with("could not read package.json:"));
}

/// Requirement: broken must never be reported as not-detected (task 1.10,
/// NON-NEGOTIABLE) — the npm package directory exists but has no
/// `package.json` inside it.
#[test]
fn npm_dir_no_package_json_fixture_yields_one_error_and_zero_warning() {
    let home = fixture_home("npm-dir-no-package-json");

    let scan = installations::scan_for(&home, HostPlatform::Windows);

    assert!(
        !scan
            .issues
            .iter()
            .any(|i| i.severity == IssueSeverity::Warning),
        "a present-but-broken slot must never read as absent"
    );
    let errors: Vec<_> = scan
        .issues
        .iter()
        .filter(|i| i.severity == IssueSeverity::Error)
        .collect();
    assert_eq!(errors.len(), 1);
    assert!(errors[0].reason.starts_with("could not read package.json:"));
}

/// Requirement: Each Desktop Version Directory Is Its Own Installation — a
/// desktop directory present with zero versioned subdirectories.
#[test]
fn desktop_empty_fixture_yields_no_installation_and_one_error_never_a_phantom() {
    let home = fixture_home("desktop-empty");

    let scan = installations::scan_for(&home, HostPlatform::Windows);

    assert!(!scan
        .installations
        .iter()
        .any(|i| i.client == ClientKind::ClaudeCode));
    let desktop_errors: Vec<_> = scan
        .issues
        .iter()
        .filter(|i| {
            i.severity == IssueSeverity::Error
                && i.reason
                    .contains("expected at least one Claude Code desktop version directory")
        })
        .collect();
    assert_eq!(desktop_errors.len(), 1);
}

/// Requirement: Each Desktop Version Directory Is Its Own Installation, N
/// candidates yield N installations, never merged, never an anomaly (CA-7
/// pin, task 1.11, §6).
#[test]
fn desktop_two_versions_fixture_yields_two_installations_never_merged() {
    let home = fixture_home("desktop-two-versions");

    let scan = installations::scan_for(&home, HostPlatform::Windows);

    let desktop_installs: Vec<&ClientInstallation> = scan
        .installations
        .iter()
        .filter(|i| i.version == "1.0.0" || i.version == "2.0.0")
        .collect();
    assert_eq!(desktop_installs.len(), 2);
    assert!(desktop_installs
        .iter()
        .all(|i| i.client == ClientKind::ClaudeCode));

    let mut paths: Vec<&std::path::Path> =
        desktop_installs.iter().map(|i| i.path.as_path()).collect();
    paths.sort();
    paths.dedup();
    assert_eq!(paths.len(), 2, "distinct paths for the two installations");

    assert_eq!(
        scan.issues.len(),
        0,
        "N >= 1 candidates is never an anomaly"
    );
}

/// Happy-path fixture mirroring the verified reference machine (design §0).
#[test]
fn reference_fixture_yields_four_installations_zero_issues() {
    let home = fixture_home("reference");

    let scan = installations::scan_for(&home, HostPlatform::Windows);

    assert_eq!(scan.installations.len(), 4);
    assert_eq!(scan.issues.len(), 0);

    let claude_code_count = scan
        .installations
        .iter()
        .filter(|i| i.client == ClientKind::ClaudeCode)
        .count();
    assert_eq!(claude_code_count, 3, "one npm plus two desktop versions");
    let opencode_count = scan
        .installations
        .iter()
        .filter(|i| i.client == ClientKind::OpenCode)
        .count();
    assert_eq!(opencode_count, 1);
}

/// Platform seam: `HostPlatform::Unsupported` yields exactly one `Warning`
/// with `path: None`, never three false "not detected" warnings (design
/// §5.2).
#[test]
fn unsupported_platform_yields_one_warning_with_no_path() {
    let home = fixture_home("reference");

    let scan = installations::scan_for(&home, HostPlatform::Unsupported);

    assert_eq!(scan.installations.len(), 0);
    assert_eq!(scan.issues.len(), 1);
    assert_eq!(scan.issues[0].severity, IssueSeverity::Warning);
    assert_eq!(scan.issues[0].path, None);
}

/// Entry-point dispatch: `scan(home)` matches `scan_for(home, ...)` for the
/// compiled target (design §5.2's payoff, verified on all three CI legs).
#[test]
fn scan_dispatches_to_the_compiled_target_platform() {
    let home = fixture_home("reference");

    let dispatched = installations::scan(&home);

    if cfg!(target_os = "windows") {
        let windows = installations::scan_for(&home, HostPlatform::Windows);
        assert_eq!(dispatched, windows);
    } else {
        let unsupported = installations::scan_for(&home, HostPlatform::Unsupported);
        assert_eq!(dispatched, unsupported);
    }
}

/// Determinism: two consecutive scans over the same fixture home yield
/// byte-identical vectors.
#[test]
fn two_runs_over_reference_and_desktop_two_versions_are_byte_identical() {
    for case in ["reference", "desktop-two-versions"] {
        let home = fixture_home(case);

        let first = installations::scan_for(&home, HostPlatform::Windows);
        let second = installations::scan_for(&home, HostPlatform::Windows);

        assert_eq!(first, second, "case {case} must be deterministic");
    }
}

/// Contract: no `ClientInstallation` ever carries an empty `version`.
#[test]
fn no_installation_ever_carries_an_empty_version() {
    for case in [
        "two-claude",
        "opencode-npm",
        "isolation",
        "desktop-two-versions",
        "reference",
    ] {
        let scan = installations::scan_for(&fixture_home(case), HostPlatform::Windows);
        assert!(
            scan.installations.iter().all(|i| !i.version.is_empty()),
            "case {case} must never carry an empty version"
        );
    }
}

/// Read-only (CA-16): a full scan over `reference/` leaves the fixture tree
/// byte-for-byte unchanged.
#[test]
fn full_scan_leaves_the_reference_fixture_tree_unchanged() {
    let home = fixture_home("reference");

    let before = fixture_tree_bytes(&home);
    let _ = installations::scan_for(&home, HostPlatform::Windows);
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

/// Tripwire (task 1.13, T4D §8 precedent): `desktop-empty/`'s
/// `Claude/claude-code/` directory exists on disk before any scanner code
/// runs. Losing this `.gitkeep` would silently turn "empty desktop
/// directory" into "desktop absent" (`Error` -> `Warning`).
#[test]
fn desktop_empty_fixture_directory_still_exists_on_disk() {
    let mut path = fixture_home("desktop-empty");
    path.push("AppData");
    path.push("Roaming");
    path.push("Claude");
    path.push("claude-code");

    let metadata =
        std::fs::metadata(&path).expect("desktop-empty fixture directory must exist on disk");

    assert!(metadata.is_dir());
}
