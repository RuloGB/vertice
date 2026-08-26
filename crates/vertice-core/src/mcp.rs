//! Shared, I/O-free MCP redaction primitives (design §3, §4, §6.2, §6.3).
//!
//! Nothing in this module reads a file, a path, or the clock. Every
//! function here is pure and unit-tested without a fixture. This is the
//! single small file a security reviewer must read: `sanitize_url` is the
//! ONLY place a raw remote URL is ever transformed, `KeyNames`/`ArgCount`/
//! `Lenient` are the ONLY primitives that extract shape from a TOML map or
//! sequence without ever allocating a value, and `discriminate_transport`
//! is the ONLY place transport discrimination happens (the 3×3 matrix,
//! design §6.3, enumerated once and nowhere else).
//!
//! Deny-lint scope (design §13.5 E2, tasks 2.6): this file's own non-test
//! code MUST NOT unwrap, MUST NOT expect, and MUST NOT index or slice with
//! `[]` — every extraction that can fail returns `Option`/`Result` and
//! every count uses a non-panicking operation. The test module below is
//! exempted, matching this crate's inline-test convention. (Deliberately
//! not spelled with the literal method-call syntax here: this paragraph is
//! itself scanned, and correctly matched, by
//! `tests/mcp_no_error_interpolation_invariant.rs`'s textual panic-surface
//! check — see that file's own module doc for why.)
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use serde::de::{Deserialize, Deserializer, IgnoredAny, MapAccess, SeqAccess, Visitor};

use crate::model::{Component, McpTransport, ScanIssue, SearchRoot};

/// Owned result of one client's MCP scan. A distinct type per the house
/// rule (OAD §5.5) — but ONE type shared by the three MCP adapters, since
/// all three produce exactly one root, N components and N issues. `pub`
/// (not `pub(crate)`) because each adapter module's `scan` function is
/// itself part of the crate's public surface and re-exports this type
/// (`mcp` itself stays a private module — see `mcp_claude`/`mcp_opencode`/
/// `mcp_codex`'s `pub use`).
#[derive(Debug, Clone, PartialEq)]
pub struct McpScan {
    /// Always exactly one root.
    pub roots: Vec<SearchRoot>,
    pub components: Vec<Component>,
    pub issues: Vec<ScanIssue>,
}

// ---------------------------------------------------------------------
// §3 — URL sanitization
// ---------------------------------------------------------------------

/// Reduce a configured remote URL to the endpoint origin, or refuse.
/// Purely subtractive: every character in the output is copied verbatim
/// from `raw`. Nothing is normalized, lowercased, decoded or invented.
///
/// The invariant this rule holds: no input may produce an output
/// containing any byte that came from the userinfo component; when the
/// authority's structure is ambiguous, the function refuses rather than
/// guesses (design §3.1). Every input is either accepted with a
/// userinfo-free authority, or rejected outright — never truncated into
/// something that merely looks clean.
pub(crate) fn sanitize_url(raw: &str) -> Option<String> {
    // Step 1: reject empty, whitespace, or any control character.
    if raw.is_empty() || raw.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return None;
    }

    // Step 2: split once on "://"; validate the scheme.
    let (scheme, rest) = raw.split_once("://")?;
    if !is_valid_scheme(scheme) {
        return None;
    }

    // Step 3: candidate authority is everything up to the first /, ?, #;
    // everything from that delimiter onward is the tail.
    let (candidate_authority, tail) = split_authority(rest);

    // Step 4: reject if the tail contains any '@' — the boundary is
    // ambiguous, and the rule never guesses.
    if tail.contains('@') {
        return None;
    }

    // Step 5: userinfo — keep only what follows the LAST '@' within the
    // candidate authority, now that step 4 confirmed the tail is clean.
    let authority = match candidate_authority.rsplit_once('@') {
        Some((_, userinfo_free)) => userinfo_free,
        None => candidate_authority,
    };

    // Step 6: host and port.
    let host_port = sanitize_host_port(authority)?;

    // Step 7: emit.
    Some(format!("{scheme}://{host_port}"))
}

