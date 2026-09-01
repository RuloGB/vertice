//! Fixture-driven behaviour tests for `vertice_core::installations::{scan,
//! scan_for}`, over the synthetic-home fixture tree committed under
//! `crates/vertice-core/tests/fixtures/client-installations/`. One test (or
//! tight group) per `client-installation-detector` spec requirement.
//! `openspec/changes/report-client-presence-as-status/design.md` §7 is the
//! authority for every asserted `ClientPresence`/`ScanIssue` shape.
//!
//! Rewritten for the presence-record contract: absence is asserted on
//! `ClientPresence.status`, never on a parsed `ScanIssue.reason` string.

use std::path::PathBuf;

use vertice_core::installations::{self, HostPlatform};
use vertice_core::model::{
    ClientInstallSlot, ClientInstallation, ClientKind, ClientPresence, ClientPresenceStatus,
    IssueSeverity,
};

/// Build a path under
/// `crates/vertice-core/tests/fixtures/client-installations/opencode-desktop/<case>/`.
fn opencode_desktop_fixture_home(case: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("fixtures");
    path.push("client-installations");
    path.push("opencode-desktop");
    path.push(case);
    path
}

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

fn presence_for<'a>(records: &'a [ClientPresence], label_contains: &str) -> &'a ClientPresence {
    records
        .iter()
        .find(|record| record.label.contains(label_contains))
        .unwrap_or_else(|| panic!("no presence record with label containing {label_contains:?}"))
}

/// CA-11 pin: a machine with no clients yields five `NotDetected` records
/// and zero issues.
#[test]
fn nothing_yields_five_not_detected_records_and_zero_issues() {
    let home = fixture_home("nothing");

    let scan = installations::scan_for(&home, HostPlatform::Windows);

    let records = scan
        .presence
        .as_ref()
        .expect("Windows always has a probe table");
    assert_eq!(records.len(), 5, "one record per defined slot");
    for record in records {
        assert_eq!(record.status, ClientPresenceStatus::NotDetected);
        assert!(record.installations.is_empty());
        assert!(!record.probed_paths.is_empty());
    }
    let slots: Vec<ClientInstallSlot> = records.iter().map(|r| r.slot).collect();
    assert_eq!(
        slots,
        vec![
            ClientInstallSlot::ClaudeCodeNpm,
            ClientInstallSlot::ClaudeCodeBundled,
            ClientInstallSlot::OpenCodeNpm,
            ClientInstallSlot::OpenCodeDesktop,
            ClientInstallSlot::CodexStandalone,
        ],
        "one record per slot, in probe-table order"
    );
    assert_eq!(scan.installations.len(), 0);
    assert_eq!(scan.issues.len(), 0, "absence is never an issue");
}

/// CA-7 pin: the bundled slot's record carries every coexisting
/// installation in one row, never merged or reduced.
#[test]
fn bundled_slot_record_carries_every_coexisting_installation() {
    let home = fixture_home("packaged-and-legacy");

    let scan = installations::scan_for(&home, HostPlatform::Windows);

    let records = scan.presence.as_ref().expect("Windows has a probe table");
    let bundled = presence_for(records, "bundled in Claude Desktop");

    assert_eq!(bundled.slot, ClientInstallSlot::ClaudeCodeBundled);
    assert_eq!(bundled.status, ClientPresenceStatus::Detected);
    assert_eq!(
        bundled.installations.len(),
        3,
        "legacy(1) + packaged(2 versions) = 3, all in the one bundled-slot record, never merged"
    );

    let mut versions: Vec<&str> = bundled
        .installations
        .iter()
        .map(|i| i.version.as_str())
        .collect();
    versions.sort();
    assert_eq!(versions, vec!["2.0.0", "3.0.0", "3.1.0"]);
}

/// `ScanReport.installations` equals the concatenation of every presence
/// record's `installations`, in record order — the flattening invariant
/// (design §3's tripwire), checked on two independent fixtures.
#[test]
fn flattened_presence_installations_equal_report_installations() {
    for case in ["packaged-and-legacy", "isolation"] {
        let home = fixture_home(case);
        let scan = installations::scan_for(&home, HostPlatform::Windows);

        let records = scan.presence.as_ref().expect("Windows has a probe table");
        let expected: Vec<&ClientInstallation> = records
            .iter()
            .flat_map(|record| record.installations.iter())
            .collect();
        let actual: Vec<&ClientInstallation> = scan.installations.iter().collect();

        assert_eq!(
            actual, expected,
            "case {case}: ScanReport.installations must equal the flattened records"
        );
    }
}

