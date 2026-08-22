//! Fixture-driven behaviour tests for `vertice_core::installations::{scan,
//! scan_for}`, over the synthetic-home fixture tree committed under
//! `crates/vertice-core/tests/fixtures/client-installations/`. One test (or
//! tight group) per `client-installation-detector` spec requirement;
//! `openspec/changes/fix-windows-claude-desktop-probe/design.md` is the
//! authority for every asserted `severity`/`reason` shape.
//!
//! `packaged_and_legacy_yields_four_never_merged_claude_installs` (CA-7 pin)
//! is written FIRST, before any other test in this file: it must exist and
//! FAIL before the slot-grouped resolver is implemented. Confirmed by
//! running this test file against the pre-resolver `installations.rs`.

use std::path::PathBuf;

use vertice_core::installations::{self, HostPlatform};
use vertice_core::model::{ClientInstallation, ClientKind, IssueSeverity};

/// Build a path under
/// `crates/vertice-core/tests/fixtures/client-installations/<case>/` from
/// per-segment pushes — never a `"/"`-joined literal, so it stays
/// separator-correct on Windows (`tests/skill_scanner.rs:23-30`'s pattern).
fn fixture_home(case: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("fixtures");
    path.push("client-installations");
    path.push(case);
    path
}

/// Requirement: Claude Code npm And Bundled Are Never Merged (CA-7) +
/// One MSIX package and the legacy path both present, both counted.
/// **Primary safeguard for this change** — written first, before any other
/// test in this file.
#[test]
fn packaged_and_legacy_yields_four_never_merged_claude_installs() {
    let home = fixture_home("packaged-and-legacy");

    let scan = installations::scan_for(&home, HostPlatform::Windows);

    let claude_code: Vec<&ClientInstallation> = scan
        .installations
        .iter()
        .filter(|i| i.client == ClientKind::ClaudeCode)
        .collect();
    assert_eq!(
        claude_code.len(),
        4,
        "npm(1) + legacy(1) + packaged(2 versions) = 4, never merged"
    );

    let mut versions: Vec<&str> = claude_code.iter().map(|i| i.version.as_str()).collect();
    versions.sort();
    assert_eq!(versions, vec!["1.0.0", "2.0.0", "3.0.0", "3.1.0"]);

    let mut paths: Vec<&std::path::Path> = claude_code.iter().map(|i| i.path.as_path()).collect();
    paths.sort();
    paths.dedup();
    assert_eq!(paths.len(), 4, "all four installations have distinct paths");

    let opencode_count = scan
        .installations
        .iter()
        .filter(|i| i.client == ClientKind::OpenCode)
        .count();
    assert_eq!(opencode_count, 1);

    assert_eq!(scan.issues.len(), 0, "the fixture is fully healthy");
}

/// Requirement: An Absent Slot Is Reported As An Explicit "Not Detected"
/// Signal (CA-11 pin) — every slot is absent, no `Error`.
#[test]
fn nothing_yields_zero_installs_three_warnings_zero_errors() {
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

    let reasons: std::collections::BTreeSet<&str> =
        scan.issues.iter().map(|i| i.reason.as_str()).collect();
    assert_eq!(
        reasons,
        std::collections::BTreeSet::from([
            "Claude Code CLI (npm) not detected",
            "Claude Code (bundled in Claude Desktop) not detected",
            "OpenCode (npm) not detected",
        ])
    );
}

/// Requirement: npm and bundled installs with different versions never
/// merge — a single MSIX package present, no legacy path.
#[test]
fn packaged_fixture_yields_npm_and_packaged_claude_installs_never_merged() {
    let home = fixture_home("packaged");

    let scan = installations::scan_for(&home, HostPlatform::Windows);

    let claude_code: Vec<&ClientInstallation> = scan
        .installations
        .iter()
        .filter(|i| i.client == ClientKind::ClaudeCode)
        .collect();
    assert_eq!(claude_code.len(), 2, "npm(1) + packaged(1), never merged");
    let mut versions: Vec<&str> = claude_code.iter().map(|i| i.version.as_str()).collect();
    versions.sort();
    assert_eq!(versions, vec!["1.5.0", "5.0.0"]);

    assert_eq!(scan.issues.len(), 0);
}

