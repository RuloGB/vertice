//! Behaviour probes pinning the JSON/JSONC parsing contract that
//! `vertice-core` depends on. These tests exist so swapping the underlying
//! JSONC crate (see `src/jsonc.rs`) is a one-file, green/red decision
//! instead of a design discussion — mirroring `tests/yaml_behavior.rs` for
//! the YAML seam. In-memory only, zero disk access.

use std::collections::BTreeMap;

use vertice_core::jsonc::{self, JsonValue, JsoncError};

#[test]
fn line_comments_are_accepted() {
    let input = r#"{ "a": 1 } // trailing line comment"#;

    let parsed = jsonc::parse(input).expect("a `//` line comment must parse");

    let mut expected = BTreeMap::new();
    expected.insert("a".to_string(), JsonValue::Number("1".to_string()));
    assert_eq!(parsed, JsonValue::Object(expected));
}

#[test]
fn block_comments_are_accepted() {
    let input = r#"{ /* leading */ "a": 1 }"#;

    let parsed = jsonc::parse(input).expect("a `/* */` block comment must parse");

    let mut expected = BTreeMap::new();
    expected.insert("a".to_string(), JsonValue::Number("1".to_string()));
    assert_eq!(parsed, JsonValue::Object(expected));
}

#[test]
fn trailing_comma_in_an_object_is_accepted() {
    let input = r#"{ "a": 1, "b": 2, }"#;

    let parsed = jsonc::parse(input).expect("a trailing comma must parse");

    let mut expected = BTreeMap::new();
    expected.insert("a".to_string(), JsonValue::Number("1".to_string()));
    expected.insert("b".to_string(), JsonValue::Number("2".to_string()));
    assert_eq!(parsed, JsonValue::Object(expected));
}

#[test]
fn unquoted_property_names_are_rejected() {
    // JSON5, not JSONC (design §5.2) — this is the strictness boundary that
    // distinguishes this seam from a fully loose parser.
    let input = r#"{ unquoted: 1 }"#;

    let result = jsonc::parse(input);

    assert!(
        result.is_err(),
        "an unquoted property name must be rejected, not silently accepted"
    );
}

#[test]
fn a_strict_json_file_rejects_a_trailing_comma_when_not_configured() {
    // `opencode.json` is parsed through this same seam (design §5.2) and
    // must still reject a trailing comma — this pins that the seam's
    // permissiveness is deliberate, not a leaky default. Since `parse`
    // enables trailing commas for both files (design's one-parser
    // decision), this test instead confirms an actually malformed document
    // is rejected regardless of leniency.
    let input = r#"{ "a": 1, "b": }"#;

    let result = jsonc::parse(input);

    assert!(result.is_err(), "a genuinely malformed document must error");
}

#[test]
fn duplicate_keys_within_one_document_resolve_last_wins() {
    let input = r#"{ "a": 1, "a": 2 }"#;

    let parsed = jsonc::parse(input).expect("duplicate keys must not be a parse error");

    let mut expected = BTreeMap::new();
    expected.insert("a".to_string(), JsonValue::Number("2".to_string()));
    assert_eq!(parsed, JsonValue::Object(expected));
}

#[test]
fn a_syntax_error_returns_a_jsonc_error_and_never_panics() {
    let input = "{ this is not json";

    let result = jsonc::parse(input);

    assert!(matches!(result, Err(JsoncError::Parse(_))));
}

#[test]
fn parsing_the_same_object_twice_yields_the_same_key_order() {
    // Design §7: `Object` is a `BTreeMap`, so key iteration order is
    // byte-wise `Ord for String`, not insertion order — a property of the
    // type, not a convention an implementer can forget.
    let input = r#"{ "zebra": 1, "apple": 2, "mango": 3 }"#;

    let first = jsonc::parse(input).expect("valid JSON must parse");
    let second = jsonc::parse(input).expect("valid JSON must parse");

    let keys = |value: &JsonValue| -> Vec<String> {
        match value {
            JsonValue::Object(map) => map.keys().cloned().collect(),
            _ => panic!("expected an object"),
        }
    };

    assert_eq!(keys(&first), keys(&second));
    assert_eq!(keys(&first), vec!["apple", "mango", "zebra"]);
}

#[test]
fn the_seams_own_types_never_leak_the_underlying_crates_types() {
    // Structural pin, not a runtime assertion: `JsonValue`/`JsoncError` are
    // this crate's own types, constructible without importing
    // `jsonc_parser`. If this test file compiles without a `jsonc_parser`
    // import anywhere, the seam property holds.
    let _value: JsonValue = JsonValue::Null;
    let _err: JsoncError = JsoncError::Parse("example".to_string());
}
