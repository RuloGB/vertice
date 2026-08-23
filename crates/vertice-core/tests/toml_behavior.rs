//! Behaviour probes pinning the TOML deserialization contract that
//! `vertice-core` depends on. These tests exist so swapping the underlying
//! TOML crate (see `src/toml.rs`) is a one-file, green/red decision instead
//! of a design discussion, mirroring `tests/yaml_behavior.rs`. See
//! `openspec/changes/2026-08-23-add-codex-client-support/design.md` §5.3.

use serde::Deserialize;
use vertice_core::toml;

#[derive(Debug, Deserialize, PartialEq)]
struct Doc {
    name: String,
    developer_instructions: Option<String>,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Required {
    name: String,
}

#[test]
fn multiline_triple_quoted_string_is_preserved_verbatim() {
    let input = "name = \"reviewer\"\ndeveloper_instructions = \"\"\"\nLine one.\n\nLine two, after a blank line.\n\"\"\"\n";

    let doc: Doc = toml::from_str(input).expect("multiline string should parse");

    assert_eq!(
        doc.developer_instructions.as_deref(),
        Some("Line one.\n\nLine two, after a blank line.\n")
    );
}

#[test]
fn escapes_inside_a_basic_string_are_decoded() {
    let input = "name = \"line one\\nline two\"\n";

    let doc: Doc = toml::from_str(input).expect("escaped string should parse");

    assert_eq!(doc.name, "line one\nline two");
}

#[test]
fn missing_required_field_surfaces_as_an_error() {
    let input = "developer_instructions = \"only instructions, no name\"\n";

    let result: Result<Required, _> = toml::from_str(input);

    let err = result.expect_err("a missing required field must be an Err, not a panic");
    assert!(
        err.to_string().contains("missing field"),
        "unexpected error message: {err}"
    );
}

#[test]
fn unknown_key_is_ignored() {
    let input = "name = \"reviewer\"\nextra_key = \"unmodelled\"\n[metadata]\nnested = true\n";

    let doc: Required = toml::from_str(input).expect("unmodelled keys must be ignored");

    assert_eq!(doc.name, "reviewer");
}