/// Full four-installation pin, kept from the original suite: npm(1) +
/// legacy(1) + packaged(2 versions), never merged.
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

    let records = scan.presence.as_ref().expect("Windows has a probe table");
    assert_eq!(records.len(), 5, "one record per defined slot");
    let opencode = presence_for(records, "OpenCode");
    assert_eq!(opencode.slot, ClientInstallSlot::OpenCodeNpm);
    assert_eq!(opencode.status, ClientPresenceStatus::Detected);
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

/// Requirement: multiple packages, and a package missing `claude-code`,
/// each isolated — two valid packages contribute one installation each, the
/// third (payload-less) package contributes nothing and does not distort
/// the record.
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

    assert_eq!(
        scan.issues.len(),
        0,
        "the payload-less package is not an issue"
    );

    let records = scan.presence.as_ref().expect("Windows has a probe table");
    let record = presence_for(records, "bundled in Claude Desktop");
    assert_eq!(record.slot, ClientInstallSlot::ClaudeCodeBundled);
    assert_eq!(record.status, ClientPresenceStatus::Detected);
    assert_eq!(record.installations.len(), 2);
}

/// Requirement: a candidate root that exists but yields nothing is
/// `Detected`, not `NotDetected` — an existing-but-empty bundled candidate
/// root is an `Error`.
#[test]
fn packaged_empty_fixture_is_detected_with_zero_installations_and_one_error() {
    let home = fixture_home("packaged-empty");

    let scan = installations::scan_for(&home, HostPlatform::Windows);

    assert!(!scan
        .installations
        .iter()
        .any(|i| i.client == ClientKind::ClaudeCode));

    let records = scan.presence.as_ref().expect("Windows has a probe table");
    let bundled = presence_for(records, "bundled in Claude Desktop");
    assert_eq!(bundled.slot, ClientInstallSlot::ClaudeCodeBundled);
    assert_eq!(
        bundled.status,
        ClientPresenceStatus::Detected,
        "a broken candidate root must never read as absent"
    );
    assert!(bundled.installations.is_empty());

    let bundled_errors: Vec<_> = scan
        .issues
        .iter()
        .filter(|i| i.reason.contains("Claude Code (bundled in Claude Desktop)"))
        .collect();
    assert_eq!(bundled_errors.len(), 1);
    assert_eq!(bundled_errors[0].severity, IssueSeverity::Error);
    assert!(bundled_errors[0].reason.contains("expected at least one"));
}

