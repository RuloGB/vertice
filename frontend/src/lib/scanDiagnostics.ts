import type { ScanIssue } from "../bindings/ScanIssue";
import type { SearchRoot } from "../bindings/SearchRoot";

export type Diagnostics = {
  unavailableRoots: SearchRoot[];
  recoverableIssues: ScanIssue[];
};

/**
 * Reconstructs the exact reason string emitted by `crates/vertice-core/src/scan.rs`
 * for a `notFound` search root. Known, load-bearing coupling (V5) —
 * out of scope to remove — pinned by `scanDiagnostics.test.ts`. If this
 * string ever drifts from `scan.rs`, a `notFound` root silently re-enters
 * `recoverableIssues` and the incident badge re-lights.
 */
function isUnavailableRootWarning(issue: ScanIssue, reasons: ReadonlySet<string>): boolean {
  return issue.severity === "warning" && issue.path === null && reasons.has(issue.reason);
}

export function partitionDiagnostics(
  roots: SearchRoot[],
  issues: ScanIssue[],
): Diagnostics {
  const unavailableRoots = roots.filter((root) => root.status === "notFound");
  const unavailableRootWarningReasons = new Set(
    unavailableRoots.map((root) => `search root ${root.id} was not found`),
  );
  const recoverableIssues = issues.filter(
    (issue) => !isUnavailableRootWarning(issue, unavailableRootWarningReasons),
  );

  return { unavailableRoots, recoverableIssues };
}

/**
 * Total incident count for the incident indicator and Home's issue count.
 * An incident is exactly "a `ScanIssue` that is not the echo of a
 * `notFound` search root" — a `notFound` root or a `notDetected`/
 * `Detected`-but-unbroken client slot never lights the badge on its own;
 * a broken client's `Error` issue still counts, since it is not a root echo.
 */
export function incidentCount(diagnostics: Diagnostics): number {
  return diagnostics.recoverableIssues.length;
}
