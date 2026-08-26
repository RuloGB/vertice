// @vitest-environment jsdom

import { tick } from "svelte";
import { mount, unmount } from "svelte";
import { describe, expect, it } from "vitest";
import type { Component } from "../../bindings/Component";
import type { Location } from "../../bindings/Location";
import SkillDetailHarness from "./SkillDetailHarness.svelte";

function location(client: Location["client"], path: string): Location {
  return {
    path,
    root: "claude-skills",
    origin: "file",
    mcpTransport: null,
    client,
  };
}

function componentWith(locations: Location[]): Component {
  return {
    id: "skill:test-skill",
    name: "test-skill",
    kind: "skill",
    description: null,
    scope: "user",
    locations,
    provenanceHint: null,
  };
}

describe("SkillDetail AI Clients section", () => {
  it("renders client groups with counts instead of the placeholder", async () => {
    const comp = componentWith([
      location("claudeCode", "/home/user/.claude/skills/test/SKILL.md"),
      location("openCode", "/home/user/.config/opencode/skills/test/SKILL.md"),
      location(null, "/home/user/.agents/skills/test/SKILL.md"),
    ]);

    const app = mount(SkillDetailHarness, {
      target: document.body,
      props: { component: comp },
    });
    await tick();

    const text = document.body.textContent ?? "";
    expect(text).toContain("Claude Code");
    expect(text).toContain("OpenCode");
    expect(text).toContain("Shared");
    expect(text).not.toContain("No AI clients data available yet.");

    unmount(app);
  });

  it("renders 'Compartido' in Spanish locale for null client", async () => {
    const comp = componentWith([location(null, "/home/user/.agents/skills/test/SKILL.md")]);

    const app = mount(SkillDetailHarness, {
      target: document.body,
      props: { component: comp, locale: "es" },
    });
    await tick();

    const text = document.body.textContent ?? "";
    expect(text).toContain("Compartido");
    expect(text).not.toContain("Shared");

    unmount(app);
  });

  it("preserves empty state when component has zero locations", async () => {
    const comp = componentWith([]);

    const app = mount(SkillDetailHarness, {
      target: document.body,
      props: { component: comp },
    });
    await tick();

    const text = document.body.textContent ?? "";
    expect(text).toContain("No AI clients data available yet.");
    expect(text).not.toContain("Claude Code");
    expect(text).not.toContain("Shared");

    unmount(app);
  });
});
