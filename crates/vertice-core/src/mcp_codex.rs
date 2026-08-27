//! Codex MCP adapter (design §5.1, §6.1, §6.2, §6.3, §6.4).
//!
//! Single-file root: `~/.codex/config.toml`, table `mcp_servers` (snake
//! case; `mcpServers` confirmed absent, M5). `command`/`args` are a string
//! plus a separate array, like Claude Code. Codex HAS a remote transport
//! (`{ url }` only, no `command`, no `type` anywhere — M6), so
//! discrimination is structural, like the other two clients. The ONLY
//! consumer of `KeyNames`/`ArgCount`/`Lenient` (design §6.2): on this path,
//! redaction is enforced by the deserializer itself — an argument or an
//! env/header value is never constructed, not merely unused.
//!
//! Every hand-rolled `Deserialize` impl in this module (`McpServersField`,
//! `CodexEntrySlot`, `LenientArgCount`, `LenientKeyNames`) is written to
//! NEVER return `Err` and to ALWAYS fully consume its `MapAccess`/
//! `SeqAccess` on every branch, including the "wrong shape" branches —
//! `serde`'s `MapAccess`/`SeqAccess` contract requires a visitor to drain
//! what it was handed, and a streaming deserializer's cursor is left in an
//! invalid state otherwise (a real defect found while implementing this
//! adapter: `crate::mcp::Lenient<T>`'s generic `T::deserialize` retry over
//! a mismatched map/seq shape does not itself drain, corrupting the
//! surrounding document parse). This module therefore does NOT reuse
//! `Lenient<T>` for `args`/`env`/`http_headers` (seq-/map-shaped fields) —
//! only for `command`/`url` (genuinely scalar fields, `Lenient<T>`'s
//! documented scope).
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::collections::BTreeMap;
use std::path::Path;

use serde::de::{Deserialize, Deserializer, IgnoredAny, MapAccess, SeqAccess, Visitor};

pub use crate::mcp::McpScan;
use crate::mcp::{
    discriminate_transport, sanitize_url, ArgCount, CommandInput, KeyNames, Lenient,
    TransportIssue, UrlInput,
};
use crate::model::{
    Component, ComponentId, ComponentKind, IssueSeverity, Location, LocationOrigin, McpTransport,
    ScanIssue, Scope,
};
use crate::roots;

/// Top-level Codex config document contract, permissive by design (CXD §8):
/// every field not modelled here is ignored, never an error.
#[derive(Debug, Clone, serde::Deserialize)]
struct CodexDocument {
    mcp_servers: Option<McpServersField>,
}

/// A per-field-lenient view of the `mcp_servers` table. `WrongType` when the
/// key is present but not a TOML table (design §6.2/§7.1) — hand-rolled
/// rather than `Lenient<BTreeMap<...>>` so the wrong-shape branches fully
/// drain their `MapAccess`/`SeqAccess` (see module doc).
#[derive(Debug, Clone, PartialEq)]
enum McpServersField {
    Value(BTreeMap<String, CodexEntrySlot>),
    WrongType,
}

impl<'de> Deserialize<'de> for McpServersField {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct FieldVisitor;

        impl<'de> Visitor<'de> for FieldVisitor {
            type Value = McpServersField;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a TOML table, or any other value that degrades to WrongType")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut result = BTreeMap::new();
                while let Some(key) = map.next_key::<String>()? {
                    let entry: CodexEntrySlot = map.next_value()?;
                    result.insert(key, entry);
                }
                Ok(McpServersField::Value(result))
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                while seq.next_element::<IgnoredAny>()?.is_some() {}
                Ok(McpServersField::WrongType)
            }

            fn visit_bool<E>(self, _v: bool) -> Result<Self::Value, E> {
                Ok(McpServersField::WrongType)
            }
            fn visit_i64<E>(self, _v: i64) -> Result<Self::Value, E> {
                Ok(McpServersField::WrongType)
            }
            fn visit_u64<E>(self, _v: u64) -> Result<Self::Value, E> {
                Ok(McpServersField::WrongType)
            }
            fn visit_f64<E>(self, _v: f64) -> Result<Self::Value, E> {
                Ok(McpServersField::WrongType)
            }
            fn visit_str<E>(self, _v: &str) -> Result<Self::Value, E> {
                Ok(McpServersField::WrongType)
            }
        }

        deserializer.deserialize_any(FieldVisitor)
    }
}