/// Requirement: a legacy (non-packaged) install still resolves when no
/// `Packages` directory exists at all.
#[test]
fn legacy_fixture_yields_npm_and_legacy_claude_installs_never_merged() {
    let home = fixture_home("legacy");

    let scan = installations::scan_for(&home, HostPlatform::Windows);

    let claude_code: Vec<&ClientInstallation> = scan
        .installations
        .iter()
        .filter(|i| i.client == ClientKind::ClaudeCode)
        .collect();
    assert_eq!(claude_code.len(), 2, "npm(1) + legacy(1), never merged");
    let mut versions: Vec<&str> = claude_code.iter().map(|i| i.version.as_str()).collect();
    versions.sort();
    assert_eq!(versions, vec!["1.1.0", "1.3.0"]);

    assert_eq!(scan.issues.len(), 0);
}

/// Requirement: Multiple packages, and a package missing `claude-code`, each
/// isolated — two valid packages contribute one installation each, the
/// third (payload-less) package contributes nothing and no issue.
#[test]
fn two_packages_fixture_yields_two_installations_third_package_contributes_nothing() {
    let home = fixture_home("two-packages");

    let scan = installations::scan_for(&home, HostPlatform::Windows);

    let bundled: Vec<&ClientInstallation> = scan
        .installations
        .iter()
        .filter(|i| i.client == ClientKind::ClaudeCode)
        .collect();
    assert_eq!(bundled.len(), 2);
    let mut versions: Vec<&str> = bundled.iter().map(|i| i.version.as_str()).collect();
    versions.sort();
    assert_eq!(versions, vec!["10.0.0", "11.0.0"]);

    // Raw, UNSORTED order: pins the byte-wise package-name ordering
    // (`Claude_pkg1` < `Claude_pkg2`) all the way through the real
    // `read_dir` enumeration, not just at the sort-helper unit test.
    let raw_versions: Vec<&str> = scan
        .installations
        .iter()
        .map(|i| i.version.as_str())
        .collect();
    assert_eq!(
        raw_versions,
        vec!["10.0.0", "11.0.0"],
        "pkg1's install must be enumerated before pkg2's, unsorted"
    );

    let bundled_issues: Vec<_> = scan
        .issues
        .iter()
        .filter(|i| i.reason.contains("Claude Code (bundled in Claude Desktop)"))
        .collect();
    assert!(
        bundled_issues.is_empty(),
        "a Claude_* package with no claude-code directory contributes no issue of its own"
    );
}

/// Requirement: an existing-but-empty candidate root is an `Error`, never
/// the "not detected" `Warning`.
#[test]
fn packaged_empty_fixture_yields_one_error_never_a_not_detected_warning() {
    let home = fixture_home("packaged-empty");

    let scan = installations::scan_for(&home, HostPlatform::Windows);

    assert!(!scan
        .installations
        .iter()
        .any(|i| i.client == ClientKind::ClaudeCode));

    let bundled_issues: Vec<_> = scan
        .issues
        .iter()
        .filter(|i| i.reason.contains("Claude Code (bundled in Claude Desktop)"))
        .collect();
    assert_eq!(bundled_issues.len(), 1);
    assert_eq!(bundled_issues[0].severity, IssueSeverity::Error);
    assert!(bundled_issues[0].reason.contains("expected at least one"));
    assert!(
        !bundled_issues[0].reason.ends_with("not detected"),
        "a broken candidate root must never read as absent"
    );
}

/// Requirement: a `Claude_*` package with no `claude-code` directory inside
/// is not a candidate root at all — with no legacy path either, the slot
/// still reports exactly one "not detected" `Warning`.
#[test]
fn non_claude_packages_fixture_contributes_nothing_and_warns_not_detected() {
    let home = fixture_home("non-claude-packages");

    let scan = installations::scan_for(&home, HostPlatform::Windows);

    assert!(!scan
        .installations
        .iter()
        .any(|i| i.client == ClientKind::ClaudeCode));

    let bundled_issues: Vec<_> = scan
        .issues
        .iter()
        .filter(|i| i.reason.contains("Claude Code (bundled in Claude Desktop)"))
        .collect();
    assert_eq!(bundled_issues.len(), 1);
    assert_eq!(bundled_issues[0].severity, IssueSeverity::Warning);
    assert_eq!(
        bundled_issues[0].reason,
        "Claude Code (bundled in Claude Desktop) not detected"
    );
}

