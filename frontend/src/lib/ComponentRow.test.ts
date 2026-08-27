// @vitest-environment jsdom

import { tick } from "svelte";
import { mount, unmount } from "svelte";
import { describe, expect, it } from "vitest";
import type { Component } from "../bindings/Component";
import type { ComponentKind } from "../bindings/ComponentKind";
import type { Location } from "../bindings/Location";
import ComponentRowHarness from "./ComponentRowHarness.svelte";


function loc(root: string, client: Location["client"], path: string): Location {
  return { path, root, origin: "file", mcpTransport: null, client };
}

function comp(kind: ComponentKind, locations: Location[]): Component {
  return { id: `${kind}:test`, name: "test", kind, description: null, scope: "user", locations, provenanceHint: null };
}

async function render(component: Component, compact = false): Promise<ReturnType<typeof mount>> {
  const app = mount(ComponentRowHarness, { target: document.body, props: { component, compact } });
  await tick();
  return app;
}

describe("ComponentRow duplicate badge", () => {
  it.each([false, true])("renders the duplicate badge for shared plus consuming skill copies (compact: %s)", async (compact) => {
    const app = await render(
      comp("skill", [
        loc("agents-skills", null, "/home/user/.agents/skills/test/SKILL.md"),
        loc("codex-skills", "codex", "/home/user/.codex/skills/test/SKILL.md"),
      ]),
      compact,
    );

    expect(document.body.textContent).toContain("Duplicate");
    unmount(app);
  });

  it.each([false, true])(
    "does not render the duplicate badge for distinct client-specific copies (compact: %s)",
    async (compact) => {
    const app = await render(
      comp("skill", [
        loc("opencode-skills", "openCode", "/home/user/.config/opencode/skills/test/SKILL.md"),
        loc("codex-skills", "codex", "/home/user/.codex/skills/test/SKILL.md"),
      ]),
      compact,
    );

    const text = document.body.textContent ?? "";
    expect(text).not.toContain("Duplicate");
    if (!compact) {
      expect(text).toContain("/home/user/.config/opencode/skills/test/SKILL.md");
      expect(text).toContain("/home/user/.codex/skills/test/SKILL.md");
    }
    unmount(app);
    },
  );
});