/// Requirement: a `Claude_*` package with no `claude-code` directory inside
/// is not a candidate root at all — with no legacy path either, the slot
/// still reports `status: NotDetected`, with zero issues.
#[test]
fn non_claude_packages_fixture_is_not_detected_with_zero_issues() {
    let home = fixture_home("non-claude-packages");

    let scan = installations::scan_for(&home, HostPlatform::Windows);

    assert!(!scan
        .installations
        .iter()
        .any(|i| i.client == ClientKind::ClaudeCode));

    let records = scan.presence.as_ref().expect("Windows has a probe table");
    let bundled = presence_for(records, "bundled in Claude Desktop");
    assert_eq!(bundled.slot, ClientInstallSlot::ClaudeCodeBundled);
    assert_eq!(bundled.status, ClientPresenceStatus::NotDetected);
    assert!(bundled.installations.is_empty());

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

/// Requirement: an unreadable `Packages` directory errors but does not
/// block the legacy fallback, which still resolves inside the same record.
#[test]
fn packages_unreadable_fixture_errors_but_legacy_still_resolves_in_the_same_record() {
    let home = fixture_home("packages-unreadable");

    let scan = installations::scan_for(&home, HostPlatform::Windows);

    let bundled: Vec<&ClientInstallation> = scan
        .installations
        .iter()
        .filter(|i| i.client == ClientKind::ClaudeCode)
        .collect();
    assert_eq!(bundled.len(), 1, "the legacy install is still reported");
    assert_eq!(bundled[0].version, "6.0.0");

    let records = scan.presence.as_ref().expect("Windows has a probe table");
    let record = presence_for(records, "bundled in Claude Desktop");
    assert_eq!(record.slot, ClientInstallSlot::ClaudeCodeBundled);
    assert_eq!(record.status, ClientPresenceStatus::Detected);
    assert_eq!(
        record.installations.len(),
        1,
        "enumeration Error and the legacy candidate resolve in the same record"
    );

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

/// Requirement: a present OpenCode npm install is reported as `Detected`.
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

    let records = scan.presence.as_ref().expect("Windows has a probe table");
    let record = presence_for(records, "OpenCode");
    assert_eq!(record.slot, ClientInstallSlot::OpenCodeNpm);
    assert_eq!(record.status, ClientPresenceStatus::Detected);
}

/// Requirement: each slot fails independently (NON-NEGOTIABLE) — one broken
/// slot never changes another slot's status.
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

    let records = scan.presence.as_ref().expect("Windows has a probe table");
    assert_eq!(records.len(), 5, "one record per defined slot");

    // The three slots that PRE-DATE this fixture each have an existing
    // candidate root, broken or not, so all three are `Detected`.
    let pre_existing = [
        ClientInstallSlot::ClaudeCodeNpm,
        ClientInstallSlot::ClaudeCodeBundled,
        ClientInstallSlot::OpenCodeNpm,
    ];
    for slot in pre_existing {
        let record = records
            .iter()
            .find(|record| record.slot == slot)
            .unwrap_or_else(|| panic!("record for {slot:?} must be present"));
        assert_eq!(
            record.status,
            ClientPresenceStatus::Detected,
            "every one of the three pre-existing slots has an existing candidate root, broken or not"
        );
    }

    // `isolation` gains no desktop install (design task 8.3, verified by
    // reading the fixture, not assumption): the count of distinct
    // installation paths below MUST stay unchanged at 4.
    let desktop = presence_for(records, "desktop app");
    assert_eq!(desktop.slot, ClientInstallSlot::OpenCodeDesktop);
    assert_eq!(
        desktop.status,
        ClientPresenceStatus::NotDetected,
        "this fixture predates the OpenCode desktop slot and has no @opencode-aidesktop tree"
    );

    let codex = presence_for(records, "Codex");
    assert_eq!(codex.slot, ClientInstallSlot::CodexStandalone);
    assert_eq!(
        codex.status,
        ClientPresenceStatus::NotDetected,
        "this fixture predates Codex support and has no .codex tree"
    );
}

/// Requirement: a malformed or unreadable `package.json` produces an
/// `Error`, never a phantom installation — `Detected` with zero
/// installations — a `package.json` with no `"version"` key.
#[test]
fn no_version_key_fixture_is_detected_with_zero_installations_and_one_error() {
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

    let records = scan.presence.as_ref().expect("Windows has a probe table");
    let opencode = presence_for(records, "OpenCode");
    assert_eq!(opencode.slot, ClientInstallSlot::OpenCodeNpm);
    assert_eq!(opencode.status, ClientPresenceStatus::Detected);
    assert!(opencode.installations.is_empty());
}

/// `"version"` present but not a string — same collapsed reason as the
/// missing-key case, same `Detected` + zero installations shape.
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

    let records = scan.presence.as_ref().expect("Windows has a probe table");
    let opencode = presence_for(records, "OpenCode");
    assert_eq!(opencode.slot, ClientInstallSlot::OpenCodeNpm);
    assert_eq!(opencode.status, ClientPresenceStatus::Detected);
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

    let records = scan.presence.as_ref().expect("Windows has a probe table");
    let opencode = presence_for(records, "OpenCode");
    assert_eq!(opencode.slot, ClientInstallSlot::OpenCodeNpm);
    assert_eq!(opencode.status, ClientPresenceStatus::Detected);
}

/// A non-UTF-8 `package.json` fails at `read_to_string`, before
/// `jsonc::parse` ever runs — distinct reason prefix from the parse-failure
/// case in `isolation/`, and the slot still reads as `Detected`, never
/// `NotDetected` (the slot is present, just broken).
#[test]
fn package_json_unreadable_fixture_is_detected_never_not_detected() {
    let home = fixture_home("package-json-unreadable");

    let scan = installations::scan_for(&home, HostPlatform::Windows);

    let records = scan.presence.as_ref().expect("Windows has a probe table");
    let opencode = presence_for(records, "OpenCode");
    assert_eq!(opencode.slot, ClientInstallSlot::OpenCodeNpm);
    assert_eq!(
        opencode.status,
        ClientPresenceStatus::Detected,
        "a present-but-broken OpenCode slot must never read as absent"
    );
    assert!(opencode.installations.is_empty());

    let errors: Vec<_> = scan
        .issues
        .iter()
        .filter(|i| i.severity == IssueSeverity::Error)
        .collect();
    assert_eq!(errors.len(), 1);
    assert!(errors[0].reason.starts_with("could not read package.json:"));
}