/// Requirement: an unreadable `Packages` directory errors but does not
/// block the legacy fallback.
#[test]
fn packages_unreadable_fixture_errors_but_legacy_still_resolves() {
    let home = fixture_home("packages-unreadable");

    let scan = installations::scan_for(&home, HostPlatform::Windows);

    let bundled: Vec<&ClientInstallation> = scan
        .installations
        .iter()
        .filter(|i| i.client == ClientKind::ClaudeCode)
        .collect();
    assert_eq!(bundled.len(), 1, "the legacy install is still reported");
    assert_eq!(bundled[0].version, "6.0.0");

    let packages_errors: Vec<_> = scan
        .issues
        .iter()
        .filter(|i| {
            i.severity == IssueSeverity::Error
                && i.path.as_ref().is_some_and(|p| p.ends_with("Packages"))
        })
        .collect();
    assert_eq!(packages_errors.len(), 1);
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

/// Requirement: Each slot fails independently (NON-NEGOTIABLE).
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
/// missing-key case.
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

/// A zero-byte `package.json` parses to an empty object and lands on the
/// same collapsed branch as a missing `"version"` key.
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
            .any(|i| i.severity == IssueSeverity::Warning && i.reason.starts_with("OpenCode")),
        "a present-but-broken OpenCode slot must never read as absent"
    );
    let errors: Vec<_> = scan
        .issues
        .iter()
        .filter(|i| i.severity == IssueSeverity::Error)
        .collect();
    assert_eq!(errors.len(), 1);
    assert!(errors[0].reason.starts_with("could not read package.json:"));
}

/// Requirement: broken must never be reported as not-detected — the npm
/// package directory exists but has no `package.json` inside it.
#[test]
fn npm_dir_no_package_json_fixture_yields_one_error_and_zero_warning() {
    let home = fixture_home("npm-dir-no-package-json");

    let scan = installations::scan_for(&home, HostPlatform::Windows);

    assert!(
        !scan.issues.iter().any(
            |i| i.severity == IssueSeverity::Warning && i.reason.starts_with("Claude Code CLI")
        ),
        "a present-but-broken npm slot must never read as absent"
    );
    let errors: Vec<_> = scan
        .issues
        .iter()
        .filter(|i| i.severity == IssueSeverity::Error)
        .collect();
    assert_eq!(errors.len(), 1);
    assert!(errors[0].reason.starts_with("could not read package.json:"));
}

/// Platform seam: `HostPlatform::Unsupported` yields exactly one `Warning`
/// with `path: None`, never three false "not detected" warnings.
#[test]
fn unsupported_platform_yields_one_warning_with_no_path() {
    let home = fixture_home("packaged-and-legacy");

    let scan = installations::scan_for(&home, HostPlatform::Unsupported);

    assert_eq!(scan.installations.len(), 0);
    assert_eq!(scan.issues.len(), 1);
    assert_eq!(scan.issues[0].severity, IssueSeverity::Warning);
    assert_eq!(scan.issues[0].path, None);
}

/// Entry-point dispatch: `scan(home)` matches `scan_for(home, ...)` for the
/// compiled target (verified on all three CI legs).
#[test]
fn scan_dispatches_to_the_compiled_target_platform() {
    let home = fixture_home("packaged-and-legacy");

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
fn two_runs_over_the_same_fixture_are_byte_identical() {
    for case in ["packaged-and-legacy", "two-packages"] {
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
        "packaged-and-legacy",
        "packaged",
        "legacy",
        "two-packages",
        "opencode-npm",
        "isolation",
    ] {
        let scan = installations::scan_for(&fixture_home(case), HostPlatform::Windows);
        assert!(
            scan.installations.iter().all(|i| !i.version.is_empty()),
            "case {case} must never carry an empty version"
        );
    }
}

/// Read-only (CA-16): a full scan over `packaged-and-legacy/` leaves the
/// fixture tree byte-for-byte unchanged.
#[test]
fn full_scan_leaves_the_fixture_tree_unchanged() {
    let home = fixture_home("packaged-and-legacy");

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
