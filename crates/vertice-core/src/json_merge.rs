//! Shared `JsonValue` deep merge — ordered fold, last-wins-at-the-leaf.
//!
//! Moved verbatim from `opencode_agents.rs` (design §4.3): the OpenCode MCP
//! root reads the same two files in the same merge order as the OpenCode
//! agent root (V4/C1), so it needs the same semantics. Owned by neither
//! consumer.

use crate::jsonc::JsonValue;

/// Merge an ordered slice of parsed JSON objects into one, last-wins at the
/// leaf (design §6.2). Deliberately an ordered fold over a slice, never a
/// two-named-parameter `merge(base, overlay)` — design §4's escape hatch
/// for a future third input depends on the arity never being hardcoded
/// into the signature.
///
/// A fold over zero inputs yields `None` (no agents, no issue); a fold over
/// one input yields that input unchanged.
pub(crate) fn merge_all(inputs: &[JsonValue]) -> Option<JsonValue> {
    inputs.iter().cloned().reduce(merge_two)
}

/// The recursive deep merge, applied pairwise by [`merge_all`]'s fold
/// (design §6.2). `Object` vs `Object` recurses per key — a key present in
/// only one side survives unchanged. Every other type pairing (array vs
/// anything, scalar vs object, object vs scalar, scalar vs scalar, and an
/// overlay value of `Null`) takes the `else` branch: the overlay replaces
/// the base wholesale. Keys are merged verbatim, never normalized — see
/// `keys_differing_only_by_case_are_not_normalized_before_merging` below.
pub(crate) fn merge_two(base: JsonValue, overlay: JsonValue) -> JsonValue {
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
}
