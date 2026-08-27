import { describe, expect, it } from "vitest";
import type { Prompt } from "../bindings/Prompt";
import { filterPrompts, normalizePromptSearchTerm } from "./promptSearch";

function prompt(overrides: Partial<Prompt>): Prompt {
  return {
    id: overrides.id ?? "id-1",
    title: overrides.title ?? "Clean Architecture",
    body: overrides.body ?? "Explain ports and adapters.",
    tags: overrides.tags ?? ["architecture"],
    bestForContext: overrides.bestForContext ?? "Code review",
    updatedAt: overrides.updatedAt ?? "2026-08-26T14:00:00Z",
  };
}

describe("promptSearch", () => {
  it("normalizes case whitespace and accents before substring matching", () => {
    expect(normalizePromptSearchTerm("  ÁRQUI  ")).toBe("arqui");
    expect(filterPrompts([prompt({ title: "Arquitectura limpia" })], " arquitectura ")).toHaveLength(1);
  });

  it("matches title tags body and context without reordering the input", () => {
    const prompts = [
      prompt({ id: "title", title: "Daily plan", tags: [], body: "x", bestForContext: null }),
      prompt({ id: "tag", title: "x", tags: ["Daily"], body: "x", bestForContext: null }),
      prompt({ id: "body", title: "x", tags: [], body: "Write a daily summary", bestForContext: null }),
      prompt({ id: "context", title: "x", tags: [], body: "x", bestForContext: "Daily standup" }),
    ];

    expect(filterPrompts(prompts, "daily").map((item) => item.id)).toEqual([
      "title",
      "tag",
      "body",
      "context",
    ]);
  });

  it("returns no result for fuzzy-only matches", () => {
    const prompts = [prompt({ title: "Clean Architecture", tags: ["solid"], body: "ports", bestForContext: "review" })];

    expect(filterPrompts(prompts, "cln arch")).toEqual([]);
  });
});