/// A per-field-lenient view of one `mcp_servers.<key>` entry. `WrongType`
/// when the entry is present but not a TOML table (design §7.1's "entry
/// present, not a table" row).
#[derive(Debug, Clone, PartialEq)]
enum CodexEntrySlot {
    Value(CodexEntry),
    WrongType,
}

/// One Codex `mcp_servers.<key>` entry. Every field is independently
/// lenient, so one wrong-typed field degrades only that field, never the
/// whole entry or the whole document (design §6.2). `enabled` and
/// `startup_timeout_sec` are never modelled — never read, per design §6.3's
/// never-read list. Unmodelled keys are ignored (CXD §8).
#[derive(Debug, Clone, PartialEq)]
struct CodexEntry {
    command: Option<Lenient<String>>,
    args: Option<LenientArgCount>,
    env: Option<LenientKeyNames>,
    url: Option<Lenient<String>>,
    http_headers: Option<LenientKeyNames>,
}

impl<'de> Deserialize<'de> for CodexEntrySlot {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct EntryVisitor;

        impl<'de> Visitor<'de> for EntryVisitor {
            type Value = CodexEntrySlot;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a TOML table, or any other value that degrades to WrongType")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut command: Option<Lenient<String>> = None;
                let mut args: Option<LenientArgCount> = None;
                let mut env: Option<LenientKeyNames> = None;
                let mut url: Option<Lenient<String>> = None;
                let mut http_headers: Option<LenientKeyNames> = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "command" => command = Some(map.next_value()?),
                        "args" => args = Some(map.next_value()?),
                        "env" => env = Some(map.next_value()?),
                        "url" => url = Some(map.next_value()?),
                        "http_headers" => http_headers = Some(map.next_value()?),
                        _ => {
                            map.next_value::<IgnoredAny>()?;
                        }
                    }
                }

                Ok(CodexEntrySlot::Value(CodexEntry {
                    command,
                    args,
                    env,
                    url,
                    http_headers,
                }))
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                while seq.next_element::<IgnoredAny>()?.is_some() {}
                Ok(CodexEntrySlot::WrongType)
            }

            fn visit_bool<E>(self, _v: bool) -> Result<Self::Value, E> {
                Ok(CodexEntrySlot::WrongType)
            }
            fn visit_i64<E>(self, _v: i64) -> Result<Self::Value, E> {
                Ok(CodexEntrySlot::WrongType)
            }
            fn visit_u64<E>(self, _v: u64) -> Result<Self::Value, E> {
                Ok(CodexEntrySlot::WrongType)
            }
            fn visit_f64<E>(self, _v: f64) -> Result<Self::Value, E> {
                Ok(CodexEntrySlot::WrongType)
            }
            fn visit_str<E>(self, _v: &str) -> Result<Self::Value, E> {
                Ok(CodexEntrySlot::WrongType)
            }
        }

        deserializer.deserialize_any(EntryVisitor)
    }
}

/// A lenient wrapper around [`ArgCount`] (design §6.2): `args` is
/// seq-shaped, so — unlike the genuinely scalar fields that reuse
/// `crate::mcp::Lenient<T>` — this hand-rolled visitor fully drains a
/// wrong-shaped map/seq itself rather than risking a second, un-drained
/// deserialize attempt (see module doc).
#[derive(Debug, Clone, PartialEq, Eq)]
enum LenientArgCount {
    Value(ArgCount),
    WrongType,
}

impl<'de> Deserialize<'de> for LenientArgCount {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ArgCountVisitor;

        impl<'de> Visitor<'de> for ArgCountVisitor {
            type Value = LenientArgCount;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a sequence, or any other value that degrades to WrongType")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut count = 0usize;
                while seq.next_element::<IgnoredAny>()?.is_some() {
                    count = count.saturating_add(1);
                }
                Ok(LenientArgCount::Value(ArgCount(count)))
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                while map.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {}
                Ok(LenientArgCount::WrongType)
            }

            fn visit_bool<E>(self, _v: bool) -> Result<Self::Value, E> {
                Ok(LenientArgCount::WrongType)
            }
            fn visit_i64<E>(self, _v: i64) -> Result<Self::Value, E> {
                Ok(LenientArgCount::WrongType)
            }
            fn visit_u64<E>(self, _v: u64) -> Result<Self::Value, E> {
                Ok(LenientArgCount::WrongType)
            }
            fn visit_f64<E>(self, _v: f64) -> Result<Self::Value, E> {
                Ok(LenientArgCount::WrongType)
            }
            fn visit_str<E>(self, _v: &str) -> Result<Self::Value, E> {
                Ok(LenientArgCount::WrongType)
            }
        }

        deserializer.deserialize_any(ArgCountVisitor)
    }
}

