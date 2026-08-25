// @vitest-environment jsdom

import { tick } from "svelte";
import { mount, unmount } from "svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ClientPresence } from "../../bindings/ClientPresence";
import type { FreshnessReport } from "../../bindings/FreshnessReport";
import type { UserSettings } from "../../bindings/UserSettings";
import type { ScanReport } from "../../bindings/ScanReport";
import { fetchFreshness } from "../freshness";
import { fetchUserSettings, setUserSettings } from "../settings";
import ClientsPage from "./ClientsPageHarness.svelte";

vi.mock("../freshness", () => ({
  fetchFreshness: vi.fn(),
}));

vi.mock("../settings", () => ({
  fetchUserSettings: vi.fn(),
  setUserSettings: vi.fn(),
}));

const mockedFetchFreshness = vi.mocked(fetchFreshness);
const mockedFetchUserSettings = vi.mocked(fetchUserSettings);
const mockedSetUserSettings = vi.mocked(setUserSettings);

function defaultSettings(overrides: Partial<UserSettings> = {}): UserSettings {
  return { locale: null, enabled: true, disclosureSeen: true, ...overrides };
}

function claudeNpmPresence(version: string): ClientPresence {
  return {
    slot: "claudeCodeNpm",
    label: "Claude Code CLI (npm)",
    probedPaths: ["C:/clients/claude-npm"],
    status: "detected",
    installations: [{ client: "claudeCode", version, path: "C:/clients/claude-npm" }],
  };
}

function openCodePresence(version: string): ClientPresence {
  return {
    slot: "openCodeNpm",
    label: "OpenCode (npm)",
    probedPaths: ["C:/clients/opencode-npm"],
    status: "detected",
    installations: [{ client: "openCode", version, path: "C:/clients/opencode-npm" }],
  };
}

function codexPresence(version: string): ClientPresence {
  return {
    slot: "codexStandalone",
    label: "Codex CLI (standalone)",
    probedPaths: ["C:/clients/codex"],
    status: "detected",
    installations: [{ client: "codex", version, path: "C:/clients/codex" }],
  };
}

