//! Behaviour probes pinning the YAML deserialization contract that
//! `vertice-core` depends on. These tests exist so swapping the underlying
//! YAML crate (see `src/yaml.rs`) is a one-file, green/red decision instead
//! of a design discussion. See `openspec/changes/bootstrap-workspace-ci/design.md`
//! ("YAML crate" decision) for the crates evaluated and why.

use serde::Deserialize;
use vertice_core::yaml;

#[derive(Debug, Deserialize, PartialEq)]
struct Doc {
    description: String,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Flags {
    enabled: String,
    version: String,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Simple {
    value: String,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Probe {
    name: String,
    #[allow(dead_code)]
    description: Option<String>,
}

#[test]
fn folded_scalar_joins_lines_with_spaces() {
    let input = "description: >\n  This is a\n  folded block\n  scalar.\n";

    let doc: Doc = yaml::from_str(input).expect("folded scalar should parse");

    assert_eq!(doc.description, "This is a folded block scalar.\n");
}

#[test]
fn literal_scalar_preserves_newlines() {
    let input = "description: |\n  Line one\n  Line two\n";

    let doc: Doc = yaml::from_str(input).expect("literal scalar should parse");

    assert_eq!(doc.description, "Line one\nLine two\n");
}

#[test]
fn unquoted_no_is_treated_as_a_string_not_a_boolean() {
    // This is the reason `serde_norway` was chosen over a YAML-1.1-resolving
    // fork: third-party frontmatter we don't control can contain bareword
    // `no`/`yes` that must NOT be coerced into a boolean when the target
    // field is a `String`.
    let input = "enabled: no\nversion: placeholder\n";

    let flags: Flags = yaml::from_str(input).expect("unquoted `no` should parse into a String");

    assert_eq!(flags.enabled, "no");
}

#[test]
fn unquoted_float_looking_value_is_treated_as_a_string() {
    // Same rationale: unquoted `2.0` must not be coerced into a float when
    // the target field is a `String` (e.g. semantic version strings).
    let input = "enabled: yes\nversion: 2.0\n";

    let flags: Flags = yaml::from_str(input).expect("unquoted `2.0` should parse into a String");

    assert_eq!(flags.version, "2.0");
}

#[test]
fn crlf_line_endings_are_normalized_in_literal_scalars() {
    let input = "description: |\r\n  Line one\r\n  Line two\r\n";

    let doc: Doc = yaml::from_str(input).expect("CRLF input should parse");

    assert_eq!(doc.description, "Line one\nLine two\n");
}

#[test]
fn duplicate_keys_are_rejected_as_a_parse_error() {
    // Observed `serde_norway` behaviour: unlike some YAML 1.1 parsers that
    // silently let the last key win, `serde_norway` treats a repeated
    // mapping key as a hard deserialization error. Pinned here so a crate
    // swap that silently changed this to "last wins" would be caught.
    let input = "value: first\nvalue: second\n";

    let result: Result<Simple, _> = yaml::from_str(input);

    let err = result.expect_err("duplicate keys should be rejected");
    assert!(
        err.to_string().contains("duplicate field"),
        "unexpected error message: {err}"
    );
}

#[test]
fn scalar_field_given_a_yaml_sequence_returns_an_error_not_a_panic() {
    // T3 checkpoint (openspec/changes/skill-frontmatter-reader/design.md §10):
    // at this point in the apply sequence `frontmatter.rs` does not exist yet,
    // so this probe calls the seam directly to confirm, before the module is
    // built, that `serde_norway` returns `Err` on a type mismatch instead of
    // panicking. The YAML fragment below is hardcoded inline and matches
    // `tests/fixtures/frontmatter/type-mismatch-name/SKILL.md`'s frontmatter
    // block. If this test ever panics instead of failing an assertion, the
    // fix belongs in `src/yaml.rs`, not in a future `frontmatter.rs`.
    let input = "name:\n  - not\n  - a\n  - string\ndescription: The name field above is a YAML sequence, not a scalar string.\n";

    let result: Result<Probe, _> = yaml::from_str(input);

    let err =
        result.expect_err("a YAML sequence fed to a String field must be an Err, not a panic");
    assert!(
        err.to_string().contains("invalid type"),
        "unexpected error message: {err}"
    );
}
