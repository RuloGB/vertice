//! JSON/JSONC deserialization seam.
//!
//! This is the ONLY module in `vertice-core` allowed to import the JSONC
//! parsing crate (`jsonc-parser`) directly. Every other module MUST go
//! through [`parse`]. Swapping the underlying JSONC crate later means
//! changing this file and `Cargo.toml` only — see `tests/jsonc_behavior.rs`
//! for the pinned behaviours this seam guarantees, and
//! `openspec/changes/opencode-agent-adapter/design.md` §5.2 for the crate
//! evaluation and decision.

use std::collections::BTreeMap;

use jsonc_parser::ParseOptions;

/// A parsed JSON/JSONC value, owned by the seam. The parsing crate's own
/// value type NEVER escapes this module — that is what makes the crate
/// swappable (design §5.2).
#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    /// Source text, verbatim. The seam makes no numeric decision for data
    /// this crate does not consume (design §5.2).
    Number(String),
    String(String),
    Array(Vec<JsonValue>),
    /// `BTreeMap`, not `HashMap` and not the crate's map type: key order is
    /// sorted-by-construction, so determinism (design §7) is a property of
    /// the type and not a convention an implementer can forget. This also
    /// fixes duplicate-key behavior inside a single document: the
    /// underlying crate's parser already resolves a repeated key to its
    /// last occurrence before this conversion runs.
    Object(BTreeMap<String, JsonValue>),
}

/// Error returned when JSON/JSONC input cannot be parsed.
#[derive(Debug, thiserror::Error)]
pub enum JsoncError {
    #[error("failed to parse JSON: {0}")]
    Parse(String),
}

/// Parser options for [`parse`], set explicitly rather than left at the
/// underlying crate's own defaults (design §5.2): comments and trailing
/// commas are accepted (real JSONC), everything else that would make this
/// parser accept JSON5-shaped or otherwise looser input than OpenCode's own
/// config loader is switched off. Unquoted/loose property names in
/// particular are JSON5, not JSONC, and are deliberately rejected.
const OPTIONS: ParseOptions = ParseOptions {
    allow_comments: true,
    allow_loose_object_property_names: false,
    allow_trailing_commas: true,
    allow_missing_commas: false,
    allow_single_quoted_strings: false,
    allow_hexadecimal_numbers: false,
    allow_unary_plus_numbers: false,
};

/// Parse JSON or JSONC (comments and trailing commas) into a `JsonValue`.
/// Unquoted property names are NOT accepted — that is JSON5, not JSONC.
///
/// Empty or whitespace-only input parses to an empty `Object` rather than
/// erroring, mirroring the underlying crate's own `None`-for-empty-input
/// behavior lifted into this seam's value type.
pub fn parse(input: &str) -> Result<JsonValue, JsoncError> {
    let parsed = jsonc_parser::parse_to_value(input, &OPTIONS)
        .map_err(|err| JsoncError::Parse(err.to_string()))?;

    Ok(match parsed {
        Some(value) => convert(value),
        None => JsonValue::Object(BTreeMap::new()),
    })
}

/// Convert the parsing crate's own (borrowed) value tree into this seam's
/// owned `JsonValue`. The only function in the crate that names
/// `jsonc_parser::JsonValue` — everything past this point sees only the
/// seam's own type.
fn convert(value: jsonc_parser::JsonValue<'_>) -> JsonValue {
    match value {
        jsonc_parser::JsonValue::Null => JsonValue::Null,
        jsonc_parser::JsonValue::Boolean(b) => JsonValue::Bool(b),
        jsonc_parser::JsonValue::Number(n) => JsonValue::Number(n.to_string()),
        jsonc_parser::JsonValue::String(s) => JsonValue::String(s.into_owned()),
        jsonc_parser::JsonValue::Array(array) => {
            JsonValue::Array(array.into_iter().map(convert).collect())
        }
        jsonc_parser::JsonValue::Object(object) => {
            let mut map = BTreeMap::new();
            for (key, value) in object {
                map.insert(key.into_owned(), convert(value));
            }
            JsonValue::Object(map)
        }
    }
}
