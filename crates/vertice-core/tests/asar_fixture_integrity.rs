//! Integrity tests for the committed `opencode-desktop` asar fixture blobs
//! (design `detect-desktop-client-installs` §8.2, tasks 4.1-4.4).
//!
//! Each fixture is committed as bytes, never generated at test time (CA-16).
//! Opacity of a committed binary blob is handled by two devices: a sidecar
//! `app.asar.layout.txt` a reviewer reads instead of the blob, and this
//! file's per-fixture integrity test, which reconstructs the expected bytes
//! from the documented inputs and asserts byte-for-byte equality against
//! the committed file. A hand-corrupted or truncated blob then fails with a
//! named assertion here instead of silently degrading into "no version",
//! which is the one failure the `client_installations.rs` behaviour suite
//! could not otherwise distinguish from a pass.
//!
//! `build_asar` below is a deliberate, documented duplicate of
//! `asar.rs`'s `#[cfg(test)]` helper of the same name: integration tests
//! under `tests/` compile as a separate crate and cannot see items gated by
//! the library's own `#[cfg(test)]`, so there is no way to share one
//! definition across the crate boundary. Both implementations follow the
//! same design §2.2/§8.1 formula and are kept in lock-step by inspection.

use std::path::PathBuf;

fn fixture_root(case: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("fixtures");
    path.push("client-installations");
    path.push("opencode-desktop");
    path.push(case);
    path.push("AppData");
    path.push("Local");
    path.push("Programs");
    path.push("@opencode-aidesktop");
    path.push("resources");
    path
}

fn archive_path(case: &str) -> PathBuf {
    let mut path = fixture_root(case);
    path.push("app.asar");
    path
}

/// Reconstruct a full asar archive from a header JSON string and a payload
/// (design §8.1/§8.2's `build_asar`, duplicated here — see module doc).
fn build_asar(header_json: &str, payload: &[u8]) -> Vec<u8> {
    let header_bytes = header_json.as_bytes();
    let json_len = u32::try_from(header_bytes.len()).expect("test header fits in u32");

    let unpadded = 4usize + header_bytes.len();
    let padded = unpadded.div_ceil(4) * 4;
    let padding_len = padded - unpadded;
    let header_payload_len = u32::try_from(padded).expect("test header fits in u32");
    let header_len = header_payload_len + 4;

    let mut out = Vec::new();
    out.extend_from_slice(&4u32.to_le_bytes());
    out.extend_from_slice(&header_len.to_le_bytes());
    out.extend_from_slice(&header_payload_len.to_le_bytes());
    out.extend_from_slice(&json_len.to_le_bytes());
    out.extend_from_slice(header_bytes);
    out.extend(std::iter::repeat_n(0u8, padding_len));
    out.extend_from_slice(payload);
    out
}

/// A raw, hand-built 16-byte (or longer) prefix, for the three fixtures
/// that are not well-formed enough to go through `build_asar` at all
/// (`oversized-header`, `bad-prefix`, `tiny-header-len`) plus the
/// deliberately truncated `truncated` fixture.
fn raw_prefix(
    pickle_header_size: u32,
    header_len: u32,
    header_payload_len: u32,
    json_len: u32,
) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&pickle_header_size.to_le_bytes());
    out.extend_from_slice(&header_len.to_le_bytes());
    out.extend_from_slice(&header_payload_len.to_le_bytes());
    out.extend_from_slice(&json_len.to_le_bytes());
    out
}

fn assert_matches_committed(case: &str, expected: &[u8]) {
    let committed =
        std::fs::read(archive_path(case)).unwrap_or_else(|err| panic!("case {case}: {err}"));
    assert_eq!(
        committed, expected,
        "case {case}: committed app.asar must equal the bytes reconstructed from its \
         app.asar.layout.txt sidecar's documented inputs"
    );
}