/// Requirement: broken must never be reported as not-detected — the
/// OpenCode npm directory exists but has no `package.json` inside it.
#[test]
fn npm_dir_no_package_json_fixture_is_detected_never_not_detected() {
    let home = fixture_home("npm-dir-no-package-json");

    let scan = installations::scan_for(&home, HostPlatform::Windows);

    let records = scan.presence.as_ref().expect("Windows has a probe table");
    let opencode = presence_for(records, "OpenCode");
    assert_eq!(opencode.slot, ClientInstallSlot::OpenCodeNpm);
    assert_eq!(
        opencode.status,
        ClientPresenceStatus::Detected,
        "a present-but-broken npm slot must never read as absent"
    );
    assert!(opencode.installations.is_empty());

    let errors: Vec<_> = scan
        .issues
        .iter()
        .filter(|i| i.severity == IssueSeverity::Error)
        .collect();
    assert_eq!(errors.len(), 1);
    assert!(errors[0].reason.starts_with("could not read package.json:"));
}

/// Platform seam: `HostPlatform::Unsupported` yields `client_presence: None`
/// (never three fabricated `NotDetected` records, never `Some(vec![])`),
/// exactly one `Warning` with `path: None`, byte-identical to before this
/// change (design §4 tripwire).
#[test]
fn unsupported_platform_yields_none_presence_and_one_warning_with_no_path() {
    let home = fixture_home("packaged-and-legacy");

    let scan = installations::scan_for(&home, HostPlatform::Unsupported);

    assert_eq!(scan.installations.len(), 0);
    assert_eq!(scan.issues.len(), 1);
    assert_eq!(scan.issues[0].severity, IssueSeverity::Warning);
    assert_eq!(scan.issues[0].path, None);
    assert!(
        scan.presence.is_none(),
        "an unsupported platform has no probe table at all"
    );
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
/// byte-identical vectors, including `presence`.
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

/// Read-only (CA-16): a full scan over every new `opencode-desktop` fixture
/// home — including `happy`, which is the only case that actually opens and
/// reads the committed `app.asar` — leaves every one of those fixture trees
/// byte-for-byte unchanged.
#[test]
fn full_scan_leaves_every_opencode_desktop_fixture_tree_unchanged() {
    let cases = [
        "happy",
        "no-asar",
        "oversized-header",
        "bad-prefix",
        "tiny-header-len",
        "truncated",
        "malformed-header",
        "no-package-json-entry",
        "nested-package-json-only",
        "entry-out-of-range",
        "shifted-payload",
        "no-name-key",
        "no-version-key",
        "empty-version",
    ];

    for case in cases {
        let home = opencode_desktop_fixture_home(case);
        let before = fixture_tree_bytes(&home);
        let _ = installations::scan_for(&home, HostPlatform::Windows);
        let after = fixture_tree_bytes(&home);

        assert_eq!(before, after, "case {case}: fixture tree must be untouched");
    }
}

// -- Codex CLI standalone slot (design §3, §10.3) --

/// Build a path under
/// `crates/vertice-core/tests/fixtures/client-installations/codex-installations/<case>/`
/// from per-segment pushes.
fn codex_fixture_home(case: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("fixtures");
    path.push("client-installations");
    path.push("codex-installations");
    path.push(case);
    path
}

/// Tripwire, mirroring `skill_scanner.rs:36-52`: git cannot track an empty
/// directory, so every release directory's presence depends on its
/// `.gitkeep`. Asserted independent of any scanner code.
#[test]
fn codex_installation_fixture_release_directories_exist_on_disk() {
    let cases: &[(&str, &[&str])] = &[
        (
            "single-release",
            &[
                ".codex",
                "packages",
                "standalone",
                "releases",
                "0.149.0-x86_64-pc-windows-msvc",
            ],
        ),
        (
            "two-releases",
            &[
                ".codex",
                "packages",
                "standalone",
                "releases",
                "0.148.0-x86_64-pc-windows-msvc",
            ],
        ),
        (
            "prerelease",
            &[
                ".codex",
                "packages",
                "standalone",
                "releases",
                "0.150.0-rc.1-x86_64-pc-windows-msvc",
            ],
        ),
        (
            "unknown-triple",
            &[
                ".codex",
                "packages",
                "standalone",
                "releases",
                "0.151.0-riscv64-unknown-linux-gnu",
            ],
        ),
        (
            "empty-releases",
            &[".codex", "packages", "standalone", "releases"],
        ),
    ];

    for (case, segments) in cases {
        let mut path = codex_fixture_home(case);
        for segment in *segments {
            path.push(segment);
        }
        let metadata = std::fs::metadata(&path)
            .unwrap_or_else(|err| panic!("fixture directory must exist for {case}: {err}"));
        assert!(
            metadata.is_dir(),
            "fixture path for {case} must be a directory"
        );
    }
}

/// `codex-installations/single-release`: `Detected`, one `ClientInstallation`,
/// version `0.149.0` from the directory name, path = the release directory.
#[test]
fn single_release_yields_one_detected_installation_from_the_directory_name() {
    let home = codex_fixture_home("single-release");

    let scan = installations::scan_for(&home, HostPlatform::Windows);

    let records = scan.presence.as_ref().expect("Windows has a probe table");
    let codex = presence_for(records, "Codex");
    assert_eq!(codex.slot, ClientInstallSlot::CodexStandalone);
    assert_eq!(codex.status, ClientPresenceStatus::Detected);
    assert_eq!(codex.installations.len(), 1);
    assert_eq!(codex.installations[0].client, ClientKind::Codex);
    assert_eq!(codex.installations[0].version, "0.149.0");
    assert!(codex.installations[0]
        .path
        .ends_with("0.149.0-x86_64-pc-windows-msvc"));
    assert!(scan.issues.is_empty());
}

/// CA-7: two release directories yield two unmerged installations.
#[test]
fn two_release_directories_yield_two_unmerged_installations() {
    let home = codex_fixture_home("two-releases");

    let scan = installations::scan_for(&home, HostPlatform::Windows);

    let records = scan.presence.as_ref().expect("Windows has a probe table");
    let codex = presence_for(records, "Codex");
    assert_eq!(codex.slot, ClientInstallSlot::CodexStandalone);
    assert_eq!(codex.status, ClientPresenceStatus::Detected);
    assert_eq!(
        codex.installations.len(),
        2,
        "never merged, never reduced to a winner"
    );

    let mut versions: Vec<&str> = codex
        .installations
        .iter()
        .map(|i| i.version.as_str())
        .collect();
    versions.sort();
    assert_eq!(versions, vec!["0.148.0", "0.149.0"]);
    assert!(scan.issues.is_empty());
}

/// `0.150.0-rc.1-x86_64-pc-windows-msvc` yields `0.150.0-rc.1`. The RED test
/// that kills "split on the first `-`".
#[test]
fn prerelease_release_directory_name_yields_the_full_prerelease_version() {
    let home = codex_fixture_home("prerelease");

    let scan = installations::scan_for(&home, HostPlatform::Windows);

    let records = scan.presence.as_ref().expect("Windows has a probe table");
    let codex = presence_for(records, "Codex");
    assert_eq!(codex.slot, ClientInstallSlot::CodexStandalone);
    assert_eq!(codex.status, ClientPresenceStatus::Detected);
    assert_eq!(codex.installations.len(), 1);
    assert_eq!(codex.installations[0].version, "0.150.0-rc.1");
    assert!(scan.issues.is_empty());
}

/// CA-11: no `~/.codex` at all yields `NotDetected` and zero issues.
#[test]
fn home_without_codex_yields_not_detected_and_zero_issues() {
    let home = codex_fixture_home("nothing");

    let scan = installations::scan_for(&home, HostPlatform::Windows);

    let records = scan.presence.as_ref().expect("Windows has a probe table");
    let codex = presence_for(records, "Codex");
    assert_eq!(codex.slot, ClientInstallSlot::CodexStandalone);
    assert_eq!(codex.status, ClientPresenceStatus::NotDetected);
    assert!(codex.installations.is_empty());
    assert!(scan.issues.is_empty());
}

/// An unrecognized target triple yields `Detected` + 0 installations for
/// that directory + 1 `Error` carrying the directory's path (design §3.3).
#[test]
fn unknown_triple_yields_detected_zero_installations_and_one_error_with_its_path() {
    let home = codex_fixture_home("unknown-triple");

    let scan = installations::scan_for(&home, HostPlatform::Windows);

    let records = scan.presence.as_ref().expect("Windows has a probe table");
    let codex = presence_for(records, "Codex");
    assert_eq!(codex.slot, ClientInstallSlot::CodexStandalone);
    assert_eq!(codex.status, ClientPresenceStatus::Detected);
    assert!(codex.installations.is_empty());

    let errors: Vec<_> = scan
        .issues
        .iter()
        .filter(|i| i.severity == IssueSeverity::Error)
        .collect();
    assert_eq!(errors.len(), 1);
    assert!(errors[0]
        .path
        .as_ref()
        .expect("unparseable release directory issue must carry a path")
        .ends_with("0.151.0-riscv64-unknown-linux-gnu"));
}

/// A `releases/` directory that exists but is empty yields `Detected` + 0
/// installations + 1 `Error` (design §3.3, mirrors `packaged-empty`).
#[test]
fn empty_releases_yields_detected_zero_installations_and_one_error() {
    let home = codex_fixture_home("empty-releases");

    let scan = installations::scan_for(&home, HostPlatform::Windows);

    let records = scan.presence.as_ref().expect("Windows has a probe table");
    let codex = presence_for(records, "Codex");
    assert_eq!(codex.slot, ClientInstallSlot::CodexStandalone);
    assert_eq!(codex.status, ClientPresenceStatus::Detected);
    assert!(codex.installations.is_empty());

    let errors: Vec<_> = scan
        .issues
        .iter()
        .filter(|i| i.severity == IssueSeverity::Error)
        .collect();
    assert_eq!(errors.len(), 1);
    assert!(errors[0].reason.contains("expected at least one"));
}

/// `~/.codex/version.json`'s `latest_version` is never read as the version:
/// the reported version equals the release directory name.
#[test]
fn stale_version_json_never_wins_over_the_release_directory_name() {
    let home = codex_fixture_home("stale-version-json");

    let scan = installations::scan_for(&home, HostPlatform::Windows);

    let records = scan.presence.as_ref().expect("Windows has a probe table");
    let codex = presence_for(records, "Codex");
    assert_eq!(codex.slot, ClientInstallSlot::CodexStandalone);
    assert_eq!(codex.installations.len(), 1);
    assert_eq!(codex.installations[0].version, "0.149.0");
    assert_ne!(codex.installations[0].version, "9.9.9");
}

// -- OpenCode desktop slot (`detect-desktop-client-installs` design §4-§6) --

/// CA-11: a root without a readable archive is still `Detected` with zero
/// installations — presence never depends on version extraction.
#[test]
fn opencode_desktop_root_without_a_readable_archive_is_detected_with_no_installations() {
    let home = opencode_desktop_fixture_home("no-asar");

    let scan = installations::scan_for(&home, HostPlatform::Windows);

    let records = scan.presence.as_ref().expect("Windows has a probe table");
    let desktop = presence_for(records, "desktop app");
    assert_eq!(desktop.slot, ClientInstallSlot::OpenCodeDesktop);
    assert_eq!(desktop.status, ClientPresenceStatus::Detected);
    assert!(desktop.installations.is_empty());

    let errors: Vec<_> = scan
        .issues
        .iter()
        .filter(|i| i.reason.contains("OpenCode (desktop app)"))
        .collect();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].severity, IssueSeverity::Error);
}

