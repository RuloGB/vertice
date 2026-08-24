import { invoke } from "@tauri-apps/api/core";
import type { FreshnessReport } from "../bindings/FreshnessReport";
import type { FreshnessSettings } from "../bindings/FreshnessSettings";

/**
 * Invoke the `freshness` command: an independent check, never awaited by
 * `scan`/`rescan` and never blocking the first render (design §1, §9).
 */
export function fetchFreshness(): Promise<FreshnessReport> {
  return invoke<FreshnessReport>("freshness");
}

/**
 * Invoke the `freshness_settings` command: a read-only view of the
 * persisted opt-out and disclosure-seen state, resolvable before any
 * `freshness` report has ever run.
 */
export function fetchFreshnessSettings(): Promise<FreshnessSettings> {
  return invoke<FreshnessSettings>("freshness_settings");
}

/**
 * Invoke the `set_freshness_settings` command. Always sends the full
 * desired state (both fields) rather than a partial patch, matching the
 * command's read-modify-write contract.
 */
export function setFreshnessSettings(
  enabled: boolean,
  disclosureSeen: boolean,
): Promise<FreshnessSettings> {
  return invoke<FreshnessSettings>("set_freshness_settings", {
    enabled,
    disclosureSeen,
  });
}
