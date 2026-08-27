// @vitest-environment jsdom

import { tick } from "svelte";
import { mount, unmount } from "svelte";
import { describe, expect, it } from "vitest";
import type { Component } from "../../bindings/Component";
import type { Location } from "../../bindings/Location";
import AgentDetailHarness from "./AgentDetailHarness.svelte";

function location(client: Location["client"], path: string): Location {
  return {
    path,
    root: "claude-agents",
    origin: "file",
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

describe("AgentDetail AI Clients section", () => {
  it("renders client groups with counts instead of the placeholder", async () => {
    const comp = componentWith([
      location("claudeCode", "/home/user/.claude/agents/test.md"),
      location("claudeCode", "/home/user/.claude/agents/test2.md"),
      location(null, "/home/user/.agents/agents/test.md"),
    ]);

    const app = mount(AgentDetailHarness, {
      target: document.body,
      props: { component: comp },
    });
    await tick();

    const text = document.body.textContent ?? "";
    expect(text).toContain("Claude Code");
    expect(text).toContain("Shared");
    expect(text).not.toContain("No AI clients data available yet.");

    unmount(app);
  });

  it("renders 'Compartido' in Spanish locale for null client", async () => {
    const comp = componentWith([location(null, "/home/user/.agents/agents/test.md")]);

    const app = mount(AgentDetailHarness, {
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

    const app = mount(AgentDetailHarness, {
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