/// CA-11: no `@opencode-aidesktop` root at all yields `NotDetected` and
/// zero issues for that slot.
#[test]
fn home_without_the_desktop_root_yields_not_detected_and_zero_issues() {
    let home = fixture_home("nothing");

    let scan = installations::scan_for(&home, HostPlatform::Windows);

    let records = scan.presence.as_ref().expect("Windows has a probe table");
    let desktop = presence_for(records, "desktop app");
    assert_eq!(desktop.slot, ClientInstallSlot::OpenCodeDesktop);
    assert_eq!(desktop.status, ClientPresenceStatus::NotDetected);
    assert!(desktop.installations.is_empty());
    assert!(scan
        .issues
        .iter()
        .all(|i| !i.reason.contains("OpenCode (desktop app)")));
}

/// design §5.2's one `Warning` row in the taxonomy: an oversized header
/// degrades with a deliberate skip, never an `Error`.
#[test]
fn oversized_header_degrades_with_a_warning_not_an_error() {
    let home = opencode_desktop_fixture_home("oversized-header");

    let scan = installations::scan_for(&home, HostPlatform::Windows);

    let records = scan.presence.as_ref().expect("Windows has a probe table");
    let desktop = presence_for(records, "desktop app");
    assert_eq!(desktop.status, ClientPresenceStatus::Detected);
    assert!(desktop.installations.is_empty());

    let matching: Vec<_> = scan
        .issues
        .iter()
        .filter(|i| i.reason.contains("OpenCode (desktop app)"))
        .collect();
    assert_eq!(matching.len(), 1);
    assert_eq!(matching[0].severity, IssueSeverity::Warning);
    assert!(matching[0].reason.starts_with("skipped"));
    assert!(matching[0].reason.contains("5000000"));
}

