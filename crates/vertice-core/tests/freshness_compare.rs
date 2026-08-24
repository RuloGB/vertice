//! `component-freshness` spec: `compare` is total and pure — every input
//! pair yields a `Freshness`, never a panic, never a guess. One test per
//! scenario in `specs/component-freshness/spec.md`.

use vertice_core::freshness::compare;
use vertice_core::model::Freshness;

#[test]
fn an_older_installed_version_is_outdated_carrying_the_reference() {
    let verdict = compare("1.0.0", "1.1.0");

    assert_eq!(
        verdict,
        Freshness::Outdated {
            latest: "1.1.0".to_string()
        }
    );
}

#[test]
fn an_equal_installed_version_is_up_to_date() {
    assert_eq!(compare("1.0.0", "1.0.0"), Freshness::UpToDate);
}

#[test]
fn an_installed_prerelease_older_than_the_reference_is_outdated() {
    let verdict = compare("0.150.0-rc.1", "0.150.0");

    assert_eq!(
        verdict,
        Freshness::Outdated {
            latest: "0.150.0".to_string()
        },
        "the user is on a release candidate of a version that has since shipped"
    );
}

#[test]
fn an_installed_prerelease_newer_than_the_reference_is_up_to_date() {
    let verdict = compare("0.151.0-rc.1", "0.149.1");

    assert_eq!(
        verdict,
        Freshness::UpToDate,
        "ahead of the latest reference has no update to make, and is never a fourth state"
    );
}

#[test]
fn an_msix_shaped_directory_name_installed_version_is_unknown_never_a_panic() {
    let verdict = compare("Claude_8wekyb3d8bbwe", "1.0.0");

    assert_eq!(
        verdict,
        Freshness::Unknown {
            reason: "could not parse one or both version strings as semver".to_string()
        }
    );
}

#[test]
fn an_empty_installed_string_is_unknown() {
    match compare("", "1.0.0") {
        Freshness::Unknown { .. } => {}
        other => panic!("expected Unknown, got {other:?}"),
    }
}

#[test]
fn an_empty_reference_string_is_unknown() {
    match compare("1.0.0", "") {
        Freshness::Unknown { .. } => {}
        other => panic!("expected Unknown, got {other:?}"),
    }
}

#[test]
fn garbage_on_either_side_is_unknown_never_a_panic() {
    match compare("not-a-version", "also-not-a-version") {
        Freshness::Unknown { .. } => {}
        other => panic!("expected Unknown, got {other:?}"),
    }
}