function reportWithClients(clientPresence: ClientPresence[]): ScanReport {
  return {
    components: [],
    installations: [],
    rootsScanned: [],
    issues: [],
    clientPresence,
    durationMs: 4,
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

describe("ClientsPage freshness badge", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    document.body.innerHTML = "";
    mockedFetchUserSettings.mockResolvedValue(defaultSettings());
  });

  it("shows the pending state before the freshness report resolves", async () => {
    let resolveFreshness: (report: FreshnessReport) => void = () => {};
    mockedFetchFreshness.mockImplementation(
      () =>
        new Promise<FreshnessReport>((resolve) => {
          resolveFreshness = resolve;
        }),
    );

    const app = mount(ClientsPage, {
      target: document.body,
      props: {
        report: reportWithClients([claudeNpmPresence("1.0.0")]),
        status: "ready",
        failureMessage: null,
      },
    });
    await flush();

    expect(visibleText()).toContain("Checking...");

    resolveFreshness({
      enabled: true,
      checks: [
        {
          subject: { clientInstallation: { slot: "claudeCodeNpm", path: "C:/clients/claude-npm" } },
          installed: "1.0.0",
          verdict: "upToDate",
        },
      ],
    });
    await flush();

    expect(visibleText()).toContain("Up to date");
    expect(visibleText()).not.toContain("Checking...");

    unmount(app);
  });

  it("renders up to date, outdated, and unknown as three distinct, non-pending states", async () => {
    mockedFetchFreshness.mockResolvedValue({
      enabled: true,
      checks: [
        {
          subject: { clientInstallation: { slot: "claudeCodeNpm", path: "C:/clients/claude-npm" } },
          installed: "1.0.0",
          verdict: "upToDate",
        },
        {
          subject: { clientInstallation: { slot: "openCodeNpm", path: "C:/clients/opencode-npm" } },
          installed: "1.0.0",
          verdict: { outdated: { latest: "2.1.241" } },
        },
        {
          subject: { clientInstallation: { slot: "codexStandalone", path: "C:/clients/codex" } },
          installed: "some-msix-dir",
          verdict: { unknown: { reason: "no known upstream" } },
        },
      ],
    });

    const app = mount(ClientsPage, {
      target: document.body,
      props: {
        report: reportWithClients([
          claudeNpmPresence("1.0.0"),
          openCodePresence("1.0.0"),
          codexPresence("some-msix-dir"),
        ]),
        status: "ready",
        failureMessage: null,
      },
    });
    await flush();
    await flush();

    expect(visibleText()).toContain("Up to date");
    expect(visibleText()).toContain("Update available: 2.1.241");
    expect(visibleText()).toContain("Unknown");
    expect(visibleText()).not.toContain("Checking...");

    unmount(app);
  });

  it("renders Unknown as a first-class state, never as an alert or failure role", async () => {
    mockedFetchFreshness.mockResolvedValue({
      enabled: true,
      checks: [
        {
          subject: { clientInstallation: { slot: "claudeCodeNpm", path: "C:/clients/claude-npm" } },
          installed: "some-msix-dir",
          verdict: { unknown: { reason: "no known upstream" } },
        },
      ],
    });

    const app = mount(ClientsPage, {
      target: document.body,
      props: {
        report: reportWithClients([claudeNpmPresence("some-msix-dir")]),
        status: "ready",
        failureMessage: null,
      },
    });
    await flush();
    await flush();

    const badge = document.querySelector('[data-testid="freshness-badge"]');
    expect(badge?.textContent).toContain("Unknown");
    expect(document.querySelector('[role="alert"]')).toBeNull();

    unmount(app);
  });

  it("does not fetch or render freshness data while the setting is disabled", async () => {
    mockedFetchUserSettings.mockResolvedValue(defaultSettings({ enabled: false }));

    const app = mount(ClientsPage, {
      target: document.body,
      props: {
        report: reportWithClients([claudeNpmPresence("1.0.0")]),
        status: "ready",
        failureMessage: null,
      },
    });
    await flush();
    await flush();

    expect(mockedFetchFreshness).not.toHaveBeenCalled();
    expect(document.querySelector('[data-testid="freshness-badge"]')).toBeNull();

    unmount(app);
  });

  it("shows the first-run disclosure until dismissed, then persists the acknowledgement", async () => {
    mockedFetchUserSettings.mockResolvedValue(defaultSettings({ disclosureSeen: false }));
    mockedFetchFreshness.mockResolvedValue({ enabled: true, checks: [] });
    mockedSetUserSettings.mockResolvedValue(defaultSettings({ disclosureSeen: true }));

    const app = mount(ClientsPage, {
      target: document.body,
      props: {
        report: reportWithClients([]),
        status: "ready",
        failureMessage: null,
      },
    });
    await flush();
    await flush();

    expect(visibleText()).toContain("Checking for newer versions");

    const dismiss = Array.from(document.querySelectorAll("button")).find(
      (candidate) => candidate.textContent?.trim() === "Got it",
    );
    expect(dismiss).not.toBeUndefined();
    dismiss?.click();
    await flush();

    expect(mockedSetUserSettings).toHaveBeenCalledWith({ disclosureSeen: true });
    expect(visibleText()).not.toContain("Checking for newer versions");

    unmount(app);
  });

  it("re-runs the check when the setting is switched back on, instead of staying pending", async () => {
    mockedFetchFreshness.mockResolvedValue({
      enabled: true,
      checks: [
        {
          subject: { clientInstallation: { slot: "claudeCodeNpm", path: "C:/clients/claude-npm" } },
          installed: "1.0.0",
          verdict: "upToDate",
        },
      ],
    });
    mockedSetUserSettings
      .mockResolvedValueOnce(defaultSettings({ enabled: false }))
      .mockResolvedValueOnce(defaultSettings({ enabled: true }));

    const app = mount(ClientsPage, {
      target: document.body,
      props: {
        report: reportWithClients([claudeNpmPresence("1.0.0")]),
        status: "ready",
        failureMessage: null,
      },
    });
    await flush();
    await flush();
    expect(visibleText()).toContain("Up to date");

    const toggle = document.querySelector<HTMLInputElement>(
      'input[aria-label="Enable version freshness checks"]',
    );

    // Off: the verdict must disappear along with the requests.
    toggle?.click();
    await flush();
    expect(document.querySelector('[data-testid="freshness-badge"]')).toBeNull();

    // Back on: the check must actually run again. Regression — it used to
    // stay stuck on the pending copy until the page was remounted.
    toggle?.click();
    await flush();
    await flush();

    expect(mockedFetchFreshness).toHaveBeenCalledTimes(2);
    expect(visibleText()).toContain("Up to date");
    expect(visibleText()).not.toContain("Checking...");

    unmount(app);
  });

  it("ignores an in-flight response that lands after the check is switched off", async () => {
    let resolveFreshness: (report: FreshnessReport) => void = () => {};
    mockedFetchFreshness.mockImplementation(
      () =>
        new Promise<FreshnessReport>((resolve) => {
          resolveFreshness = resolve;
        }),
    );
    mockedSetUserSettings.mockResolvedValue(defaultSettings({ enabled: false }));

    const app = mount(ClientsPage, {
      target: document.body,
      props: {
        report: reportWithClients([claudeNpmPresence("1.0.0")]),
        status: "ready",
        failureMessage: null,
      },
    });
    await flush();

    const toggle = document.querySelector<HTMLInputElement>(
      'input[aria-label="Enable version freshness checks"]',
    );
    toggle?.click();
    await flush();

    resolveFreshness({
      enabled: true,
      checks: [
        {
          subject: { clientInstallation: { slot: "claudeCodeNpm", path: "C:/clients/claude-npm" } },
          installed: "1.0.0",
          verdict: "upToDate",
        },
      ],
    });
    await flush();

    expect(document.querySelector('[data-testid="freshness-badge"]')).toBeNull();

    unmount(app);
  });

  it("exposes a visible opt-out setting that disables the check on toggle", async () => {
    mockedFetchFreshness.mockResolvedValue({ enabled: true, checks: [] });
    mockedSetUserSettings.mockResolvedValue(defaultSettings({ enabled: false }));

    const app = mount(ClientsPage, {
      target: document.body,
      props: {
        report: reportWithClients([]),
        status: "ready",
        failureMessage: null,
      },
    });
    await flush();
    await flush();

    const toggle = document.querySelector<HTMLInputElement>(
      'input[aria-label="Enable version freshness checks"]',
    );
    expect(toggle).not.toBeNull();
    expect(toggle?.checked).toBe(true);

    toggle?.click();
    await flush();

    expect(mockedSetUserSettings).toHaveBeenCalledWith({ enabled: false });

    unmount(app);
  });
});
