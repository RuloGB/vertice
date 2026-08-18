//! Fixture-driven behaviour tests for `vertice_core::skills::scan` and
//! `vertice_core::roots::skill_roots`, over the synthetic-home fixture tree
//! committed under `crates/vertice-core/tests/fixtures/roots/`. One test (or
//! tight group) per skill-scanner spec requirement; `design.md` §7/§8 is the
//! authority for every asserted `status`/`severity`/`reason` shape.
//!
//! `openspec/changes/skill-scanner-user-roots/design.md` §8's `.gitkeep`
//! tripwire is deliberately split in two: the disk-existence half below
//! needs no `roots`/`skills` module and lands with the fixture tree; the
//! `status == Found` half lands once `roots::skill_roots` exists.

use std::path::PathBuf;

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
