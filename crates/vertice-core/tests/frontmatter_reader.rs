//! Fixture-driven behaviour tests for `vertice_core::frontmatter::read`.
//!
//! One test per fixture committed under
//! `crates/vertice-core/tests/fixtures/frontmatter/`, plus the I/O-failure
//! class (a repository-relative path with no fixture on disk), the
//! non-UTF-8 byte tripwire, and the generic-reuse probe.
//! `openspec/changes/skill-frontmatter-reader/design.md` §3/§5/§7 is the
//! authority for every asserted `severity` and `reason` shape.

use std::path::{Path, PathBuf};

use serde::Deserialize;
use vertice_core::frontmatter::{self, SkillFrontmatter};
use vertice_core::model::IssueSeverity;

/// Build a path under `crates/vertice-core/tests/fixtures/frontmatter/<case>/SKILL.md`
/// from per-segment pushes — never a `"/"`-joined literal, so it stays
/// separator-correct on Windows.
fn fixture_path(case: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("fixtures");
    path.push("frontmatter");
    path.push(case);
    path.push("SKILL.md");
    path
}

#[test]
fn valid_minimal_returns_the_exact_name_and_description() {
    let path = fixture_path("valid-minimal");

    let fm: SkillFrontmatter = frontmatter::read(&path).expect("valid frontmatter should parse");

    assert_eq!(fm.name, "valid-minimal");
    assert_eq!(
        fm.description,
        Some("A minimal skill with a single-line description.".to_string())
    );
}

/// CA-10: the folded `description: >` scalar is asserted in full, never a
/// prefix.
#[test]
fn valid_folded_description_is_complete_and_correct() {
    let path = fixture_path("valid-folded-description");

    let fm: SkillFrontmatter = frontmatter::read(&path).expect("folded frontmatter should parse");

    assert_eq!(fm.name, "valid-folded-description");
    assert_eq!(
        fm.description,
        Some(
            "This description is written as a folded block scalar that spans several lines \
             of source and must be joined into a single string with spaces, never truncated \
             or altered.\n"
                .to_string()
        )
    );
}

/// A description-less skill is a success, not a failure — `Component.description`
/// is `Option<String>` and this case must still reach T8's consolidation count.
#[test]
fn valid_no_description_succeeds_with_none() {
    let path = fixture_path("valid-no-description");

    let fm: SkillFrontmatter =
        frontmatter::read(&path).expect("a missing description key is not a failure");

    assert_eq!(fm.name, "valid-no-description");
    assert_eq!(fm.description, None);
}

#[test]
fn no_frontmatter_is_a_warning_carrying_its_path() {
    let path = fixture_path("no-frontmatter");

    let issue: vertice_core::model::ScanIssue =
        frontmatter::read::<SkillFrontmatter>(&path).expect_err("plain Markdown has no fence");

    assert_eq!(issue.severity, IssueSeverity::Warning);
    assert_eq!(issue.path, Some(path));
    assert!(
        issue.reason.contains("no frontmatter block"),
        "unexpected reason: {}",
        issue.reason
    );
}

#[test]
fn empty_file_is_a_warning_distinct_from_absent_frontmatter() {
    let path = fixture_path("empty");

    let issue = frontmatter::read::<SkillFrontmatter>(&path).expect_err("empty file must fail");

    assert_eq!(issue.severity, IssueSeverity::Warning);
    assert_eq!(issue.path, Some(path));
    assert_eq!(issue.reason, "file is empty");
}

/// CA-12 partial: a corrupt fenced block carries its path and a non-empty
/// `reason` describing the parse failure.
#[test]
fn corrupt_yaml_carries_its_path_and_a_parse_reason() {
    let path = fixture_path("corrupt-yaml");

    let issue = frontmatter::read::<SkillFrontmatter>(&path).expect_err("malformed YAML must fail");

    assert_eq!(issue.severity, IssueSeverity::Error);
    assert_eq!(issue.path, Some(path));
    assert!(
        issue.reason.starts_with("frontmatter is not valid YAML:"),
        "unexpected reason: {}",
        issue.reason
    );
    assert!(!issue.reason.is_empty());
}

#[test]
fn missing_name_is_an_error_carrying_its_path() {
    let path = fixture_path("missing-name");

    let issue = frontmatter::read::<SkillFrontmatter>(&path)
        .expect_err("an absent required field must fail");

    assert_eq!(issue.severity, IssueSeverity::Error);
    assert_eq!(issue.path, Some(path));
    assert!(
        issue.reason.starts_with("frontmatter is not valid YAML:"),
        "unexpected reason: {}",
        issue.reason
    );
}

