// @vitest-environment jsdom

import { tick } from "svelte";
import { mount, unmount } from "svelte";
import { beforeEach, describe, expect, it } from "vitest";
import type { Component } from "../../bindings/Component";
import type { ScanReport } from "../../bindings/ScanReport";
import SkillsPage from "./SkillsPageHarness.svelte";

function skill(index: number): Component {
  return {
    id: `skill:${index}`,
    kind: "skill",
    name: `Skill ${index}`,
    description: null,
    scope: "user",
    locations: [],
    provenanceHint: null,
  };
}

function reportFixture(): ScanReport {
  return {
    components: Array.from({ length: 12 }, (_, index) => skill(index + 1)),
    installations: [],
    rootsScanned: [],
    issues: [],
    clientPresence: [],
    durationMs: 0,
  };
}

async function flush(): Promise<void> {
  await tick();
  await tick();
}

beforeEach(() => {
  document.body.innerHTML = "";
  localStorage.clear();
});

describe("SkillsPage", () => {
  it("keeps the selected list page when returning from a detail page", async () => {
    const app = mount(SkillsPage, {
      target: document.body,
      props: {
        status: "ready",
        report: reportFixture(),
        failureMessage: null,
        query: "",
        incidents: 0,
        onQueryChange: () => {},
        onReload: () => {},
        onNavigate: () => {},
      },
    });
    await flush();

    const pageSize = document.querySelector<HTMLSelectElement>('select[aria-label="Components per page"]');
    expect(pageSize).not.toBeNull();
    pageSize!.value = "10";
    pageSize!.dispatchEvent(new Event("change", { bubbles: true }));
    document.querySelector<HTMLButtonElement>('button[aria-label="Go to next page"]')?.click();
    await flush();

    expect(document.body.textContent).toContain("Page 2 of 2");
    Array.from(document.querySelectorAll<HTMLButtonElement>("button"))
      .find((button) => button.textContent?.trim() === "Skill 11")
      ?.click();
    await flush();
    Array.from(document.querySelectorAll<HTMLButtonElement>("button"))
      .find((button) => button.textContent?.includes("Back to list"))
      ?.click();
    await flush();

    expect(document.body.textContent).toContain("Page 2 of 2");
    expect(document.body.textContent).toContain("Skill 11");
    unmount(app);
  });
});
