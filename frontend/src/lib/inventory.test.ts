import { describe, expect, it } from "vitest";
import type { Component } from "../bindings/Component";
import type { ComponentKind } from "../bindings/ComponentKind";
import type { Location } from "../bindings/Location";
import { isDuplicate } from "./inventory";

function loc(root: string, client: Location["client"], path: string | null): Location {
  return { path, root, origin: path === null ? "embedded" : "file", mcpTransport: null, client };
}

function comp(kind: ComponentKind, locations: Location[]): Component {
  return { id: `${kind}:fixture`, name: "Fixture", kind, description: null, scope: "user", locations, provenanceHint: null };
}

describe("isDuplicate", () => {
  it.each([
    ["zero locations", comp("skill", [])],
    ["a single file-backed location", comp("skill", [loc("agents-skills", null, "C:/skills/fixture")])],
    ["a single nullable-path location", comp("skill", [loc("agents-skills", null, null)])],
    ["distinct client-specific skill copies", comp("skill", [loc("opencode-skills", "openCode", "C:/opencode/skills/shared-name/SKILL.md"), loc("codex-skills", "codex", "C:/codex/skills/shared-name/SKILL.md")])],
    ["shared-only skill copies", comp("skill", [loc("agents-skills", null, "C:/agents/skills/one/SKILL.md"), loc("agents-skills", null, "C:/agents/skills/two/SKILL.md")])],
    ["an unknown shared root plus a client-specific skill copy", comp("skill", [loc("claude-skills", null, "C:/claude/shared/fixture/SKILL.md"), loc("opencode-skills", "openCode", "C:/opencode/skills/fixture/SKILL.md")])],
    ["unverified Claude shared-skill consumption", comp("skill", [loc("agents-skills", null, "C:/agents/skills/fixture/SKILL.md"), loc("claude-skills", "claudeCode", "C:/claude/skills/fixture/SKILL.md")])],
    ["shared plus client-specific agent copies", comp("agent", [loc("agents-agent", null, "C:/agents/agent/fixture.md"), loc("codex-agent", "codex", "C:/codex/agent/fixture.md")])],
    ["shared plus client-specific MCP copies", comp("mcp", [loc("agents-mcp", null, "C:/agents/mcp/fixture.json"), loc("codex-mcp", "codex", "C:/codex/config.toml")])],
  ])("is false for %s", (_case, component) => {
    expect(isDuplicate(component)).toBe(false);
  });

  it.each([
    ["OpenCode", loc("opencode-skills", "openCode", "C:/opencode/skills/formatter/SKILL.md")],
    ["Codex", loc("codex-skills", "codex", "C:/codex/skills/reviewer/SKILL.md")],
    ["Codex with nullable shared path", loc("codex-skills", "codex", "C:/codex/skills/fixture/SKILL.md")],
  ])("is true for agents-skills plus %s skill copies", (_case, clientLocation) => {
    const sharedPath = _case === "Codex with nullable shared path" ? null : "C:/agents/skills/fixture/SKILL.md";

    expect(isDuplicate(comp("skill", [loc("agents-skills", null, sharedPath), clientLocation]))).toBe(true);
  });
});