/// One test per remaining §8.2 fixture row: exact `(status, installation
/// count, issue severity)` triple.
#[test]
fn every_remaining_opencode_desktop_fixture_matches_its_taxonomy_row() {
    let cases: &[(&str, ClientPresenceStatus, usize, Option<IssueSeverity>)] = &[
        (
            "bad-prefix",
            ClientPresenceStatus::Detected,
            0,
            Some(IssueSeverity::Error),
        ),
        (
            "tiny-header-len",
            ClientPresenceStatus::Detected,
            0,
            Some(IssueSeverity::Error),
        ),
        (
            "truncated",
            ClientPresenceStatus::Detected,
            0,
            Some(IssueSeverity::Error),
        ),
        (
            "malformed-header",
            ClientPresenceStatus::Detected,
            0,
            Some(IssueSeverity::Error),
        ),
        (
            "no-package-json-entry",
            ClientPresenceStatus::Detected,
            0,
            Some(IssueSeverity::Error),
        ),
        (
            "nested-package-json-only",
            ClientPresenceStatus::Detected,
            0,
            Some(IssueSeverity::Error),
        ),
        (
            "entry-out-of-range",
            ClientPresenceStatus::Detected,
            0,
            Some(IssueSeverity::Error),
        ),
        ("shifted-payload", ClientPresenceStatus::Detected, 1, None),
        (
            "no-name-key",
            ClientPresenceStatus::Detected,
            0,
            Some(IssueSeverity::Error),
        ),
        (
            "no-version-key",
            ClientPresenceStatus::Detected,
            0,
            Some(IssueSeverity::Error),
        ),
        (
            "empty-version",
            ClientPresenceStatus::Detected,
            0,
            Some(IssueSeverity::Error),
        ),
        ("happy", ClientPresenceStatus::Detected, 1, None),
    ];

    for (case, expected_status, expected_installations, expected_severity) in cases {
        let home = opencode_desktop_fixture_home(case);
        let scan = installations::scan_for(&home, HostPlatform::Windows);

        let records = scan.presence.as_ref().expect("Windows has a probe table");
        let desktop = presence_for(records, "desktop app");
        assert_eq!(
            desktop.status, *expected_status,
            "case {case}: status mismatch"
        );
        assert_eq!(
            desktop.installations.len(),
            *expected_installations,
            "case {case}: installation count mismatch"
        );

        let matching: Vec<_> = scan
            .issues
            .iter()
            .filter(|i| i.reason.contains("OpenCode (desktop app)"))
            .collect();
        match expected_severity {
            Some(severity) => {
                assert_eq!(matching.len(), 1, "case {case}: expected exactly one issue");
                assert_eq!(
                    matching[0].severity, *severity,
                    "case {case}: severity mismatch"
                );
            }
            None => {
                assert!(matching.is_empty(), "case {case}: expected zero issues");
            }
        }
    }
}

