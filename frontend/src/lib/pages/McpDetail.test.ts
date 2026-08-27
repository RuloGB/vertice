// @vitest-environment jsdom

import { tick } from "svelte";
import { mount, unmount } from "svelte";
import { describe, expect, it } from "vitest";
import type { Component } from "../../bindings/Component";
import type { Location } from "../../bindings/Location";
import McpDetailHarness from "./McpDetailHarness.svelte";


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
    id: "mcp:test-server",
    name: "test-server",
    kind: "mcp",
    description: null,
    scope: "user",
    locations,
    provenanceHint: null,
  };
}

async function renderDetail(component: Component, locale: "en" | "es" = "en"): Promise<ReturnType<typeof mount>> {
  const app = mount(McpDetailHarness, {
    target: document.body,
    props: { component, locale },
  });
  await tick();
  return app;
}

describe("McpDetail AI Clients section", () => {
  it("renders client groups with counts instead of the placeholder", async () => {
    const comp = componentWith([
      location("claude-mcp", "claudeCode", "/home/user/.claude.json"),
      location("codex-mcp", "codex", "/home/user/.codex/config.toml"),
    ]);

    const app = await renderDetail(comp);

    const text = document.body.textContent ?? "";
    expect(text).toContain("Claude Code");
    expect(text).toContain("Codex");
    expect(text).not.toContain("No AI clients data available yet.");

    unmount(app);
  });

  it("renders 'Compartido' in Spanish locale for null client", async () => {
    const comp = componentWith([location("agents-mcp", null, "/home/user/.agents/mcp/test.json")]);

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

  it("does not render the duplicate badge for shared plus client-specific MCP copies", async () => {
    const comp = componentWith([
      location("agents-mcp", null, "/home/user/.agents/mcp/test.json"),
      location("codex-mcp", "codex", "/home/user/.codex/config.toml"),
    ]);

    const app = await renderDetail(comp);

    const text = document.body.textContent ?? "";
    expect(text).not.toContain("Duplicate");
    expect(text).toContain("/home/user/.agents/mcp/test.json");
    expect(text).toContain("/home/user/.codex/config.toml");

    unmount(app);
  });

  it("renders a nullable location path safely without a duplicate badge", async () => {
    const comp = componentWith([
      location("agents-mcp", null, null),
      location("codex-mcp", "codex", "/home/user/.codex/config.toml"),
    ]);

    const app = await renderDetail(comp);

    const text = document.body.textContent ?? "";
    expect(text).toContain("(no path on disk)");
    expect(text).toContain("/home/user/.codex/config.toml");
    expect(text).not.toContain("Duplicate");

    unmount(app);
  });
});