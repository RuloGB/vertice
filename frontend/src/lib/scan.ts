import { invoke } from "@tauri-apps/api/core";
import type { ScanError } from "../bindings/ScanError";
import type { ScanReport } from "../bindings/ScanReport";

/** Invoke the `scan` command: a full inventory scan of the registered user roots. */
export function scan(): Promise<ScanReport> {
  return invoke<ScanReport>("scan");
}

/**
 * Invoke the `rescan` command. Identical to `scan` today — the core holds no
 * cache or state; kept as a stable IPC entry point for future
 * cache-invalidation semantics.
 */
export function rescan(): Promise<ScanReport> {
  return invoke<ScanReport>("rescan");
}

/** Narrow an untyped command rejection to the generated `ScanError` payload. */
export function isScanError(error: unknown): error is ScanError {
  if (typeof error !== "object" || error === null) {
    return false;
  }
  const candidate = error as { kind?: unknown; detail?: unknown };
  if (candidate.kind === "noRootsConfigured") {
    return true;
  }
  if (candidate.kind === "internal") {
    const detail = candidate.detail as { reason?: unknown } | null | undefined;
    return typeof detail === "object" && detail !== null && typeof detail.reason === "string";
  }
  return false;
}