#[test]
fn happy_fixture_matches_its_documented_layout() {
    let header = r#"{"files":{"package.json":{"size":37,"offset":"0"}}}"#;
    let payload = br#"{"name":"opencode","version":"0.4.2"}"#;
    let expected = build_asar(header, payload);
    assert_eq!(
        expected.len(),
        105,
        "the happy fixture is exactly 105 bytes (design §8.2)"
    );
    assert_matches_committed("happy", &expected);
}

#[test]
fn oversized_header_fixture_matches_its_documented_layout() {
    let declared_json_len = 5_000_000u32;
    let header_payload_len = declared_json_len + 4;
    let header_len = header_payload_len + 4;
    let expected = raw_prefix(4, header_len, header_payload_len, declared_json_len);
    assert_eq!(expected.len(), 16);
    assert_matches_committed("oversized-header", &expected);
}

#[test]
fn bad_prefix_fixture_matches_its_documented_layout() {
    let mut expected = vec![0xFFu8; 16];
    expected.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]);
    assert_matches_committed("bad-prefix", &expected);
}

#[test]
fn tiny_header_len_fixture_matches_its_documented_layout() {
    let expected = raw_prefix(4, 0, 0, 0);
    assert_matches_committed("tiny-header-len", &expected);
}

#[test]
fn truncated_fixture_matches_its_documented_layout() {
    let header = r#"{"files":{"package.json":{"size":37,"offset":"0"}}}"#;
    let full = build_asar(header, b"");
    let expected = full[..40].to_vec();
    assert_matches_committed("truncated", &expected);
}

#[test]
fn malformed_header_fixture_matches_its_documented_layout() {
    let header = "not json {{{";
    let expected = build_asar(header, b"");
    assert_matches_committed("malformed-header", &expected);
}

#[test]
fn no_package_json_entry_fixture_matches_its_documented_layout() {
    let header = r#"{"files":{"README.md":{"size":2,"offset":"0"}}}"#;
    let payload = b"ok";
    let expected = build_asar(header, payload);
    assert_matches_committed("no-package-json-entry", &expected);
}

#[test]
fn nested_package_json_only_fixture_matches_its_documented_layout() {
    let header =
        r#"{"files":{"node_modules":{"files":{"package.json":{"size":38,"offset":"0"}}}}}"#;
    let payload = br#"{"name":"left-pad","version":"9.9.9"}"#;
    let expected = build_asar(header, payload);
    assert_matches_committed("nested-package-json-only", &expected);
}

#[test]
fn entry_out_of_range_fixture_matches_its_documented_layout() {
    let header = r#"{"files":{"package.json":{"size":10,"offset":"9999"}}}"#;
    let payload = br#"{"name":"opencode","version":"0.4.2"}"#;
    let expected = build_asar(header, payload);
    assert_matches_committed("entry-out-of-range", &expected);
}

#[test]
fn shifted_payload_fixture_matches_its_documented_layout_and_has_nonzero_padding() {
    let root_manifest = br#"{"name":"opencode","version":"0.4.2"}"#;
    let neighbour = br#"{"name":"left-pad","version":"9.9.9"}"#;
    let header = format!(
        r#"{{"files":{{"package.json":{{"size":{},"offset":"0"}}}}}}"#,
        root_manifest.len()
    );
    let mut payload = Vec::new();
    payload.extend_from_slice(root_manifest);
    payload.extend_from_slice(neighbour);
    let expected = build_asar(&header, &payload);

    // The whole point of this fixture: non-zero padding, or the forbidden
    // `data_start = json_start + json_len` formula would pass silently
    // (design §8.2).
    let json_len = header.len();
    let unpadded = 4 + json_len;
    let padding_len = unpadded.div_ceil(4) * 4 - unpadded;
    assert_ne!(padding_len, 0, "shifted-payload MUST have non-zero padding");

    assert_matches_committed("shifted-payload", &expected);
}

