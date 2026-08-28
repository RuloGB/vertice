// @vitest-environment jsdom

import { tick } from "svelte";
import { mount, unmount } from "svelte";
import { beforeEach, describe, expect, it } from "vitest";
import type { Component } from "../../bindings/Component";
import type { ComponentKind } from "../../bindings/ComponentKind";
import type { ScanReport } from "../../bindings/ScanReport";
import ComponentListPage from "./ComponentListPageHarness.svelte";

type ListRoute = "agents" | "skills" | "mcp";

function component(kind: ComponentKind, index: number): Component {
  const label = kind === "mcp" ? "MCP" : `${kind[0].toUpperCase()}${kind.slice(1)}`;
  return {
    id: `${kind}:${index}`,
    kind,
    name: `${label} ${index}`,
    description: null,
    scope: "user",
    locations: [],
    provenanceHint: null,
  };
}

function reportFixture(kind: ComponentKind): ScanReport {
  return {
    components: Array.from({ length: 12 }, (_, index) => component(kind, index + 1)),
    installations: [],
    rootsScanned: [],
    issues: [],
    clientPresence: [],
    durationMs: 0,
  };
}

function componentKind(route: ListRoute): ComponentKind {
  return route === "agents" ? "agent" : route === "skills" ? "skill" : "mcp";
}

function pageProps(route: ListRoute) {
  return {
    route,
    status: "ready" as const,
    report: reportFixture(componentKind(route)),
    failureMessage: null,
    query: "",
    incidents: 0,
    onQueryChange: () => {},
    onReload: () => {},
    onNavigate: () => {},
  };
}

async function flush(): Promise<void> {
  await tick();
  await tick();
}

function pageSizeControl(): HTMLSelectElement {
  const control = document.querySelector<HTMLSelectElement>('select[aria-label="Components per page"]');
  expect(control).not.toBeNull();
  return control!;
}

function clickButtonContaining(text: string): void {
  const button = Array.from(document.querySelectorAll<HTMLButtonElement>("button")).find((candidate) =>
    candidate.textContent?.includes(text),
  );
  expect(button).toBeDefined();
  button!.click();
}

beforeEach(() => {
  document.body.innerHTML = "";
  localStorage.clear();
});

describe("component list preferences", () => {
  for (const route of ["agents", "skills", "mcp"] as const) {
    it(`restores the ${route} page size after remounting`, async () => {
      const first = mount(ComponentListPage, { target: document.body, props: pageProps(route) });
      await flush();

      const control = pageSizeControl();
      control.value = "15";
      control.dispatchEvent(new Event("change", { bubbles: true }));
      await flush();
      unmount(first);

      const second = mount(ComponentListPage, { target: document.body, props: pageProps(route) });
      await flush();

      expect(pageSizeControl().value).toBe("15");
      unmount(second);
    });

    it(`restores the ${route} page after returning from its detail`, async () => {
      const app = mount(ComponentListPage, { target: document.body, props: pageProps(route) });
      await flush();

      const control = pageSizeControl();
      control.value = "10";
      control.dispatchEvent(new Event("change", { bubbles: true }));
      document.querySelector<HTMLButtonElement>('button[aria-label="Go to next page"]')?.click();
      await flush();

      const kind = componentKind(route);
      const label = kind === "mcp" ? "MCP" : `${kind[0].toUpperCase()}${kind.slice(1)}`;
      clickButtonContaining(`${label} 11`);
      await flush();
      clickButtonContaining("Back to");
      await flush();

      expect(document.body.textContent).toContain("Page 2 of 2");
      expect(document.body.textContent).toContain(`${label} 11`);
      unmount(app);
    });
  }
});