fn is_valid_scheme(scheme: &str) -> bool {
    let mut chars = scheme.chars();
    match chars.next() {
        Some(first) if first.is_ascii_alphabetic() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.')
}

/// Split `rest` into the candidate authority (everything up to the first
/// `/`, `?` or `#`) and the tail (that delimiter onward, or empty). Uses
/// `str::split_at`, never bracket indexing — the split point always comes
/// from `str::find` over single-byte ASCII delimiters, so it is always a
/// valid char boundary.
fn split_authority(rest: &str) -> (&str, &str) {
    match rest.find(['/', '?', '#']) {
        Some(idx) => rest.split_at(idx),
        None => (rest, ""),
    }
}

/// Validate and format the host (optionally bracketed IPv6) and optional
/// port from an already userinfo-free authority.
fn sanitize_host_port(authority: &str) -> Option<String> {
    if let Some(rest) = authority.strip_prefix('[') {
        let (bracketed, after) = rest.split_once(']')?;
        if bracketed.is_empty()
            || !bracketed
                .chars()
                .all(|c| c.is_ascii_hexdigit() || c == ':' || c == '.')
        {
            return None;
        }
        if after.is_empty() {
            return Some(format!("[{bracketed}]"));
        }
        let port = after.strip_prefix(':')?;
        if port.is_empty() || !port.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        return Some(format!("[{bracketed}]:{port}"));
    }

    match authority.rsplit_once(':') {
        Some((host, port)) => {
            if port.is_empty() || !port.chars().all(|c| c.is_ascii_digit()) {
                return None;
            }
            if host.is_empty() || host.chars().any(is_forbidden_host_char) {
                return None;
            }
            Some(format!("{host}:{port}"))
        }
        None => {
            if authority.is_empty() || authority.chars().any(is_forbidden_host_char) {
                return None;
            }
            Some(authority.to_string())
        }
    }
}

fn is_forbidden_host_char(c: char) -> bool {
    matches!(c, '@' | '/' | '\\' | '?' | '#' | '[' | ']')
}

// ---------------------------------------------------------------------
// §6.2 — TOML-side redaction primitives
// ---------------------------------------------------------------------

/// Deserializes ANY map, keeping only its key NAMES. Every value is
/// consumed with `serde::de::IgnoredAny`, so no value is ever allocated,
/// bound, or formatted. There is no constructor that accepts a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KeyNames(pub Vec<String>);

impl<'de> Deserialize<'de> for KeyNames {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct KeyNamesVisitor;

        impl<'de> Visitor<'de> for KeyNamesVisitor {
            type Value = KeyNames;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a map")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut keys = Vec::new();
                while let Some(key) = map.next_key::<String>()? {
                    map.next_value::<IgnoredAny>()?;
                    keys.push(key);
                }
                Ok(KeyNames(keys))
            }
        }

        deserializer.deserialize_map(KeyNamesVisitor)
    }
}

/// Deserializes ANY sequence, keeping only its LENGTH. Every element is
/// consumed with `IgnoredAny`. A count cannot carry a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ArgCount(pub usize);

impl<'de> Deserialize<'de> for ArgCount {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ArgCountVisitor;

        impl<'de> Visitor<'de> for ArgCountVisitor {
            type Value = ArgCount;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a sequence")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut count = 0usize;
                while seq.next_element::<IgnoredAny>()?.is_some() {
                    count = count.saturating_add(1);
                }
                Ok(ArgCount(count))
            }
        }

        deserializer.deserialize_seq(ArgCountVisitor)
    }
}

/// A field that degrades instead of failing the document. `serde` is
/// all-or-nothing per document, so a single wrong-typed field would fail
/// the whole `from_str` and turn a per-entry `Warning` into a file-level
/// `Error`. `Lenient<T>` tries `T` first via `serde::de::value`'s scalar
/// re-deserializers; a sequence or a map that is not `T`'s own shape
/// degrades to `WrongType` without allocating anything beyond what was
/// already parsed by the outer document walk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Lenient<T> {
    Value(T),
    WrongType,
}

