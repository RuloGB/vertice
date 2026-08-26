//! OpenCode MCP adapter (design §5.1, §5.2, §6.1, §6.3, §6.4).
//!
//! Two-file root: `opencode.json` (base) merged with `opencode.jsonc`
//! (overlay), the same order the OpenCode agent root already ships and pins
//! (V4/C1). Root key `mcp`. **`command` is an ARRAY with no separate `args`
//! key** (M9/C3) — `command.first()` is the executable, the remaining
//! length is `arg_count`; elements past index 0 are counted, never
//! inspected. `type` is NEVER read: transport is discriminated structurally
//! (design §6.3).
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

/// Scan the OpenCode MCP root under `home`. Infallible. Read-only:
/// `roots::probe`'s `symlink_metadata` (via `roots::opencode_mcp_root`) and
/// `std::fs::read_to_string` are the COMPLETE disk surface (CA-16).
pub fn scan(home: &Path) -> McpScan {
    let resolved = roots::opencode_mcp_root(home);

    let mut issues = Vec::new();
    let mut surviving: Vec<(usize, BTreeMap<String, JsonValue>)> = Vec::new();

    for (index, path) in resolved.scan_paths.iter().enumerate() {
        if let Some(server_map) = read_mcp_object(path, &mut issues) {
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

/// Read, parse, and extract the `mcp` object from one config file. Fixed
/// reason strings only — no parser error is ever interpolated (design
/// §7.2). Absence — of the file, of the key, or an explicitly empty key —
/// is never a failure.
fn read_mcp_object(
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
                reason: "could not read the OpenCode MCP configuration".to_string(),
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
                reason: "could not parse the OpenCode MCP configuration".to_string(),
            });
            return None;
        }
    };

    let JsonValue::Object(root_map) = parsed else {
        issues.push(ScanIssue {
            severity: IssueSeverity::Error,
            path: Some(path.to_path_buf()),
            reason: "the OpenCode MCP configuration is not a JSON object".to_string(),
        });
        return None;
    };

    match root_map.get("mcp") {
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
                reason: "the \"mcp\" key is not a JSON object; no MCP server was read from this \
                          file"
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

/// Extract `command`/`environment` and `url`/`headers`, discriminate the
/// transport via the shared matrix (design §6.3), and push at most one
/// discrimination `Warning` plus any independent field-shape `Warning`.
fn extract_transport(
    map: &BTreeMap<String, JsonValue>,
    key: &str,
    issue_path: Option<&Path>,
    issues: &mut Vec<ScanIssue>,
) -> Option<crate::model::McpTransport> {
    // A4: OpenCode's stdio environment map key is assumed `environment`
    // (unconfirmed, §0.4) — an absent key yields no keys and no issue.
    let (env_keys, env_wrong_type) = extract_key_names(map, "environment");
    // A5′: the remote header map key is assumed `headers` (unconfirmed).
    let (header_keys, headers_wrong_type) = extract_key_names(map, "headers");

    let (command_input, command_array_wrong_type) = extract_command_input(map, env_keys);

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

    if command_array_wrong_type {
        issues.push(ScanIssue {
            severity: IssueSeverity::Warning,
            path: issue_path.map(Path::to_path_buf),
            reason: format!("MCP server \"{key}\" has a non-array command"),
        });
    }
    if env_wrong_type {
        issues.push(ScanIssue {
            severity: IssueSeverity::Warning,
            path: issue_path.map(Path::to_path_buf),
            reason: format!(
                "MCP server \"{key}\" has a non-object environment; its key names were not read"
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

/// OpenCode's `command` is a JSON ARRAY with no separate `args` key (M9).
/// `command.first()` — via `.first()`, never `[0]` — is the executable;
/// `len - 1` (via `saturating_sub`) is `arg_count`. Elements past index 0
/// are counted, never inspected, including non-string elements. An empty
/// array, or a first element that is not a string, degrades to `Unusable`.
/// Returns `(input, wrong_type)` — `wrong_type` is set only when `command`
/// is present but not an array at all (a shape distinct from "usable"/
/// "unusable", matching the other two clients' field-shape warnings).
fn extract_command_input(
    map: &BTreeMap<String, JsonValue>,
    env_keys: Vec<String>,
) -> (CommandInput, bool) {
    match map.get("command") {
        None => (CommandInput::Absent, false),
        Some(JsonValue::Array(items)) => {
            let arg_count = items.len().saturating_sub(1);
            match items.first() {
                Some(JsonValue::String(command)) if !command.is_empty() => (
                    CommandInput::Usable {
                        command: command.clone(),
                        arg_count,
                        env_keys,
                    },
                    false,
                ),
                _ => (CommandInput::Unusable, false),
            }
        }
        Some(_) => (CommandInput::Unusable, true),
    }
}

/// Extract key NAMES from a map field. Absent is never an issue.
fn extract_key_names(map: &BTreeMap<String, JsonValue>, field: &str) -> (Vec<String>, bool) {
    match map.get(field) {
        None => (Vec::new(), false),
        Some(JsonValue::Object(inner)) => (inner.keys().cloned().collect(), false),
        Some(_) => (Vec::new(), true),
    }
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

    #[test]
    fn array_command_maps_to_first_element_and_tail_length() {
        let mut map = BTreeMap::new();
        map.insert(
            "command".to_string(),
            JsonValue::Array(vec![
                JsonValue::String("npx".to_string()),
                JsonValue::String("-y".to_string()),
                JsonValue::String("pkg".to_string()),
            ]),
        );

        let (input, wrong_type) = extract_command_input(&map, Vec::new());

        assert!(!wrong_type);
        match input {
            CommandInput::Usable {
                command, arg_count, ..
            } => {
                assert_eq!(command, "npx");
                assert_eq!(arg_count, 2);
            }
            other => panic!("expected Usable, got {other:?}"),
        }
    }

    #[test]
    fn empty_command_array_is_unusable() {
        let mut map = BTreeMap::new();
        map.insert("command".to_string(), JsonValue::Array(vec![]));

        let (input, wrong_type) = extract_command_input(&map, Vec::new());

        assert!(!wrong_type);
        assert_eq!(input, CommandInput::Unusable);
    }

    #[test]
    fn non_string_first_element_is_unusable() {
        let mut map = BTreeMap::new();
        map.insert(
            "command".to_string(),
            JsonValue::Array(vec![JsonValue::Bool(true)]),
        );

        let (input, wrong_type) = extract_command_input(&map, Vec::new());

        assert!(!wrong_type);
        assert_eq!(input, CommandInput::Unusable);
    }

    #[test]
    fn non_array_command_is_wrong_type() {
        let mut map = BTreeMap::new();
        map.insert("command".to_string(), JsonValue::String("npx".to_string()));

        let (input, wrong_type) = extract_command_input(&map, Vec::new());

        assert!(wrong_type);
        assert_eq!(input, CommandInput::Unusable);
    }
}
