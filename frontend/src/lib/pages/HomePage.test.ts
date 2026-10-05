// @vitest-environment jsdom

import { tick } from "svelte";
import { mount, unmount } from "svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ScanReport } from "../../bindings/ScanReport";
import { collectHomeNotices } from "../notifications";
import { createI18n } from "../i18n/locale.svelte";
import HomePage from "./HomePage.svelte";

vi.mock("../notifications", () => ({
  collectHomeNotices: vi.fn(),
}));

vi.mock("../i18n/locale.svelte", async () => {
  const actual = await vi.importActual<typeof import("../i18n/locale.svelte")>(
    "../i18n/locale.svelte",
  );
  return {
    ...actual,
    useI18n: () => createI18n("en"),
  };
});

vi.mock("../freshness", () => ({
  fetchFreshness: vi.fn(),
}));

vi.mock("../subscriptions", () => ({
  fetchSubscriptions: vi.fn(),
}));

const mockedCollectHomeNotices = vi.mocked(collectHomeNotices);

function emptyReport(): ScanReport {
  return {
    components: [],
    installations: [],
    rootsScanned: [],
    issues: [],
    clientPresence: null,
    durationMs: 0,
  };
}

async function flush(): Promise<void> {
  await tick();
  await Promise.resolve();
  await Promise.resolve();
  await tick();
}

function visibleText(): string {
  return document.body.textContent ?? "";
}

describe("HomePage notifications section", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    document.body.innerHTML = "";
  });

  it("shows the Notifications title when at least one notice exists", async () => {
    mockedCollectHomeNotices.mockResolvedValue([
      { id: "renewal:1", title: "Anthropic", detail: "Pro", badge: "Renews today" },
    ]);

    const app = mount(HomePage, {
      target: document.body,
      props: {
        report: emptyReport(),
        status: "ready",
        failureMessage: null,
        incidents: 0,
        onNavigate: () => {},
        onRetry: () => {},
      },
    });
    await flush();

    expect(visibleText()).toContain("Notifications");
    expect(visibleText()).toContain("Anthropic");
    unmount(app);
  });

  it("hides the section when no notices exist", async () => {
    mockedCollectHomeNotices.mockResolvedValue([]);

    const app = mount(HomePage, {
      target: document.body,
      props: {
        report: emptyReport(),
        status: "ready",
        failureMessage: null,
        incidents: 0,
        onNavigate: () => {},
        onRetry: () => {},
      },
    });
    await flush();

    expect(visibleText()).not.toContain("Notifications");
    unmount(app);
  });

  it("shows notices even when the scan is not ready", async () => {
    mockedCollectHomeNotices.mockResolvedValue([
      { id: "renewal:1", title: "OpenAI", detail: "Plus", badge: "Renews tomorrow" },
    ]);

    const app = mount(HomePage, {
      target: document.body,
      props: {
        report: null,
        status: "loading",
        failureMessage: null,
        incidents: 0,
        onNavigate: () => {},
        onRetry: () => {},
      },
    });
    await flush();

    expect(visibleText()).toContain("Notifications");
    expect(visibleText()).toContain("OpenAI");
    unmount(app);
  });
});