impl<'de, T> Deserialize<'de> for Lenient<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct LenientVisitor<T>(std::marker::PhantomData<T>);

        impl<'de, T> Visitor<'de> for LenientVisitor<T>
        where
            T: Deserialize<'de>,
        {
            type Value = Lenient<T>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("any value, degrading to WrongType on a shape mismatch")
            }

            fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(
                    match T::deserialize(serde::de::value::BoolDeserializer::<E>::new(v)) {
                        Ok(value) => Lenient::Value(value),
                        Err(_) => Lenient::WrongType,
                    },
                )
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(
                    match T::deserialize(serde::de::value::I64Deserializer::<E>::new(v)) {
                        Ok(value) => Lenient::Value(value),
                        Err(_) => Lenient::WrongType,
                    },
                )
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(
                    match T::deserialize(serde::de::value::U64Deserializer::<E>::new(v)) {
                        Ok(value) => Lenient::Value(value),
                        Err(_) => Lenient::WrongType,
                    },
                )
            }

            fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(
                    match T::deserialize(serde::de::value::F64Deserializer::<E>::new(v)) {
                        Ok(value) => Lenient::Value(value),
                        Err(_) => Lenient::WrongType,
                    },
                )
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(
                    match T::deserialize(serde::de::value::StrDeserializer::<E>::new(v)) {
                        Ok(value) => Lenient::Value(value),
                        Err(_) => Lenient::WrongType,
                    },
                )
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                self.visit_str(&v)
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                while seq.next_element::<IgnoredAny>()?.is_some() {}
                Ok(Lenient::WrongType)
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                while map.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {}
                Ok(Lenient::WrongType)
            }
        }

        deserializer.deserialize_any(LenientVisitor(std::marker::PhantomData))
    }
}

// ---------------------------------------------------------------------
// §6.3 — the total transport-discrimination matrix (E1)
// ---------------------------------------------------------------------

/// An already-extracted, normalized `command` reading, common to all three
/// clients: absent, usable (present, not wrong-typed, not empty), or
/// unusable (present but wrong-typed or empty).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CommandInput {
    Absent,
    Usable {
        command: String,
        arg_count: usize,
        env_keys: Vec<String>,
    },
    Unusable,
}

/// An already-extracted, normalized `url` reading: absent, valid (survived
/// `sanitize_url`), or unsanitizable (present but wrong-typed, or refused
/// by `sanitize_url`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum UrlInput {
    Absent,
    Valid {
        url: String,
        header_keys: Vec<String>,
    },
    Unsanitizable,
}

/// One classification outcome from `discriminate_transport`'s matrix,
/// named after §7.1's reason rows. The exact `ScanIssue.reason` text
/// (including the server key and client label — the only two identifiers
/// §7.2 permits interpolating) is built by the calling per-client adapter;
/// this type carries only the category, never formatted text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TransportIssue {
    /// Neither a usable `command` nor a valid `url` was declared.
    NeitherCommandNorUrl,
    /// The `url` could not be reduced to a safe endpoint, and there was no
    /// usable `command` to fall back to.
    UrlUnsafe,
    /// Both a usable `command` and a valid `url` were declared; the
    /// command was used.
    BothDeclaredCommandUsed,
    /// `command` was present but not usable, and there was no valid `url`
    /// to fall back to.
    NoReadableCommand,
    /// `command` was present but not usable; the `url` was used instead.
    NoReadableCommandUrlUsed,
}

/// The total result of discriminating one entry's transport: at most one
/// transport, at most one issue — never more, per design §6.3's matrix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TransportOutcome {
    pub transport: Option<McpTransport>,
    pub issue: Option<TransportIssue>,
}

