import { describe, expect, it } from "vitest";
import type { ScanIssue } from "../bindings/ScanIssue";
import type { SearchRoot } from "../bindings/SearchRoot";
import { isMissingClientIssue, partitionDiagnostics } from "./scanDiagnostics";

const clientReasons = [
  "Claude Code (npm) not detected",
  "Claude Code (desktop) not detected",
  "OpenCode (npm) not detected",
] as const;

function issue(overrides: Partial<ScanIssue> = {}): ScanIssue {
  return {
    severity: "warning",
    path: "C:/Users/example/AppData/Roaming/npm",
    reason: clientReasons[0],
    ...overrides,
  };
}

function root(id: string, status: SearchRoot["status"]): SearchRoot {
  return { id, path: `C:/roots/${id}`, kind: "skill", status };
}

describe("isMissingClientIssue", () => {
  it.each(clientReasons)("accepts the exact supported-client warning %s", (reason) => {
    expect(isMissingClientIssue(issue({ reason }))).toBe(true);
  });

  it.each([
    issue({ reason: "Other tool not detected" }),
    issue({ severity: "error" }),
    issue({ path: null }),
  ])("rejects collision outside the closed predicate", (candidate) => {
    expect(isMissingClientIssue(candidate)).toBe(false);
  });
});

describe("partitionDiagnostics", () => {
  it("de-duplicates unavailable roots and preserves unrelated issues", () => {
    const unavailableFirst = root("claude-skills", "notFound");
    const unavailableSecond = root("opencode-skills", "notFound");
    const roots = [unavailableFirst, unavailableSecond, root("copilot-skills", "found")];
    const firstRootWarning = issue({
      path: null,
      reason: "search root claude-skills was not found",
    });
    const secondRootWarning = issue({
      path: null,
      reason: "search root opencode-skills was not found",
    });
    const missingClient = issue({ reason: clientReasons[2] });
    const ordinaryIssue = issue({
      severity: "error",
      path: "C:/fixtures/broken-skill/SKILL.md",
      reason: "Malformed frontmatter",
    });

    expect(partitionDiagnostics(roots, [firstRootWarning, secondRootWarning, missingClient, ordinaryIssue])).toEqual({
      unavailableRoots: [unavailableFirst, unavailableSecond],
      missingClientIssues: [missingClient],
      remainingRecoverableIssues: [ordinaryIssue],
    });
  });

  it("keeps a pathless warning whose reason does not exactly match a missing root", () => {
    const unrelatedWarning = issue({ path: null, reason: "search root claude-skills was not found later" });

    expect(partitionDiagnostics([root("claude-skills", "notFound")], [unrelatedWarning])).toEqual({
      unavailableRoots: [root("claude-skills", "notFound")],
      missingClientIssues: [],
      remainingRecoverableIssues: [unrelatedWarning],
    });
  });
});