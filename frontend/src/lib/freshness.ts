import { invoke } from "@tauri-apps/api/core";
import type { FreshnessReport } from "../bindings/FreshnessReport";

/**
 * Invoke the `freshness` command: an independent check, never awaited by
 * `scan`/`rescan` and never blocking the first render (design §1, §9).
 */
export function fetchFreshness(): Promise<FreshnessReport> {
  return invoke<FreshnessReport>("freshness");
}