/// `nested-package-json-only`: the nested `9.9.9` version must appear
/// nowhere in the report (D1 guard, §2.4's rejected-recursion guard).
#[test]
fn nested_package_json_only_never_leaks_its_version_into_the_report() {
    let home = opencode_desktop_fixture_home("nested-package-json-only");

    let scan = installations::scan_for(&home, HostPlatform::Windows);

    assert!(!scan.installations.iter().any(|i| i.version == "9.9.9"));
    let records = scan.presence.as_ref().expect("Windows has a probe table");
    let desktop = presence_for(records, "desktop app");
    assert!(!desktop.installations.iter().any(|i| i.version == "9.9.9"));
}

/// `shifted-payload`: the extracted version is the correct root manifest's,
/// and the neighbouring manifest's version never appears in the report.
#[test]
fn shifted_payload_never_yields_the_neighbouring_manifests_version_in_the_report() {
    let home = opencode_desktop_fixture_home("shifted-payload");

    let scan = installations::scan_for(&home, HostPlatform::Windows);

    let records = scan.presence.as_ref().expect("Windows has a probe table");
    let desktop = presence_for(records, "desktop app");
    assert_eq!(desktop.installations.len(), 1);
    assert_eq!(desktop.installations[0].version, "0.4.2");
    assert!(!scan.installations.iter().any(|i| i.version == "9.9.9"));
}