/// A lenient wrapper around [`KeyNames`] (design §6.2): `env`/`http_headers`
/// are map-shaped, mirroring [`LenientArgCount`]'s rationale for `args`.
#[derive(Debug, Clone, PartialEq, Eq)]
enum LenientKeyNames {
    Value(KeyNames),
    WrongType,
}

impl<'de> Deserialize<'de> for LenientKeyNames {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct KeyNamesVisitor;

        impl<'de> Visitor<'de> for KeyNamesVisitor {
            type Value = LenientKeyNames;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a map, or any other value that degrades to WrongType")
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
                Ok(LenientKeyNames::Value(KeyNames(keys)))
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                while seq.next_element::<IgnoredAny>()?.is_some() {}
                Ok(LenientKeyNames::WrongType)
            }

            fn visit_bool<E>(self, _v: bool) -> Result<Self::Value, E> {
                Ok(LenientKeyNames::WrongType)
            }
            fn visit_i64<E>(self, _v: i64) -> Result<Self::Value, E> {
                Ok(LenientKeyNames::WrongType)
            }
            fn visit_u64<E>(self, _v: u64) -> Result<Self::Value, E> {
                Ok(LenientKeyNames::WrongType)
            }
            fn visit_f64<E>(self, _v: f64) -> Result<Self::Value, E> {
                Ok(LenientKeyNames::WrongType)
            }
            fn visit_str<E>(self, _v: &str) -> Result<Self::Value, E> {
                Ok(LenientKeyNames::WrongType)
            }
        }

        deserializer.deserialize_any(KeyNamesVisitor)
    }
}

/// Scan the Codex MCP root under `home`. Infallible. Read-only:
/// `roots::probe`'s `symlink_metadata` (via `roots::codex_mcp_root`) and
/// `std::fs::read_to_string` are the COMPLETE disk surface (CA-16).
pub fn scan(home: &Path) -> McpScan {
    let resolved = roots::codex_mcp_root(home);

    let mut issues = Vec::new();
    let mut components = Vec::new();

    if let Some(scan_path) = resolved.scan_paths.first() {
        read_and_assemble(scan_path, &resolved, &mut components, &mut issues);
    }

    McpScan {
        roots: vec![resolved.root],
        components,
        issues,
    }
}

fn read_and_assemble(
    scan_path: &Path,
    resolved: &roots::ResolvedRoot,
    components: &mut Vec<Component>,
    issues: &mut Vec<ScanIssue>,
) {
    let contents = match std::fs::read_to_string(scan_path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return,
        Err(_) => {
            issues.push(ScanIssue {
                severity: IssueSeverity::Error,
                path: Some(scan_path.to_path_buf()),
                reason: "could not read the Codex MCP configuration".to_string(),
            });
            return;
        }
    };

    let document = match crate::toml::from_str::<CodexDocument>(&contents) {
        Ok(document) => document,
        Err(_) => {
            issues.push(ScanIssue {
                severity: IssueSeverity::Error,
                path: Some(scan_path.to_path_buf()),
                reason: "could not parse the Codex MCP configuration".to_string(),
            });
            return;
        }
    };

    match document.mcp_servers {
        None => {}
        Some(McpServersField::WrongType) => {
            issues.push(ScanIssue {
                severity: IssueSeverity::Warning,
                path: Some(scan_path.to_path_buf()),
                reason: "the \"mcp_servers\" key is not a TOML table; no MCP server was read \
                          from this file"
                    .to_string(),
            });
        }
        Some(McpServersField::Value(map)) => {
            for (key, entry) in map {
                components.push(assemble_component(resolved, &key, entry, scan_path, issues));
            }
        }
    }
}

/// Assemble one `mcp_servers.<key>` entry into a `Component` (design §6.4).
fn assemble_component(
    resolved: &roots::ResolvedRoot,
    key: &str,
    entry: CodexEntrySlot,
    scan_path: &Path,
    issues: &mut Vec<ScanIssue>,
) -> Component {
    let mcp_transport = match entry {
        CodexEntrySlot::WrongType => {
            issues.push(ScanIssue {
                severity: IssueSeverity::Warning,
                path: Some(scan_path.to_path_buf()),
                reason: format!(
                    "MCP server \"{key}\" is not a TOML table; its transport was not read"
                ),
            });
            None
        }
        CodexEntrySlot::Value(codex_entry) => {
            extract_transport(codex_entry, key, scan_path, issues)
        }
    };

    Component {
        id: ComponentId::derive(ComponentKind::Mcp, key),
        name: key.to_string(),
        kind: ComponentKind::Mcp,
        description: None,
        scope: Scope::User,
        locations: vec![Location {
            path: Some(scan_path.to_path_buf()),
            root: resolved.root.id.clone(),
            origin: LocationOrigin::File,
            mcp_transport,
            client: resolved.root.client,
        }],
        provenance_hint: None,
    }
}

