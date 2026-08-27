//! OpenCode agent discovery: entries in the `agent` object of two config
//! files, merged per key — never a directory walk, never a file per
//! component (design §1/§5.3).
//!
//! `scan` is infallible, mirroring [`crate::skills::scan`] and
//! [`crate::agents::scan`]: it takes an already-resolved `home` and returns
//! an owned [`OpenCodeAgentScan`]. Unlike those two adapters, T6 has no
//! `escalate` function (design §5.6) — every `ScanIssue` is constructed at
//! the point where caller context (which file, what was lost) is already
//! in hand.

use std::path::Path;

use crate::jsonc::{self, JsonValue};
use crate::model::{
    Component, ComponentId, ComponentKind, IssueSeverity, Location, LocationOrigin, ScanIssue,
    Scope, SearchRoot,
};
use crate::roots;

/// Owned result of one OpenCode agent scan. A distinct type from
/// `SkillScan` and `AgentScan` — not an alias, not a shared generic
/// (design §5.5).
#[derive(Debug, Clone, PartialEq)]
pub struct OpenCodeAgentScan {
    /// Always exactly one root (design §3).
    pub roots: Vec<SearchRoot>,
    pub components: Vec<Component>,
    pub issues: Vec<ScanIssue>,
}

/// Scan the OpenCode agent config under `home`. Infallible, mirroring
/// `skills::scan` and `agents::scan`. Read-only: `roots::probe`'s
/// `symlink_metadata` (via `roots::opencode_agent_root`) and
/// `std::fs::read_to_string` are the COMPLETE disk surface — no write of
/// any kind, anywhere (CA-16).
pub fn scan(home: &Path) -> OpenCodeAgentScan {
    let resolved = roots::opencode_agent_root(home);

    let mut issues = Vec::new();
    // Index retained (design §5.3): this is what makes per-file provenance
    // available in the component-assembly step below without a second
    // pass. Only the surviving `agent` objects (not the failed ones) are
    // folded, but declaration is checked against these same surviving
    // per-file maps, so a file that failed to parse also loses its
    // provenance credit — it declared nothing, because nothing of it was
    // readable.
    let mut surviving: Vec<(usize, JsonValue)> = Vec::new();

    for (index, path) in resolved.scan_paths.iter().enumerate() {
        if let Some(agent_object) = read_agent_object(path, &mut issues) {
            surviving.push((index, agent_object));
        }
    }

    let values: Vec<JsonValue> = surviving.iter().map(|(_, value)| value.clone()).collect();
    let merged = crate::json_merge::merge_all(&values);

    let mut components = Vec::new();
    if let Some(JsonValue::Object(merged_map)) = merged {
        for (key, entry) in merged_map {
            let declaring_indices: Vec<usize> = surviving
                .iter()
                .filter(|(_, value)| match value {
                    JsonValue::Object(map) => map.contains_key(&key),
                    _ => false,
                })
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

    OpenCodeAgentScan {
        roots: vec![resolved.root],
        components,
        issues,
    }
}

/// Read, parse, and extract the `agent` object from one config file. Each
/// step can fail independently: a missing file, an unreadable file, a
/// parse error, a non-object document root, or an `agent` key that is not
/// an object each produce `None` for this file plus at most one
/// `ScanIssue` (design §8). Never aborts the caller's loop. Returns `None`
/// (no issue) for an absent file, an absent `agent` key, or an empty
/// `agent` object — absence is never a failure.
fn read_agent_object(path: &Path, issues: &mut Vec<ScanIssue>) -> Option<JsonValue> {
    let contents = match std::fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return None,
        Err(err) => {
            issues.push(ScanIssue {
                severity: IssueSeverity::Error,
                path: Some(path.to_path_buf()),
                reason: format!("could not read OpenCode config: {err}"),
            });
            return None;
        }
    };

    let parsed = match jsonc::parse(&contents) {
        Ok(parsed) => parsed,
        Err(err) => {
            issues.push(ScanIssue {
                severity: IssueSeverity::Error,
                path: Some(path.to_path_buf()),
                reason: format!("could not parse OpenCode config: {err}"),
            });
            return None;
        }
    };

    let JsonValue::Object(root_map) = parsed else {
        issues.push(ScanIssue {
            severity: IssueSeverity::Error,
            path: Some(path.to_path_buf()),
            reason: "OpenCode config is not a JSON object".to_string(),
        });
        return None;
    };

    match root_map.get("agent") {
        None => None,
        Some(JsonValue::Object(agent_map)) => {
            if agent_map.is_empty() {
                None
            } else {
                Some(JsonValue::Object(agent_map.clone()))
            }
        }
        Some(_) => {
            issues.push(ScanIssue {
                severity: IssueSeverity::Error,
                path: Some(path.to_path_buf()),
                reason: "the \"agent\" key is not a JSON object".to_string(),
            });
            None
        }
    }
}

/// Assemble one merged agent key into a `Component`, per design §6.4.
/// `id`/`name` derive from the key alone, never from the entry's body or
/// its source file. One `Location` per **declaring** file — `origin:
/// LocationOrigin::File` — ordered by `scan_paths` order via
/// `declaring_indices`, which the caller already computed. A merged
/// agent's value that is not an object, or whose `description` is present
/// but not a string, still produces a `Component` with `description: None`
/// plus a `Warning` (design §8) — its metadata could not be read, but
/// nothing is missing from the inventory.
fn assemble_component(
    resolved: &roots::ResolvedRoot,
    key: &str,
    entry: &JsonValue,
    declaring_indices: &[usize],
    issues: &mut Vec<ScanIssue>,
) -> Component {
    if !matches!(entry, JsonValue::Object(_)) {
        issues.push(ScanIssue {
            severity: IssueSeverity::Warning,
            path: declaring_indices
                .last()
                .and_then(|&i| resolved.scan_paths.get(i).cloned()),
            reason: format!("agent \"{key}\" is not a JSON object; its metadata was not read"),
        });
    }

    let description = match extract_description(entry) {
        DescriptionField::Present(s) => Some(s),
        DescriptionField::Absent => None,
        DescriptionField::WrongType => {
            issues.push(ScanIssue {
                severity: IssueSeverity::Warning,
                path: declaring_indices
                    .last()
                    .and_then(|&i| resolved.scan_paths.get(i).cloned()),
                reason: format!("agent \"{key}\" has a non-string description"),
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
            mcp_transport: None,
            client: resolved.root.client,
        })
        .collect();

    Component {
        id: ComponentId::derive(ComponentKind::Agent, key),
        name: key.to_string(),
        kind: ComponentKind::Agent,
        description,
        scope: Scope::User,
        locations,
        provenance_hint: None,
    }
}

/// Outcome of reading an agent entry's `description` field, value-level
/// (design §5.4). No `#[derive(Deserialize)]` struct describes an OpenCode
/// agent entry anywhere in this module — an entry's existence depends only
/// on its key, never on successfully typing any part of its body.
#[derive(Debug, Clone, PartialEq, Eq)]
enum DescriptionField {
    /// No `description` key at all — not a failure, no issue.
    Absent,
    /// `description` present and a string.
    Present(String),
    /// `description` present but not a string — degrades the field to
    /// `None` on the `Component`, plus a `Warning` (design §8).
    WrongType,
}

/// Extract `description` from an agent entry at the value level:
/// `entry.get("description")` matched against `JsonValue::String` only.
/// Every other field — `mode`, `prompt`, `tools`, `hidden`, `permission`,
/// and anything this capability does not model — is never read (design
/// §5.4). `hidden` in particular is never inspected here or anywhere in
/// this module: its presence or absence cannot change this function's
/// result (spec: "hidden Is Never A Filtering Signal").
fn extract_description(entry: &JsonValue) -> DescriptionField {
    let JsonValue::Object(map) = entry else {
        return DescriptionField::Absent;
    };

    match map.get("description") {
        None => DescriptionField::Absent,
        Some(JsonValue::String(s)) => DescriptionField::Present(s.clone()),
        Some(_) => DescriptionField::WrongType,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn obj(pairs: &[(&str, JsonValue)]) -> JsonValue {
        let mut map = BTreeMap::new();
        for (key, value) in pairs {
            map.insert((*key).to_string(), value.clone());
        }
        JsonValue::Object(map)
    }

    // -- value-level `description` extraction (design §5.4, tasks 2.5-2.6) --

    #[test]
    fn description_absent_yields_absent() {
        let entry = obj(&[("mode", JsonValue::String("subagent".to_string()))]);

        assert_eq!(extract_description(&entry), DescriptionField::Absent);
    }

    #[test]
    fn description_string_yields_present() {
        let entry = obj(&[("description", JsonValue::String("hello".to_string()))]);

        assert_eq!(
            extract_description(&entry),
            DescriptionField::Present("hello".to_string())
        );
    }

    #[test]
    fn description_wrong_type_yields_wrong_type_never_absent() {
        for bad in [
            JsonValue::Number("42".to_string()),
            obj(&[("nested", JsonValue::Bool(true))]),
            JsonValue::Array(vec![]),
            JsonValue::Bool(true),
            JsonValue::Null,
        ] {
            let entry = obj(&[("description", bad)]);

            assert_eq!(extract_description(&entry), DescriptionField::WrongType);
        }
    }

    #[test]
    fn unmodelled_fields_do_not_affect_description_extraction() {
        let entry = obj(&[
            ("description", JsonValue::String("hello".to_string())),
            ("mode", JsonValue::String("subagent".to_string())),
            ("prompt", JsonValue::String("you are an agent".to_string())),
            (
                "tools",
                obj(&[
                    ("read", JsonValue::Bool(true)),
                    ("write", JsonValue::Bool(false)),
                ]),
            ),
            (
                "permission",
                obj(&[("bash", JsonValue::String("ask".to_string()))]),
            ),
        ]);

        assert_eq!(
            extract_description(&entry),
            DescriptionField::Present("hello".to_string())
        );
    }

    /// `hidden Is Never A Filtering Signal` (task 2.7/2.8): an entry with
    /// `hidden: true` extracts identically to the same entry without
    /// `hidden` at all — the field is never read, never matched, never
    /// branched on.
    #[test]
    fn hidden_true_does_not_affect_extraction() {
        let with_hidden = obj(&[
            ("description", JsonValue::String("hello".to_string())),
            ("hidden", JsonValue::Bool(true)),
        ]);
        let without_hidden = obj(&[("description", JsonValue::String("hello".to_string()))]);

        assert_eq!(
            extract_description(&with_hidden),
            extract_description(&without_hidden)
        );
    }

    /// A non-object entry (e.g. a bare string) yields `Absent`, never a
    /// panic — the caller layer is responsible for the accompanying
    /// `Warning` issue (design §8).
    #[test]
    fn non_object_entry_yields_absent_description() {
        let entry = JsonValue::String("not-an-object".to_string());

        assert_eq!(extract_description(&entry), DescriptionField::Absent);
    }
}
