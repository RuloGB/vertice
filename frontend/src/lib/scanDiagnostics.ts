import type { ScanIssue } from "../bindings/ScanIssue";
import type { SearchRoot } from "../bindings/SearchRoot";

export type Diagnostics = {
  unavailableRoots: SearchRoot[];
  missingClientIssues: ScanIssue[];
  remainingRecoverableIssues: ScanIssue[];
};

// These exact warning strings are emitted by the closed Windows client-probe table.
const MISSING_CLIENT_REASONS = new Set([
  "Claude Code (npm) not detected",
  "Claude Code (desktop) not detected",
  "OpenCode (npm) not detected",
]);

export function isMissingClientIssue(issue: ScanIssue): boolean {
  return (
    issue.severity === "warning" &&
    issue.path !== null &&
    MISSING_CLIENT_REASONS.has(issue.reason)
  );
}

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
  const missingClientIssues: ScanIssue[] = [];
  const remainingRecoverableIssues: ScanIssue[] = [];

  for (const issue of issues) {
    if (isMissingClientIssue(issue)) {
      missingClientIssues.push(issue);
    } else if (!isUnavailableRootWarning(issue, unavailableRootWarningReasons)) {
      remainingRecoverableIssues.push(issue);
    }
  }

  return { unavailableRoots, missingClientIssues, remainingRecoverableIssues };
}