/// Extract `command`/`args`/`env` and `url`/`http_headers` from an already
/// per-field-lenient entry, discriminate the transport via the shared
/// matrix (design §6.3), and push at most one discrimination `Warning` plus
/// any independent field-shape `Warning`.
fn extract_transport(
    entry: CodexEntry,
    key: &str,
    scan_path: &Path,
    issues: &mut Vec<ScanIssue>,
) -> Option<McpTransport> {
    let (env_keys, env_wrong_type) = match entry.env {
        None => (Vec::new(), false),
        Some(LenientKeyNames::Value(KeyNames(keys))) => (keys, false),
        Some(LenientKeyNames::WrongType) => (Vec::new(), true),
    };
    let (header_keys, headers_wrong_type) = match entry.http_headers {
        None => (Vec::new(), false),
        Some(LenientKeyNames::Value(KeyNames(keys))) => (keys, false),
        Some(LenientKeyNames::WrongType) => (Vec::new(), true),
    };
    let (arg_count, args_wrong_type) = match entry.args {
        None => (0, false),
        Some(LenientArgCount::Value(ArgCount(count))) => (count, false),
        Some(LenientArgCount::WrongType) => (0, true),
    };

    let command_input = match entry.command {
        None => CommandInput::Absent,
        Some(Lenient::Value(command)) if !command.is_empty() => CommandInput::Usable {
            command,
            arg_count,
            env_keys,
        },
        _ => CommandInput::Unusable,
    };

    let url_input = match entry.url {
        None => UrlInput::Absent,
        Some(Lenient::Value(url)) => match sanitize_url(&url) {
            Some(sanitized) => UrlInput::Valid {
                url: sanitized,
                header_keys,
            },
            None => UrlInput::Unsanitizable,
        },
        Some(Lenient::WrongType) => UrlInput::Unsanitizable,
    };

    let outcome = discriminate_transport(command_input, url_input);

    if let Some(transport_issue) = outcome.issue {
        issues.push(ScanIssue {
            severity: IssueSeverity::Warning,
            path: Some(scan_path.to_path_buf()),
            reason: transport_issue_reason(transport_issue, key),
        });
    }

    if args_wrong_type {
        issues.push(ScanIssue {
            severity: IssueSeverity::Warning,
            path: Some(scan_path.to_path_buf()),
            reason: format!("MCP server \"{key}\" has a non-array argument list"),
        });
    }
    if env_wrong_type {
        issues.push(ScanIssue {
            severity: IssueSeverity::Warning,
            path: Some(scan_path.to_path_buf()),
            reason: format!(
                "MCP server \"{key}\" has a non-object env; its key names were not read"
            ),
        });
    }
    if headers_wrong_type {
        issues.push(ScanIssue {
            severity: IssueSeverity::Warning,
            path: Some(scan_path.to_path_buf()),
            reason: format!(
                "MCP server \"{key}\" has a non-object http_headers; its key names were not read"
            ),
        });
    }

    outcome.transport
}

