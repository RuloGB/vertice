//! `component-freshness` spec: `evaluate` over the `ReferenceVersions`
//! seam. The two load-bearing tests named in design §14, plus the mapping
//! unit test (task 3.8). No network, no I/O — `MapReferenceVersions` is a
//! fixed, in-memory stub.

use std::path::PathBuf;

use vertice_core::freshness::{evaluate, MapReferenceVersions, ReferenceLookup};
use vertice_core::model::{ClientInstallSlot, Freshness, FreshnessSubject};

fn client_installation_subject(slot: ClientInstallSlot, path: &str) -> FreshnessSubject {
    FreshnessSubject::ClientInstallation {
        slot,
        path: PathBuf::from(path),
    }
}

/// The design §14 / §6 pin: a subject with no known upstream mapping never
/// reports `UpToDate`, for any installed/reference pair.
#[test]
fn no_upstream_slot_is_never_up_to_date() {
    let subject = client_installation_subject(ClientInstallSlot::ClaudeCodeBundled, "/bundled");
    let source = MapReferenceVersions::new().with(
        subject.clone(),
        ReferenceLookup::NoUpstream {
            reason: "the bundled Claude Desktop runtime has no established queryable upstream"
                .to_string(),
        },
    );

    for installed in ["1.0.0", "999.999.999", "not-a-version", ""] {
        let checks = evaluate(&source, &[(subject.clone(), installed.to_string())]);

        assert_eq!(checks.len(), 1);
        match &checks[0].verdict {
            Freshness::Unknown { .. } => {}
            other => panic!(
                "installed {installed:?}: a subject with no known upstream must never report {other:?}"
            ),
        }
    }
}

/// The design §14 / offline-first-run pin: an unavailable reference source
/// yields `Unknown` for every subject and produces zero diagnostic-channel
/// (`ScanIssue`-shaped) side effects — `evaluate`'s return type carries no
/// such channel at all, so "zero issues" holds by construction, verified
/// here by checking every verdict really is `Unknown`.
#[test]
fn unavailable_source_yields_unknown_for_every_subject_and_zero_issues() {
    let source = MapReferenceVersions::new(); // configured for nothing at all

    let subjects = vec![
        (
            client_installation_subject(ClientInstallSlot::ClaudeCodeNpm, "/npm"),
            "1.0.0".to_string(),
        ),
        (
            client_installation_subject(ClientInstallSlot::OpenCodeNpm, "/opencode"),
            "2.0.0".to_string(),
        ),
        (
            client_installation_subject(ClientInstallSlot::CodexStandalone, "/codex"),
            "0.149.0".to_string(),
        ),
    ];

    let checks = evaluate(&source, &subjects);

    assert_eq!(checks.len(), 3);
    for check in &checks {
        match &check.verdict {
            Freshness::Unknown { .. } => {}
            other => panic!("expected Unknown for an unavailable source, got {other:?}"),
        }
    }
}

/// Task 3.8: `evaluate` maps each `ReferenceLookup` variant to the right
/// verdict — `Found` runs the comparison, `Unavailable`/`NoUpstream` both
/// degrade to `Unknown`.
#[test]
fn evaluate_maps_each_reference_lookup_variant_to_the_right_verdict() {
    let found_subject = client_installation_subject(ClientInstallSlot::OpenCodeNpm, "/opencode");
    let unavailable_subject = client_installation_subject(ClientInstallSlot::ClaudeCodeNpm, "/npm");
    let no_upstream_subject =
        client_installation_subject(ClientInstallSlot::ClaudeCodeBundled, "/bundled");

    let source = MapReferenceVersions::new()
        .with(
            found_subject.clone(),
            ReferenceLookup::Found("2.0.0".to_string()),
        )
        .with(
            unavailable_subject.clone(),
            ReferenceLookup::Unavailable {
                reason: "network unreachable".to_string(),
            },
        )
        .with(
            no_upstream_subject.clone(),
            ReferenceLookup::NoUpstream {
                reason: "no established upstream".to_string(),
            },
        );

    let checks = evaluate(
        &source,
        &[
            (found_subject, "1.0.0".to_string()),
            (unavailable_subject, "1.0.0".to_string()),
            (no_upstream_subject, "1.0.0".to_string()),
        ],
    );

    assert_eq!(
        checks[0].verdict,
        Freshness::Outdated {
            latest: "2.0.0".to_string()
        },
        "Found runs the comparison"
    );
    assert!(
        matches!(checks[1].verdict, Freshness::Unknown { .. }),
        "Unavailable degrades to Unknown"
    );
    assert!(
        matches!(checks[2].verdict, Freshness::Unknown { .. }),
        "NoUpstream degrades to Unknown"
    );
}
