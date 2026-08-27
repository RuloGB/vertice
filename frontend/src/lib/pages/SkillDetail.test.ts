// @vitest-environment jsdom

import { tick } from "svelte";
import { mount, unmount } from "svelte";
import { describe, expect, it } from "vitest";
import type { Component } from "../../bindings/Component";
import type { Location } from "../../bindings/Location";
import SkillDetailHarness from "./SkillDetailHarness.svelte";


function location(root: string, client: Location["client"], path: string | null): Location {
  return {
    path,
    root,
    origin: path === null ? "embedded" : "file",
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

async function renderDetail(component: Component, locale: "en" | "es" = "en"): Promise<ReturnType<typeof mount>> {
  const app = mount(SkillDetailHarness, {
    target: document.body,
    props: { component, locale },
  });
  await tick();
  return app;
}

describe("SkillDetail AI Clients section", () => {
  it("renders client groups with counts instead of the placeholder", async () => {
    const comp = componentWith([
      location("claude-skills", "claudeCode", "/home/user/.claude/skills/test/SKILL.md"),
      location("opencode-skills", "openCode", "/home/user/.config/opencode/skills/test/SKILL.md"),
      location("agents-skills", null, "/home/user/.agents/skills/test/SKILL.md"),
    ]);

    const app = await renderDetail(comp);

    const text = document.body.textContent ?? "";
    expect(text).toContain("Claude Code");
    expect(text).toContain("OpenCode");
    expect(text).toContain("Shared");
    expect(text).not.toContain("No AI clients data available yet.");

    unmount(app);
  });

  it("renders 'Compartido' in Spanish locale for null client", async () => {
    const comp = componentWith([location("agents-skills", null, "/home/user/.agents/skills/test/SKILL.md")]);

    const app = await renderDetail(comp, "es");

    const text = document.body.textContent ?? "";
    expect(text).toContain("Compartido");
    expect(text).not.toContain("Shared");

    unmount(app);
  });

  it("preserves empty state when component has zero locations", async () => {
    const comp = componentWith([]);

    const app = await renderDetail(comp);

    const text = document.body.textContent ?? "";
    expect(text).toContain("No AI clients data available yet.");
    expect(text).not.toContain("Claude Code");
    expect(text).not.toContain("Shared");

    unmount(app);
  });

  it("renders the duplicate badge for shared plus consuming client-specific skill copies", async () => {
    const comp = componentWith([
      location("agents-skills", null, "/home/user/.agents/skills/test/SKILL.md"),
      location("codex-skills", "codex", "/home/user/.codex/skills/test/SKILL.md"),
    ]);

    const app = await renderDetail(comp);

    const text = document.body.textContent ?? "";
    expect(text).toContain("Duplicate");
    expect(text).toContain("/home/user/.agents/skills/test/SKILL.md");
    expect(text).toContain("/home/user/.codex/skills/test/SKILL.md");

    unmount(app);
  });

  it("does not render the duplicate badge for distinct client-specific skill copies", async () => {
    const comp = componentWith([
      location("opencode-skills", "openCode", "/home/user/.config/opencode/skills/test/SKILL.md"),
      location("codex-skills", "codex", "/home/user/.codex/skills/test/SKILL.md"),
    ]);

    const app = await renderDetail(comp);

    const text = document.body.textContent ?? "";
    expect(text).not.toContain("Duplicate");
    expect(text).toContain("/home/user/.config/opencode/skills/test/SKILL.md");
    expect(text).toContain("/home/user/.codex/skills/test/SKILL.md");

    unmount(app);
  });
});