/// Map a [`TransportIssue`] to its fixed reason string (design §7.1/§7.2).
fn transport_issue_reason(issue: TransportIssue, key: &str) -> String {
    match issue {
        TransportIssue::NeitherCommandNorUrl => {
            format!("MCP server \"{key}\" declares neither a command nor a URL")
        }
        TransportIssue::UrlUnsafe => {
            format!("MCP server \"{key}\" has a URL that could not be reduced to a safe endpoint")
        }
        TransportIssue::BothDeclaredCommandUsed => {
            format!("MCP server \"{key}\" declares both a command and a URL; the command was used")
        }
        TransportIssue::NoReadableCommand => {
            format!("MCP server \"{key}\" has no readable command")
        }
        TransportIssue::NoReadableCommandUrlUsed => {
            format!("MCP server \"{key}\" has no readable command; the URL was used instead")
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    /// The document's `mcp_servers` table deserializes normally when
    /// well-shaped, and each entry's fields degrade independently.
    #[test]
    fn well_shaped_document_deserializes_every_field() {
        let toml = r#"
[mcp_servers.github]
command = "npx"
args = ["-y", "pkg"]

[mcp_servers.github.env]
GITHUB_TOKEN = "ghp_FAKE0000000000000000000000000000000000"
"#;

        let document: CodexDocument = crate::toml::from_str(toml).expect("valid TOML");
        let Some(McpServersField::Value(map)) = document.mcp_servers else {
            panic!("expected mcp_servers to deserialize as a table");
        };
        let Some(CodexEntrySlot::Value(entry)) = map.get("github").cloned() else {
            panic!("expected the github entry to deserialize as a table");
        };

        assert_eq!(entry.command, Some(Lenient::Value("npx".to_string())));
        assert_eq!(entry.args, Some(LenientArgCount::Value(ArgCount(2))));
        assert_eq!(
            entry.env,
            Some(LenientKeyNames::Value(KeyNames(vec![
                "GITHUB_TOKEN".to_string()
            ])))
        );
    }

    /// A wrong-typed `mcp_servers` root degrades to `WrongType`, never a
    /// parse-level `Err` (design §6.2/§7.1), and does not corrupt the rest
    /// of the document parse.
    #[test]
    fn wrong_typed_root_key_degrades_instead_of_failing() {
        let toml = "mcp_servers = \"oops\"\n";

        let document: CodexDocument = crate::toml::from_str(toml).expect("valid TOML syntax");

        assert_eq!(document.mcp_servers, Some(McpServersField::WrongType));
    }

    /// One wrong-typed scalar field inside one entry degrades only that
    /// field — the sibling `github` entry, and the whole document, still
    /// deserialize (design §6.2, the direct regression for §10.4's
    /// `entry-field-wrong-type`).
    #[test]
    fn one_wrong_typed_field_does_not_fail_the_sibling_entry_or_the_document() {
        let toml = r#"
[mcp_servers.broken-field]
command = 42

[mcp_servers.github]
command = "npx"
"#;

        let document: CodexDocument = crate::toml::from_str(toml).expect("valid TOML");
        let Some(McpServersField::Value(map)) = document.mcp_servers else {
            panic!("expected mcp_servers to deserialize as a table");
        };

        let Some(CodexEntrySlot::Value(broken)) = map.get("broken-field").cloned() else {
            panic!("expected broken-field to deserialize as a table");
        };
        assert_eq!(broken.command, Some(Lenient::WrongType));

        let Some(CodexEntrySlot::Value(github)) = map.get("github").cloned() else {
            panic!("expected github to deserialize as a table");
        };
        assert_eq!(github.command, Some(Lenient::Value("npx".to_string())));
    }

    #[test]
    fn empty_args_array_yields_zero_with_no_wrong_type() {
        let toml = "[mcp_servers.github]\ncommand = \"npx\"\nargs = []\n";

        let document: CodexDocument = crate::toml::from_str(toml).expect("valid TOML");
        let Some(McpServersField::Value(map)) = document.mcp_servers else {
            panic!("expected mcp_servers to deserialize as a table");
        };
        let Some(CodexEntrySlot::Value(entry)) = map.get("github").cloned() else {
            panic!("expected github to deserialize as a table");
        };

        assert_eq!(entry.args, Some(LenientArgCount::Value(ArgCount(0))));
    }

    /// Direct regression for the defect found while building this module:
    /// a wrong-typed `args` (a nested table instead of an array) must not
    /// leave the underlying TOML deserializer mid-consumption — the whole
    /// document, including a sibling top-level table, must still parse.
    #[test]
    fn wrong_typed_args_table_does_not_corrupt_the_rest_of_the_document() {
        let toml = r#"
[mcp_servers.github]
command = "npx"

[mcp_servers.github.args]
not = "an-array"

[mcp_servers.other]
command = "codex-other"
"#;

        let document: CodexDocument = crate::toml::from_str(toml).expect("valid TOML");
        let Some(McpServersField::Value(map)) = document.mcp_servers else {
            panic!("expected mcp_servers to deserialize as a table");
        };

        let Some(CodexEntrySlot::Value(github)) = map.get("github").cloned() else {
            panic!("expected github to deserialize as a table");
        };
        assert_eq!(github.args, Some(LenientArgCount::WrongType));

        let Some(CodexEntrySlot::Value(other)) = map.get("other").cloned() else {
            panic!("expected the sibling `other` entry to still deserialize");
        };
        assert_eq!(
            other.command,
            Some(Lenient::Value("codex-other".to_string()))
        );
    }
}