#[test]
fn no_name_key_fixture_matches_its_documented_layout() {
    let payload = br#"{"version":"0.4.2"}"#;
    let header = format!(
        r#"{{"files":{{"package.json":{{"size":{},"offset":"0"}}}}}}"#,
        payload.len()
    );
    let expected = build_asar(&header, payload);
    assert_matches_committed("no-name-key", &expected);
}

#[test]
fn no_version_key_fixture_matches_its_documented_layout() {
    let payload = br#"{"name":"opencode"}"#;
    let header = format!(
        r#"{{"files":{{"package.json":{{"size":{},"offset":"0"}}}}}}"#,
        payload.len()
    );
    let expected = build_asar(&header, payload);
    assert_matches_committed("no-version-key", &expected);
}

#[test]
fn empty_version_fixture_matches_its_documented_layout() {
    let payload = br#"{"name":"opencode","version":""}"#;
    let header = format!(
        r#"{{"files":{{"package.json":{{"size":{},"offset":"0"}}}}}}"#,
        payload.len()
    );
    let expected = build_asar(&header, payload);
    assert_matches_committed("empty-version", &expected);
}

// -- Tripwire (design §8.2, mirroring `skill_scanner.rs`'s `empty-alias`
// precedent): every fixture path must exist on disk. `no-asar` has no
// `app.asar` at all — its `resources/` directory is kept alive only by a
// committed `.gitkeep`, since git does not track empty directories. --

#[test]
fn every_fixture_case_directory_exists_on_disk() {
    let cases_with_archive = [
        "happy",
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
    for case in cases_with_archive {
        let path = archive_path(case);
        assert!(
            path.is_file(),
            "case {case}: app.asar must exist on disk at {path:?}"
        );
    }

    let no_asar_gitkeep = {
        let mut path = fixture_root("no-asar");
        path.push(".gitkeep");
        path
    };
    assert!(
        no_asar_gitkeep.is_file(),
        "no-asar/resources/.gitkeep must exist so git tracks the empty directory"
    );
    assert!(
        !archive_path("no-asar").exists(),
        "no-asar must have NO app.asar at all"
    );
}

// -- Sanity: `asar::read_package_version` against each committed blob
// produces the exact outcome its case name promises (design §8.2's table),
// BEFORE this is wired into `resolve_opencode_desktop_slot` (Phase 6). --

#[test]
fn read_package_version_sanity_matches_the_design_table() {
    use vertice_core::asar::{read_package_version, AsarError};

    let happy = read_package_version(&archive_path("happy")).expect("happy must read a version");
    assert_eq!(happy, "0.4.2");

    assert!(matches!(
        read_package_version(&archive_path("oversized-header")),
        Err(AsarError::HeaderTooLarge { .. })
    ));
    assert!(matches!(
        read_package_version(&archive_path("bad-prefix")),
        Err(AsarError::Malformed(_))
    ));
    assert!(matches!(
        read_package_version(&archive_path("tiny-header-len")),
        Err(AsarError::Malformed(_))
    ));
    assert!(read_package_version(&archive_path("truncated")).is_err());
    assert!(read_package_version(&archive_path("malformed-header")).is_err());
    assert!(matches!(
        read_package_version(&archive_path("no-package-json-entry")),
        Err(AsarError::Entry(_))
    ));

    let nested = read_package_version(&archive_path("nested-package-json-only"));
    assert!(matches!(nested, Err(AsarError::Entry(_))));

    assert!(matches!(
        read_package_version(&archive_path("entry-out-of-range")),
        Err(AsarError::Entry(_))
    ));

    let shifted = read_package_version(&archive_path("shifted-payload"))
        .expect("shifted-payload must read the root manifest's version");
    assert_eq!(shifted, "0.4.2");

    assert!(matches!(
        read_package_version(&archive_path("no-name-key")),
        Err(AsarError::Entry(_))
    ));
    assert!(matches!(
        read_package_version(&archive_path("no-version-key")),
        Err(AsarError::NoVersion)
    ));
    assert!(matches!(
        read_package_version(&archive_path("empty-version")),
        Err(AsarError::NoVersion)
    ));
}
