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
    let merged = merge_all(&values);

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

/// Merge an ordered slice of parsed `agent` objects into one, last-wins at
/// the leaf (design §6.2). Deliberately an ordered fold over a slice, never
/// a two-named-parameter `merge(base, overlay)` — design §4's escape hatch
/// for a future third input (a legacy `config.json`) depends on the arity
/// never being hardcoded into the signature.
///
/// A fold over zero inputs yields `None` (no agents, no issue); a fold over
/// one input yields that input unchanged.
fn merge_all(inputs: &[JsonValue]) -> Option<JsonValue> {
    inputs.iter().cloned().reduce(merge_two)
}

/// The recursive deep merge, applied pairwise by [`merge_all`]'s fold
/// (design §6.2). `Object` vs `Object` recurses per key — a key present in
/// only one side survives unchanged. Every other type pairing (array vs
/// anything, scalar vs object, object vs scalar, scalar vs scalar, and an
/// overlay value of `Null`) takes the `else` branch: the overlay replaces
/// the base wholesale. Keys are merged verbatim, never normalized — see
/// `keys_differing_only_by_case_are_not_normalized_before_merging` below.
fn merge_two(base: JsonValue, overlay: JsonValue) -> JsonValue {
    match (base, overlay) {
        (JsonValue::Object(mut base_map), JsonValue::Object(overlay_map)) => {
            for (key, overlay_value) in overlay_map {
                match base_map.remove(&key) {
                    Some(base_value) => {
                        base_map.insert(key, merge_two(base_value, overlay_value));
                    }
                    None => {
                        base_map.insert(key, overlay_value);
                    }
                }
            }
            JsonValue::Object(base_map)
        }
        // Array vs anything, scalar vs object, object vs scalar, scalar vs
        // scalar, and `Null` overlay: the overlay replaces the base
        // wholesale. `Null` is a value like any other here — it is never
        // treated as "delete this key" (design §6.2, not RFC 7386).
        (_, overlay) => overlay,
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

    fn get<'a>(value: &'a JsonValue, key: &str) -> Option<&'a JsonValue> {
        match value {
            JsonValue::Object(map) => map.get(key),
            _ => None,
        }
    }

    /// A base-only key survives the fold.
    #[test]
    fn base_only_key_survives() {
        let base = obj(&[("alpha", JsonValue::String("from-base".to_string()))]);

        let merged = merge_all(&[base]).expect("one input must yield that input");

        assert_eq!(
            get(&merged, "alpha"),
            Some(&JsonValue::String("from-base".to_string()))
        );
    }

    /// An overlay-only key survives the fold.
    #[test]
    fn overlay_only_key_survives() {
        let base = obj(&[("alpha", JsonValue::String("from-base".to_string()))]);
        let overlay = obj(&[("beta", JsonValue::String("from-overlay".to_string()))]);

        let merged = merge_all(&[base, overlay]).expect("two inputs must yield a merged value");

        assert_eq!(
            get(&merged, "alpha"),
            Some(&JsonValue::String("from-base".to_string()))
        );
        assert_eq!(
            get(&merged, "beta"),
            Some(&JsonValue::String("from-overlay".to_string()))
        );
    }

    /// The `partial-override` shape (design §6.2, tasks 2.1-2.3): a shared
    /// key with a partial override merges per-field, not per-object. The
    /// base's non-overridden sibling field survives, and a field nested one
    /// level deeper inside a shared nested object also survives.
    ///
    /// **This is the test that MUST fail against the naive stub above.**
    #[test]
    fn shared_key_partial_override_merges_per_field_not_per_object() {
        let base = obj(&[(
            "reviewer",
            obj(&[
                ("description", JsonValue::String("from base".to_string())),
                (
                    "permission",
                    obj(&[
                        ("edit", JsonValue::String("ask".to_string())),
                        ("bash", JsonValue::String("deny".to_string())),
                    ]),
                ),
            ]),
        )]);
        let overlay = obj(&[(
            "reviewer",
            obj(&[(
                "permission",
                obj(&[("edit", JsonValue::String("allow".to_string()))]),
            )]),
        )]);

        let merged = merge_all(&[base, overlay]).expect("two inputs must yield a merged value");
        let reviewer = get(&merged, "reviewer").expect("reviewer key must survive the merge");

        // The base's non-overridden sibling field (`description`) survives.
        assert_eq!(
            get(reviewer, "description"),
            Some(&JsonValue::String("from base".to_string())),
            "the base's non-overridden `description` field must survive a partial override"
        );

        let permission = get(reviewer, "permission").expect("permission object must survive");
        // A field nested one level deeper inside a shared nested object
        // survives from the base...
        assert_eq!(
            get(permission, "bash"),
            Some(&JsonValue::String("deny".to_string())),
            "the base's non-overridden nested `permission.bash` field must survive"
        );
        // ...while the overridden nested leaf takes the overlay's value.
        assert_eq!(
            get(permission, "edit"),
            Some(&JsonValue::String("allow".to_string())),
            "the overlay's nested `permission.edit` override must win"
        );
    }

    /// Array vs anything: overlay replaces wholesale, never concatenated.
    #[test]
    fn array_vs_anything_overlay_replaces_wholesale() {
        let base = obj(&[(
            "key",
            JsonValue::Array(vec![JsonValue::String("a".to_string())]),
        )]);
        let overlay = obj(&[(
            "key",
            JsonValue::Array(vec![JsonValue::String("b".to_string())]),
        )]);

        let merged = merge_all(&[base, overlay]).expect("two inputs must yield a merged value");

        assert_eq!(
            get(&merged, "key"),
            Some(&JsonValue::Array(vec![JsonValue::String("b".to_string())])),
            "arrays must never be concatenated or element-merged"
        );
    }

    /// Scalar vs Object: overlay replaces wholesale.
    #[test]
    fn scalar_vs_object_overlay_replaces() {
        let base = obj(&[("key", JsonValue::String("scalar".to_string()))]);
        let overlay = obj(&[("key", obj(&[("nested", JsonValue::Bool(true))]))]);

        let merged = merge_all(&[base, overlay]).expect("two inputs must yield a merged value");

        assert_eq!(
            get(&merged, "key"),
            Some(&obj(&[("nested", JsonValue::Bool(true))]))
        );
    }

    /// Object vs Scalar: overlay replaces wholesale, the mirror case.
    #[test]
    fn object_vs_scalar_overlay_replaces() {
        let base = obj(&[("key", obj(&[("nested", JsonValue::Bool(true))]))]);
        let overlay = obj(&[("key", JsonValue::String("scalar".to_string()))]);

        let merged = merge_all(&[base, overlay]).expect("two inputs must yield a merged value");

        assert_eq!(
            get(&merged, "key"),
            Some(&JsonValue::String("scalar".to_string()))
        );
    }

    /// Overlay value `Null` replaces and the key SURVIVES with value
    /// `Null` — NOT RFC 7386 "null deletes a key" semantics (design §6.2).
    #[test]
    fn overlay_null_replaces_and_does_not_delete() {
        let base = obj(&[("key", JsonValue::String("present".to_string()))]);
        let overlay = obj(&[("key", JsonValue::Null)]);

        let merged = merge_all(&[base, overlay]).expect("two inputs must yield a merged value");

        assert_eq!(get(&merged, "key"), Some(&JsonValue::Null));
    }

    /// A fold over zero inputs yields nothing.
    #[test]
    fn fold_over_zero_inputs_yields_nothing() {
        let merged = merge_all(&[]);

        assert!(merged.is_none());
    }

    /// A fold over one input yields that input unchanged.
    #[test]
    fn fold_over_one_input_yields_identity() {
        let only = obj(&[("alpha", JsonValue::String("value".to_string()))]);

        let merged =
            merge_all(std::slice::from_ref(&only)).expect("one input must yield that input");

        assert_eq!(merged, only);
    }

    /// Keys differing only by case or Unicode form are treated as distinct
    /// and NOT normalized before merging (design §6.2/§9) — normalization
    /// happens only at the identity layer, after the merge.
    #[test]
    fn keys_differing_only_by_case_are_not_normalized_before_merging() {
        let base = obj(&[("Reviewer", JsonValue::String("upper".to_string()))]);
        let overlay = obj(&[("reviewer", JsonValue::String("lower".to_string()))]);

        let merged = merge_all(&[base, overlay]).expect("two inputs must yield a merged value");

        assert_eq!(
            get(&merged, "Reviewer"),
            Some(&JsonValue::String("upper".to_string()))
        );
        assert_eq!(
            get(&merged, "reviewer"),
            Some(&JsonValue::String("lower".to_string()))
        );
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
