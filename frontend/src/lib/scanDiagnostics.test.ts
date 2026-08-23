import { describe, expect, it } from "vitest";
import type { ScanIssue } from "../bindings/ScanIssue";
import type { SearchRoot } from "../bindings/SearchRoot";
import { incidentCount, partitionDiagnostics } from "./scanDiagnostics";

function issue(overrides: Partial<ScanIssue> = {}): ScanIssue {
  return {
    severity: "error",
    path: "C:/fixtures/broken-skill/SKILL.md",
    reason: "Malformed frontmatter",
    ...overrides,
  };
}

function root(id: string, status: SearchRoot["status"]): SearchRoot {
  return { id, path: `C:/roots/${id}`, kind: "skill", status };
}

describe("partitionDiagnostics", () => {
  it("de-duplicates unavailable roots and excludes their echo warnings from recoverableIssues", () => {
    const unavailableFirst = root("claude-skills", "notFound");
    const unavailableSecond = root("opencode-skills", "notFound");
    const roots = [unavailableFirst, unavailableSecond, root("copilot-skills", "found")];
    const firstRootWarning = issue({
      severity: "warning",
      path: null,
      reason: "search root claude-skills was not found",
    });
    const secondRootWarning = issue({
      severity: "warning",
      path: null,
      reason: "search root opencode-skills was not found",
    });
    const ordinaryIssue = issue();

    expect(
      partitionDiagnostics(roots, [firstRootWarning, secondRootWarning, ordinaryIssue]),
    ).toEqual({
      unavailableRoots: [unavailableFirst, unavailableSecond],
      recoverableIssues: [ordinaryIssue],
    });
  });

  it("keeps a pathless warning whose reason does not exactly match a missing root", () => {
    const unrelatedWarning = issue({
      severity: "warning",
      path: null,
      reason: "search root claude-skills was not found later",
    });

    expect(partitionDiagnostics([root("claude-skills", "notFound")], [unrelatedWarning])).toEqual(
      {
        unavailableRoots: [root("claude-skills", "notFound")],
        recoverableIssues: [unrelatedWarning],
      },
    );
  });
});

describe("the not-found-root echo suppression (V5, load-bearing coupling)", () => {
  it("excludes the exact root-not-found reason string from incidentCount", () => {
    const rootEcho = issue({
      severity: "warning",
      path: null,
      reason: "search root skills-user was not found",
    });

    expect(
      incidentCount(partitionDiagnostics([root("skills-user", "notFound")], [rootEcho])),
    ).toBe(0);
  });

  it("does not exclude a one-word-drift reason string, proving the match is exact, not fuzzy", () => {
    const drifted = issue({
      severity: "warning",
      path: null,
      reason: "search root skills-user is not found",
    });

    expect(
      incidentCount(partitionDiagnostics([root("skills-user", "notFound")], [drifted])),
    ).toBe(1);
  });
});

describe("incidentCount", () => {
  it("counts zero for a NotDetected client slot alongside a not-found root (absence is never an incident)", () => {
    const roots = [root("claude-skills", "notFound")];
    const rootWarning = issue({
      severity: "warning",
      path: null,
      reason: "search root claude-skills was not found",
    });

    expect(incidentCount(partitionDiagnostics(roots, [rootWarning]))).toBe(0);
  });

  it("counts non-zero for a broken client slot's Error issue", () => {
    const brokenClient = issue({
      severity: "error",
      path: "C:/Users/example/AppData/Roaming/npm",
      reason: "could not read package.json: not found",
    });

    expect(incidentCount(partitionDiagnostics([], [brokenClient]))).toBe(1);
  });

  it("counts zero for a fully clean report", () => {
    expect(incidentCount(partitionDiagnostics([root("claude-skills", "found")], []))).toBe(0);
  });

  it("counts one not-found root's de-duplicated warning plus one real issue as one, not two", () => {
    const roots = [root("claude-skills", "notFound")];
    const rootWarning = issue({
      severity: "warning",
      path: null,
      reason: "search root claude-skills was not found",
    });
    const realIssue = issue();

    expect(incidentCount(partitionDiagnostics(roots, [rootWarning, realIssue]))).toBe(1);
  });
});