/// The rule is a total function over one 3×3 matrix, enumerated here ONCE
/// and nowhere else (design §6.3). Every other statement of this rule is
/// derived from this table; if any prose elsewhere disagrees, this
/// function is the defect to fix, not this function.
pub(crate) fn discriminate_transport(command: CommandInput, url: UrlInput) -> TransportOutcome {
    match (command, url) {
        (CommandInput::Absent, UrlInput::Absent) => TransportOutcome {
            transport: None,
            issue: Some(TransportIssue::NeitherCommandNorUrl),
        },
        (CommandInput::Absent, UrlInput::Valid { url, header_keys }) => TransportOutcome {
            transport: Some(McpTransport::Remote { url, header_keys }),
            issue: None,
        },
        (CommandInput::Absent, UrlInput::Unsanitizable) => TransportOutcome {
            transport: None,
            issue: Some(TransportIssue::UrlUnsafe),
        },
        (
            CommandInput::Usable {
                command,
                arg_count,
                env_keys,
            },
            UrlInput::Absent,
        ) => TransportOutcome {
            transport: Some(McpTransport::Stdio {
                command,
                arg_count,
                env_keys,
            }),
            issue: None,
        },
        (
            CommandInput::Usable {
                command,
                arg_count,
                env_keys,
            },
            UrlInput::Valid { .. },
        ) => TransportOutcome {
            transport: Some(McpTransport::Stdio {
                command,
                arg_count,
                env_keys,
            }),
            issue: Some(TransportIssue::BothDeclaredCommandUsed),
        },
        (
            CommandInput::Usable {
                command,
                arg_count,
                env_keys,
            },
            UrlInput::Unsanitizable,
        ) => TransportOutcome {
            // The usable command was selected on its own merits and
            // nothing was lost; the entry's surplus URL was never a
            // transport candidate, so this cell is silent (design §6.3,
            // resolved 2026-08-25).
            transport: Some(McpTransport::Stdio {
                command,
                arg_count,
                env_keys,
            }),
            issue: None,
        },
        (CommandInput::Unusable, UrlInput::Absent) => TransportOutcome {
            transport: None,
            issue: Some(TransportIssue::NoReadableCommand),
        },
        (CommandInput::Unusable, UrlInput::Valid { url, header_keys }) => TransportOutcome {
            transport: Some(McpTransport::Remote { url, header_keys }),
            issue: Some(TransportIssue::NoReadableCommandUrlUsed),
        },
        (CommandInput::Unusable, UrlInput::Unsanitizable) => TransportOutcome {
            transport: None,
            issue: Some(TransportIssue::NoReadableCommand),
        },
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    // -- §3.2's full table: acceptance rows --

    /// Anchor 0.3 (`tasks.md`): the load-bearing URL from design §10.2 —
    /// a credential in userinfo, path, query AND fragment simultaneously —
    /// reduces to exactly the sanitized origin, proving port-preserved,
    /// everything-else-dropped in one assertion.
    #[test]
    fn dirty_url_is_reduced_to_scheme_host_and_port() {
        let raw =
            "https://u_FAKE:tok_FAKE@mcp.example.test:8443/mcp/tok_FAKE?apiKey=tok_FAKE#f_FAKE";

        assert_eq!(
            sanitize_url(raw),
            Some("https://mcp.example.test:8443".to_string())
        );
    }

    #[test]
    fn simple_path_is_dropped() {
        assert_eq!(
            sanitize_url("https://mcp.example.test/mcp"),
            Some("https://mcp.example.test".to_string())
        );
    }

    #[test]
    fn port_is_preserved() {
        assert_eq!(
            sanitize_url("http://localhost:3000/sse"),
            Some("http://localhost:3000".to_string())
        );
    }

    #[test]
    fn ipv6_host_with_port_is_supported() {
        assert_eq!(
            sanitize_url("https://[::1]:8080/mcp"),
            Some("https://[::1]:8080".to_string())
        );
    }

    #[test]
    fn ipv6_host_without_port_is_supported() {
        assert_eq!(
            sanitize_url("https://[::1]/mcp"),
            Some("https://[::1]".to_string())
        );
    }

    /// Percent-encoded `%40` is not decoded and never functions as a
    /// userinfo delimiter — the rule is byte-level (design §3.2).
    #[test]
    fn percent_encoded_at_sign_passes_through_unmodified() {
        assert_eq!(
            sanitize_url("https://tok3n%40host.example/mcp"),
            Some("https://tok3n%40host.example".to_string())
        );
    }

    // -- §3.2's full table: rejection rows --

    #[test]
    fn empty_string_is_rejected() {
        assert_eq!(sanitize_url(""), None);
    }

    #[test]
    fn whitespace_inside_the_url_is_rejected() {
        assert_eq!(sanitize_url("https:// mcp.example.test"), None);
    }

    #[test]
    fn control_character_is_rejected() {
        assert_eq!(sanitize_url("https://host\u{0}.example/mcp"), None);
    }

    #[test]
    fn missing_scheme_separator_is_rejected() {
        assert_eq!(sanitize_url("mcp.example.test/mcp"), None);
    }

    #[test]
    fn scheme_starting_with_a_digit_is_rejected() {
        assert_eq!(sanitize_url("1abc://host.example"), None);
    }

    #[test]
    fn scheme_with_an_invalid_character_is_rejected() {
        assert_eq!(sanitize_url("ht!p://host.example"), None);
    }

    #[test]
    fn empty_host_is_rejected() {
        assert_eq!(sanitize_url("https://@/x"), None);
    }

    #[test]
    fn malformed_ipv6_bracket_is_rejected() {
        assert_eq!(sanitize_url("https://[::1/mcp"), None);
    }

    #[test]
    fn ipv6_bracket_with_non_hex_content_is_rejected() {
        assert_eq!(sanitize_url("https://[zzzz]:80/mcp"), None);
    }

    #[test]
    fn malformed_port_is_rejected() {
        assert_eq!(sanitize_url("https://host.example:abc/mcp"), None);
    }

    #[test]
    fn empty_port_is_rejected() {
        assert_eq!(sanitize_url("https://host.example:/mcp"), None);
    }

    /// Anchor 0.4 (`tasks.md`): the direct regression for the verified
    /// leak (design §3.1) — a userinfo containing a path delimiter (`/`,
    /// `?`, or `#`) must be REJECTED, never truncated into something that
    /// merely looks clean.
    #[test]
    fn userinfo_containing_a_path_delimiter_is_rejected_not_truncated() {
        assert_eq!(sanitize_url("https://tok3n/@host.example/mcp"), None);
        assert_eq!(sanitize_url("https://tok3n?x@host.example/mcp"), None);
        assert_eq!(sanitize_url("https://tok3n#x@host.example/mcp"), None);
    }

    /// The naive "last `@` in the whole remainder" mis-parse this rule
    /// must never repeat: `https://host/path@foo` must NOT resolve to
    /// host `foo`.
    #[test]
    fn at_sign_after_the_authority_boundary_is_rejected_not_treated_as_userinfo() {
        assert_eq!(sanitize_url("https://host/path@foo"), None);
    }

    // -- §6.2's TOML-side redaction primitives --

    #[test]
    fn key_names_deserializes_any_map_keeping_only_key_names() {
        let parsed: KeyNames =
            serde_json::from_str(r#"{"a":1,"b":"two","c":[1,2,3]}"#).expect("valid JSON");

        assert_eq!(
            parsed.0,
            vec!["a".to_string(), "b".to_string(), "c".to_string()]
        );
    }

    #[test]
    fn key_names_over_an_empty_map_yields_no_keys() {
        let parsed: KeyNames = serde_json::from_str("{}").expect("valid JSON");

        assert!(parsed.0.is_empty());
    }

    #[test]
    fn arg_count_deserializes_any_sequence_keeping_only_length() {
        let parsed: ArgCount = serde_json::from_str(r#"[1,"two",true]"#).expect("valid JSON");

        assert_eq!(parsed.0, 3);
    }

    #[test]
    fn arg_count_over_an_empty_sequence_is_zero() {
        let parsed: ArgCount = serde_json::from_str("[]").expect("valid JSON");

        assert_eq!(parsed.0, 0);
    }

    #[derive(Debug, serde::Deserialize)]
    struct LenientWrapper {
        count: Lenient<u32>,
    }

    #[test]
    fn lenient_field_matching_the_type_yields_value() {
        let parsed: LenientWrapper = serde_json::from_str(r#"{"count":5}"#).expect("valid JSON");

        assert_eq!(parsed.count, Lenient::Value(5));
    }

    #[test]
    fn lenient_field_degrades_instead_of_failing_the_document() {
        let parsed: LenientWrapper =
            serde_json::from_str(r#"{"count":"not-a-number"}"#).expect("valid JSON");

        assert_eq!(parsed.count, Lenient::WrongType);
    }

    #[test]
    fn lenient_field_wrong_shape_array_degrades_too() {
        let parsed: LenientWrapper =
            serde_json::from_str(r#"{"count":[1,2]}"#).expect("valid JSON");

        assert_eq!(parsed.count, Lenient::WrongType);
    }

    // -- §6.3's total 3×3 discrimination matrix (E1) --

    fn usable(command: &str, arg_count: usize, env_keys: &[&str]) -> CommandInput {
        CommandInput::Usable {
            command: command.to_string(),
            arg_count,
            env_keys: env_keys.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn valid(url: &str, header_keys: &[&str]) -> UrlInput {
        UrlInput::Valid {
            url: url.to_string(),
            header_keys: header_keys.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn matrix_command_absent_url_absent() {
        let outcome = discriminate_transport(CommandInput::Absent, UrlInput::Absent);

        assert_eq!(outcome.transport, None);
        assert_eq!(outcome.issue, Some(TransportIssue::NeitherCommandNorUrl));
    }

    #[test]
    fn matrix_command_absent_url_valid() {
        let outcome =
            discriminate_transport(CommandInput::Absent, valid("https://mcp.example.test", &[]));

        assert_eq!(
            outcome.transport,
            Some(McpTransport::Remote {
                url: "https://mcp.example.test".to_string(),
                header_keys: vec![],
            })
        );
        assert_eq!(outcome.issue, None);
    }

    #[test]
    fn matrix_command_absent_url_unsanitizable() {
        let outcome = discriminate_transport(CommandInput::Absent, UrlInput::Unsanitizable);

        assert_eq!(outcome.transport, None);
        assert_eq!(outcome.issue, Some(TransportIssue::UrlUnsafe));
    }

    #[test]
    fn matrix_command_usable_url_absent() {
        let outcome = discriminate_transport(usable("npx", 0, &[]), UrlInput::Absent);

        assert_eq!(
            outcome.transport,
            Some(McpTransport::Stdio {
                command: "npx".to_string(),
                arg_count: 0,
                env_keys: vec![],
            })
        );
        assert_eq!(outcome.issue, None);
    }

    #[test]
    fn matrix_command_usable_url_valid() {
        let outcome = discriminate_transport(
            usable("npx", 0, &[]),
            valid("https://mcp.example.test", &[]),
        );

        assert_eq!(
            outcome.transport,
            Some(McpTransport::Stdio {
                command: "npx".to_string(),
                arg_count: 0,
                env_keys: vec![],
            })
        );
        assert_eq!(outcome.issue, Some(TransportIssue::BothDeclaredCommandUsed));
    }

    /// The cell resolved explicitly in design §6.3 (2026-08-25): a usable
    /// `command` wins outright over an unsanitizable `url`, and the
    /// "URL refused" Warning is NOT also emitted.
    #[test]
    fn matrix_command_usable_url_unsanitizable() {
        let outcome = discriminate_transport(usable("npx", 0, &[]), UrlInput::Unsanitizable);

        assert_eq!(
            outcome.transport,
            Some(McpTransport::Stdio {
                command: "npx".to_string(),
                arg_count: 0,
                env_keys: vec![],
            })
        );
        assert_eq!(outcome.issue, None);
    }

    #[test]
    fn matrix_command_unusable_url_absent() {
        let outcome = discriminate_transport(CommandInput::Unusable, UrlInput::Absent);

        assert_eq!(outcome.transport, None);
        assert_eq!(outcome.issue, Some(TransportIssue::NoReadableCommand));
    }

    /// Anchor 0.7a (`tasks.md`) / E1: an unusable `command` with a valid
    /// `url` falls back to `Remote`, never `None` — it loses no
    /// information the entry actually offers.
    #[test]
    fn matrix_command_unusable_url_valid() {
        let outcome = discriminate_transport(
            CommandInput::Unusable,
            valid("https://mcp.example.test", &[]),
        );

        assert_eq!(
            outcome.transport,
            Some(McpTransport::Remote {
                url: "https://mcp.example.test".to_string(),
                header_keys: vec![],
            })
        );
        assert_eq!(
            outcome.issue,
            Some(TransportIssue::NoReadableCommandUrlUsed)
        );
    }

    #[test]
    fn matrix_command_unusable_url_unsanitizable() {
        let outcome = discriminate_transport(CommandInput::Unusable, UrlInput::Unsanitizable);

        assert_eq!(outcome.transport, None);
        assert_eq!(outcome.issue, Some(TransportIssue::NoReadableCommand));
    }

    /// Anchor 0.7a, named identically to the tasks.md anchor: an unusable
    /// `command` with a valid `url` falls back to `Remote`, not `None`.
    #[test]
    fn unusable_command_with_a_valid_url_falls_back_to_remote_not_none() {
        let outcome = discriminate_transport(
            CommandInput::Unusable,
            valid("https://mcp.example.test:8443", &["Authorization"]),
        );

        assert!(matches!(
            outcome.transport,
            Some(McpTransport::Remote { .. })
        ));
        assert_ne!(outcome.transport, None);
    }
}