#[test]
fn type_mismatch_name_is_an_error_carrying_its_path() {
    let path = fixture_path("type-mismatch-name");

    let issue = frontmatter::read::<SkillFrontmatter>(&path)
        .expect_err("a sequence fed to a String field must fail");

    assert_eq!(issue.severity, IssueSeverity::Error);
    assert_eq!(issue.path, Some(path));
    assert!(
        issue.reason.contains("invalid type"),
        "unexpected reason: {}",
        issue.reason
    );
}

#[test]
fn unterminated_fence_is_an_error_carrying_its_path() {
    let path = fixture_path("unterminated-fence");

    let issue = frontmatter::read::<SkillFrontmatter>(&path)
        .expect_err("an opening fence with no closing fence must fail");

    assert_eq!(issue.severity, IssueSeverity::Error);
    assert_eq!(issue.path, Some(path));
    assert!(
        issue.reason.contains("unterminated frontmatter block"),
        "unexpected reason: {}",
        issue.reason
    );
}

/// Non-UTF-8 *content* carries `path: Some(path)`, never `None` — distinct
/// from T2's non-UTF-8 *path* contract, which is T4's concern.
#[test]
fn non_utf8_content_is_a_warning_carrying_its_path_never_none() {
    let path = fixture_path("non-utf8-content");

    let issue =
        frontmatter::read::<SkillFrontmatter>(&path).expect_err("non-UTF-8 bytes must fail");

    assert_eq!(issue.severity, IssueSeverity::Warning);
    assert_eq!(issue.path, Some(path));
    assert!(
        issue.reason.contains("not valid UTF-8"),
        "unexpected reason: {}",
        issue.reason
    );
}

/// The sole failure class with no fixture on disk: a repository-relative
/// path that does not exist.
#[test]
fn unreadable_path_is_a_warning_carrying_its_path() {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("fixtures");
    path.push("frontmatter");
    path.push("does-not-exist-on-disk");
    path.push("SKILL.md");

    let issue = frontmatter::read::<SkillFrontmatter>(&path).expect_err("a missing file must fail");

    assert_eq!(issue.severity, IssueSeverity::Warning);
    assert_eq!(issue.path, Some(path));
    assert!(
        issue.reason.starts_with("could not read file:"),
        "unexpected reason: {}",
        issue.reason
    );
}

/// Spec: Generic Over the Deserialization Target. A second, non-skill
/// target type reads `valid-folded-description/SKILL.md` through the same
/// read/split/error path, unchanged, proving unknown-field tolerance
/// (`license`, `disable-model-invocation`, `metadata`) with no new fixture.
#[test]
fn reader_is_generic_over_a_second_non_skill_target_type() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct LicenseProbe {
        license: String,
    }

    let path = fixture_path("valid-folded-description");

    let probe: LicenseProbe =
        frontmatter::read(&path).expect("a second target type should reuse the same reader");

    assert_eq!(probe.license, "MIT");
}

/// Tripwire (design §9): a mangled fixture fails loudly here instead of
/// corrupting an unrelated assertion. If this test fails, check
/// `.gitattributes`, not the reader — 425 is the exact byte length recorded
/// against Work Unit 1's fixture authoring evidence
/// (`apply-progress.md`, task 1.3).
#[test]
fn non_utf8_fixture_is_still_non_utf8_on_disk() {
    let path = fixture_path("non-utf8-content");

    let bytes = std::fs::read(&path).expect("fixture must be readable");

    assert_eq!(
        bytes.len(),
        425,
        ".gitattributes tripwire: fixture byte length changed — checkout likely rewrote line \
         endings"
    );
    assert!(
        std::str::from_utf8(&bytes).is_err(),
        ".gitattributes tripwire: fixture was sanitized into valid UTF-8"
    );
}

/// Requirement: Single-File Input Only — the reader touches only the given
/// path and discovers no other.
#[test]
fn reader_touches_only_the_given_path() {
    let path: &Path = &fixture_path("valid-minimal");
    let sibling = fixture_path("valid-folded-description");

    let fm: SkillFrontmatter = frontmatter::read(path).expect("valid frontmatter should parse");

    assert_eq!(fm.name, "valid-minimal");
    assert_ne!(path, sibling);
}