/// `happy`: `Detected`, one installation, version `0.4.2`, `path` = the
/// install root (never the `.asar` file itself), zero issues.
#[test]
fn happy_fixture_yields_one_detected_opencode_desktop_installation() {
    let home = opencode_desktop_fixture_home("happy");

    let scan = installations::scan_for(&home, HostPlatform::Windows);

    let records = scan.presence.as_ref().expect("Windows has a probe table");
    let desktop = presence_for(records, "desktop app");
    assert_eq!(desktop.slot, ClientInstallSlot::OpenCodeDesktop);
    assert_eq!(desktop.status, ClientPresenceStatus::Detected);
    assert_eq!(desktop.installations.len(), 1);
    assert_eq!(desktop.installations[0].client, ClientKind::OpenCode);
    assert_eq!(desktop.installations[0].version, "0.4.2");
    assert!(desktop.installations[0]
        .path
        .ends_with("@opencode-aidesktop"));
    assert!(!desktop.installations[0]
        .path
        .to_string_lossy()
        .ends_with("app.asar"));
    assert!(scan
        .issues
        .iter()
        .all(|i| !i.reason.contains("OpenCode (desktop app)")));
}

#[test]
fn scan_for_emits_five_records_in_probe_table_order() {
    let home = fixture_home("nothing");

    let scan = installations::scan_for(&home, HostPlatform::Windows);

    let records = scan.presence.as_ref().expect("Windows has a probe table");
    assert_eq!(records.len(), 5, "one record per defined slot");
    let slots: Vec<ClientInstallSlot> = records.iter().map(|r| r.slot).collect();
    assert_eq!(
        slots,
        vec![
            ClientInstallSlot::ClaudeCodeNpm,
            ClientInstallSlot::ClaudeCodeBundled,
            ClientInstallSlot::OpenCodeNpm,
            ClientInstallSlot::OpenCodeDesktop,
            ClientInstallSlot::CodexStandalone,
        ]
    );
}

// -- Slot promotion tripwire (design §2, task 1.7) --

/// Tripwire: promoting the private `InstallSlot` to the public
/// `ClientInstallSlot` and adding `ClientPresence.slot` must leave
/// detection behavior byte-identical — the only new thing on the wire is
/// the `slot` field itself. For every existing fixture this asserts (a)
/// exactly one record per defined slot, in the fixed probe-table order,
/// (b) each record's `label` is still derived from its own `slot` (never
/// independently set, never drifted), and (c) `installations`/`issues`/
/// ordering are unaffected — already pinned byte-for-byte by this file's
/// other per-fixture tests, which this test runs alongside without
/// altering their fixtures or assertions.
#[test]
fn slot_promotion_leaves_detection_output_unchanged_except_for_the_new_field() {
    let expected_slot_order = [
        ClientInstallSlot::ClaudeCodeNpm,
        ClientInstallSlot::ClaudeCodeBundled,
        ClientInstallSlot::OpenCodeNpm,
        ClientInstallSlot::OpenCodeDesktop,
        ClientInstallSlot::CodexStandalone,
    ];

    for case in [
        "nothing",
        "packaged-and-legacy",
        "packaged",
        "legacy",
        "two-packages",
        "packaged-empty",
        "non-claude-packages",
        "packages-unreadable",
        "opencode-npm",
        "isolation",
        "no-version-key",
        "version-not-a-string",
        "package-json-empty",
        "package-json-unreadable",
        "npm-dir-no-package-json",
    ] {
        let home = fixture_home(case);
        let scan = installations::scan_for(&home, HostPlatform::Windows);
        let records = scan.presence.as_ref().expect("Windows has a probe table");

        assert_eq!(records.len(), 5, "case {case}: one record per defined slot");

        let slots: Vec<ClientInstallSlot> = records.iter().map(|r| r.slot).collect();
        assert_eq!(
            slots, expected_slot_order,
            "case {case}: slot order must equal the fixed probe-table order"
        );

        for record in records {
            assert_eq!(
                record.label,
                record.slot.label(),
                "case {case}: label must remain derived from slot, not independently set"
            );
        }
    }
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
