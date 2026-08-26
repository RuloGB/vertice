//! Claude Code MCP adapter (design §5.1, §5.2, §6.1, §6.3, §6.4).
//!
//! Two-file root: `~/.claude.json` (base) merged with
//! `~/.claude/settings.json` (overlay, wins at the leaf — A10). Root key
//! `mcpServers`. `type` is NEVER read: transport is discriminated
//! structurally via [`crate::mcp::discriminate_transport`], the ONLY place
//! that decision is made (design §6.3).
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::collections::BTreeMap;
use std::path::Path;

use crate::jsonc::{self, JsonValue};
pub use crate::mcp::McpScan;
use crate::mcp::{discriminate_transport, sanitize_url, CommandInput, TransportIssue, UrlInput};
use crate::model::{
    Component, ComponentId, ComponentKind, IssueSeverity, Location, LocationOrigin, ScanIssue,
    Scope,
};
use crate::roots;

/// Scan the Claude Code MCP root under `home`. Infallible, mirroring
/// `opencode_agents::scan`. Read-only: `roots::probe`'s `symlink_metadata`
/// (via `roots::claude_mcp_root`) and `std::fs::read_to_string` are the
/// COMPLETE disk surface — no write of any kind, anywhere (CA-16).
pub fn scan(home: &Path) -> McpScan {
    let resolved = roots::claude_mcp_root(home);

    let mut issues = Vec::new();
    let mut surviving: Vec<(usize, BTreeMap<String, JsonValue>)> = Vec::new();

    for (index, path) in resolved.scan_paths.iter().enumerate() {
        if let Some(server_map) = read_mcp_servers_object(path, &mut issues) {
            surviving.push((index, server_map));
        }
    }

    let values: Vec<JsonValue> = surviving
        .iter()
        .map(|(_, map)| JsonValue::Object(map.clone()))
        .collect();
    let merged = crate::json_merge::merge_all(&values);

    let mut components = Vec::new();
    if let Some(JsonValue::Object(merged_map)) = merged {
        for (key, entry) in merged_map {
            let declaring_indices: Vec<usize> = surviving
                .iter()
                .filter(|(_, map)| map.contains_key(&key))
                .map(|(index, _)| *index)
                .collect();

            components.push(assemble_component(
                &resolved,
                &key,
                &entry,
                &declaring_indices,
                &mut issues,
            ));
        }
    }

    McpScan {
        roots: vec![resolved.root],
        components,
        issues,
    }
}

/// Read, parse, and extract the `mcpServers` object from one config file.
/// Every step can fail independently, and every failure produces at most one
/// `ScanIssue` (design §7.1) — reasons are fixed strings, NEVER an
/// interpolated parser error (design §7.2). Absence — of the file, of the
/// key, or an explicitly empty key — is never a failure.
fn read_mcp_servers_object(
    path: &Path,
    issues: &mut Vec<ScanIssue>,
) -> Option<BTreeMap<String, JsonValue>> {
    let contents = match std::fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return None,
        Err(_) => {
            issues.push(ScanIssue {
                severity: IssueSeverity::Error,
                path: Some(path.to_path_buf()),
                reason: "could not read the Claude Code MCP configuration".to_string(),
            });
            return None;
        }
    };

    let parsed = match jsonc::parse(&contents) {
        Ok(parsed) => parsed,
        Err(_) => {
            issues.push(ScanIssue {
                severity: IssueSeverity::Error,
                path: Some(path.to_path_buf()),
                reason: "could not parse the Claude Code MCP configuration".to_string(),
            });
            return None;
        }
    };

    let JsonValue::Object(root_map) = parsed else {
        issues.push(ScanIssue {
            severity: IssueSeverity::Error,
            path: Some(path.to_path_buf()),
            reason: "the Claude Code MCP configuration is not a JSON object".to_string(),
        });
        return None;
    };

    match root_map.get("mcpServers") {
        None => None,
        Some(JsonValue::Object(server_map)) => {
            if server_map.is_empty() {
                None
            } else {
                Some(server_map.clone())
            }
        }
        Some(_) => {
            issues.push(ScanIssue {
                severity: IssueSeverity::Warning,
                path: Some(path.to_path_buf()),
                reason: "the \"mcpServers\" key is not a JSON object; no MCP server was read \
                          from this file"
                    .to_string(),
            });
            None
        }
    }
}

/// Assemble one merged server key into a `Component` (design §6.4). One
/// `Location` per declaring file, all sharing the merged effective
/// transport (design §5.2).
fn assemble_component(
    resolved: &roots::ResolvedRoot,
    key: &str,
    entry: &JsonValue,
    declaring_indices: &[usize],
    issues: &mut Vec<ScanIssue>,
) -> Component {
    let issue_path = declaring_indices
        .last()
        .and_then(|&i| resolved.scan_paths.get(i).cloned());

    let mcp_transport = match entry {
        JsonValue::Object(map) => extract_transport(map, key, issue_path.as_deref(), issues),
        _ => {
            issues.push(ScanIssue {
                severity: IssueSeverity::Warning,
                path: issue_path.clone(),
                reason: format!(
                    "MCP server \"{key}\" is not a JSON object; its transport was not read"
                ),
            });
            None
        }
    };

    let locations = declaring_indices
        .iter()
        .filter_map(|&i| resolved.scan_paths.get(i))
        .map(|path| Location {
            path: Some(path.clone()),
            root: resolved.root.id.clone(),
            origin: LocationOrigin::File,
            mcp_transport: mcp_transport.clone(),
        })
        .collect();

    Component {
        id: ComponentId::derive(ComponentKind::Mcp, key),
        name: key.to_string(),
        kind: ComponentKind::Mcp,
        description: None,
        scope: Scope::User,
        locations,
        provenance_hint: None,
    }
}

