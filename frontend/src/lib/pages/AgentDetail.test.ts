// @vitest-environment jsdom

import { tick } from "svelte";
import { mount, unmount } from "svelte";
import { describe, expect, it } from "vitest";
import type { Component } from "../../bindings/Component";
import type { Location } from "../../bindings/Location";
import AgentDetailHarness from "./AgentDetailHarness.svelte";


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
    id: "agent:test-agent",
    name: "test-agent",
    kind: "agent",
    description: null,
    scope: "user",
    locations,
    provenanceHint: null,
  };
}

async function renderDetail(component: Component, locale: "en" | "es" = "en"): Promise<ReturnType<typeof mount>> {
  const app = mount(AgentDetailHarness, {
    target: document.body,
    props: { component, locale },
  });
  await tick();
  return app;
}

describe("AgentDetail AI Clients section", () => {
  it("renders client groups with counts instead of the placeholder", async () => {
    const comp = componentWith([
      location("claude-agents", "claudeCode", "/home/user/.claude/agents/test.md"),
      location("claude-agents", "claudeCode", "/home/user/.claude/agents/test2.md"),
      location("agents-agents", null, "/home/user/.agents/agents/test.md"),
    ]);

    const app = await renderDetail(comp);

    const text = document.body.textContent ?? "";
    expect(text).toContain("Claude Code");
    expect(text).toContain("Shared");
    expect(text).not.toContain("No AI clients data available yet.");

    unmount(app);
  });

  it("renders 'Compartido' in Spanish locale for null client", async () => {
    const comp = componentWith([location("agents-agents", null, "/home/user/.agents/agents/test.md")]);

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

  it("does not render the duplicate badge for shared plus client-specific agent copies", async () => {
    const comp = componentWith([
      location("agents-agents", null, "/home/user/.agents/agents/test.md"),
      location("codex-agents", "codex", "/home/user/.codex/agents/test.md"),
    ]);

    const app = await renderDetail(comp);

    const text = document.body.textContent ?? "";
    expect(text).not.toContain("Duplicate");
    expect(text).toContain("/home/user/.agents/agents/test.md");
    expect(text).toContain("/home/user/.codex/agents/test.md");

    unmount(app);
  });
});