/// Extract `command`/`args`/`env` and `url`/`headers`, discriminate the
/// transport via the shared matrix (design §6.3), and push at most one
/// discrimination `Warning` plus any independent field-shape `Warning`s
/// (design §7.1's last three rows).
fn extract_transport(
    map: &BTreeMap<String, JsonValue>,
    key: &str,
    issue_path: Option<&Path>,
    issues: &mut Vec<ScanIssue>,
) -> Option<crate::model::McpTransport> {
    let (env_keys, env_wrong_type) = extract_key_names(map, "env");
    let (arg_count, args_wrong_type) = extract_arg_count(map);
    let (header_keys, headers_wrong_type) = extract_key_names(map, "headers");

    let command_input = match map.get("command") {
        None => CommandInput::Absent,
        Some(JsonValue::String(s)) if !s.is_empty() => CommandInput::Usable {
            command: s.clone(),
            arg_count,
            env_keys,
        },
        _ => CommandInput::Unusable,
    };

    let url_input = match map.get("url") {
        None => UrlInput::Absent,
        Some(JsonValue::String(s)) => match sanitize_url(s) {
            Some(url) => UrlInput::Valid { url, header_keys },
            None => UrlInput::Unsanitizable,
        },
        Some(_) => UrlInput::Unsanitizable,
    };

    let outcome = discriminate_transport(command_input, url_input);

    if let Some(transport_issue) = outcome.issue {
        issues.push(ScanIssue {
            severity: IssueSeverity::Warning,
            path: issue_path.map(Path::to_path_buf),
            reason: transport_issue_reason(transport_issue, key),
        });
    }

    if args_wrong_type {
        issues.push(ScanIssue {
            severity: IssueSeverity::Warning,
            path: issue_path.map(Path::to_path_buf),
            reason: format!("MCP server \"{key}\" has a non-array argument list"),
        });
    }
    if env_wrong_type {
        issues.push(ScanIssue {
            severity: IssueSeverity::Warning,
            path: issue_path.map(Path::to_path_buf),
            reason: format!(
                "MCP server \"{key}\" has a non-object env; its key names were not read"
            ),
        });
    }
    if headers_wrong_type {
        issues.push(ScanIssue {
            severity: IssueSeverity::Warning,
            path: issue_path.map(Path::to_path_buf),
            reason: format!(
                "MCP server \"{key}\" has a non-object headers; its key names were not read"
            ),
        });
    }

    outcome.transport
}

/// Extract key NAMES from a map field. `None`/absent is never an issue;
/// present-but-wrong-typed returns `(vec![], true)` so the caller can raise
/// exactly one `Warning`.
fn extract_key_names(map: &BTreeMap<String, JsonValue>, field: &str) -> (Vec<String>, bool) {
    match map.get(field) {
        None => (Vec::new(), false),
        Some(JsonValue::Object(inner)) => (inner.keys().cloned().collect(), false),
        Some(_) => (Vec::new(), true),
    }
}

/// Extract `args.len()`. Absent ⇒ `0`, no issue. Present but not an array ⇒
/// `(0, true)` so the caller raises exactly one `Warning` (design §6.3's
/// edge rules).
fn extract_arg_count(map: &BTreeMap<String, JsonValue>) -> (usize, bool) {
    match map.get("args") {
        None => (0, false),
        Some(JsonValue::Array(items)) => (items.len(), false),
        Some(_) => (0, true),
    }
}

/// Map a [`TransportIssue`] to its fixed reason string (design §7.1/§7.2 —
/// only the server key and the client label are ever interpolated).
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
    use std::path::PathBuf;

    /// A wrong-typed root key degrades to a `Warning`, never an `Error`
    /// (design §7.1's deliberate divergence from `opencode_agents.rs`).
    /// Reads the committed `root-key-wrong-type` fixture rather than
    /// writing a temp file — this crate is read-only, even in its own
    /// tests (CA-16, pinned by `tests/read_only_audit.rs`).
    #[test]
    fn wrong_typed_root_key_is_a_warning_not_an_error() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests");
        path.push("fixtures");
        path.push("mcp");
        path.push("claude");
        path.push("root-key-wrong-type");
        path.push(".claude.json");

        let mut issues = Vec::new();
        let result = read_mcp_servers_object(&path, &mut issues);

        assert!(result.is_none());
        assert_eq!(issues.len(), 1);
        assert_eq!(
            issues.first().map(|issue| issue.severity),
            Some(IssueSeverity::Warning)
        );
    }

    #[test]
    fn extract_arg_count_absent_is_zero_with_no_issue() {
        let map = BTreeMap::new();

        assert_eq!(extract_arg_count(&map), (0, false));
    }

    #[test]
    fn extract_key_names_absent_yields_no_keys_no_issue() {
        let map = BTreeMap::new();

        assert_eq!(extract_key_names(&map, "env"), (Vec::new(), false));
    }

    #[test]
    fn transport_issue_reason_never_interpolates_beyond_the_key() {
        for issue in [
            TransportIssue::NeitherCommandNorUrl,
            TransportIssue::UrlUnsafe,
            TransportIssue::BothDeclaredCommandUsed,
            TransportIssue::NoReadableCommand,
            TransportIssue::NoReadableCommandUrlUsed,
        ] {
            let reason = transport_issue_reason(issue, "github");
            assert!(reason.contains("github"));
        }
    }
}
