// @vitest-environment jsdom

import { tick } from "svelte";
import { mount, unmount } from "svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App.svelte";
import type { Component } from "./bindings/Component";
import type { ScanReport } from "./bindings/ScanReport";
import { fetchLogFilePath } from "./lib/appLog";
import { createPrompt, deletePrompt, fetchPrompts, updatePrompt } from "./lib/prompts";
import { fetchFreshness } from "./lib/freshness";
import { rescan, scan } from "./lib/scan";
import { fetchUserSettings, setUserSettings } from "./lib/settings";

vi.mock("./lib/scan", () => ({
  isScanError: (error: unknown) =>
    typeof error === "object" && error !== null && "kind" in error,
  rescan: vi.fn(),
  scan: vi.fn(),
}));

vi.mock("./lib/freshness", () => ({
  fetchFreshness: vi.fn(),
}));

vi.mock("./lib/settings", () => ({
  fetchUserSettings: vi.fn(),
  setUserSettings: vi.fn(),
}));

vi.mock("./lib/prompts", () => ({
  createPrompt: vi.fn(),
  deletePrompt: vi.fn(),
  fetchPrompts: vi.fn(),
  updatePrompt: vi.fn(),
}));

vi.mock("./lib/appLog", () => ({
  fetchLogFilePath: vi.fn(),
}));

const mockedScan = vi.mocked(scan);
const mockedRescan = vi.mocked(rescan);
const mockedFetchFreshness = vi.mocked(fetchFreshness);
const mockedFetchUserSettings = vi.mocked(fetchUserSettings);
const mockedSetUserSettings = vi.mocked(setUserSettings);
const mockedFetchLogFilePath = vi.mocked(fetchLogFilePath);
const mockedFetchPrompts = vi.mocked(fetchPrompts);
const mockedCreatePrompt = vi.mocked(createPrompt);
const mockedUpdatePrompt = vi.mocked(updatePrompt);
const mockedDeletePrompt = vi.mocked(deletePrompt);

function skillFixture(): Component {
  return {
    id: "skill:formatter",
    name: "Formatter",
    kind: "skill",
    description: "Formats source files",
    scope: "user",
    locations: [
      { path: "C:/fixtures/formatter", root: "claude-skills", origin: "file", mcpTransport: null, client: null },
      { path: null, root: "embedded-skills", origin: "embedded", mcpTransport: null, client: null },
    ],
    provenanceHint: null,
  };
}

function agentFixture(): Component {
  return {
    id: "agent:reviewer",
    name: "Reviewer",
    kind: "agent",
    description: "Reviews pull requests",
    scope: "user",
    locations: [
      { path: "C:/fixtures/reviewer", root: "claude-agents", origin: "file", mcpTransport: null, client: null },
      { path: null, root: "embedded-agents", origin: "embedded", mcpTransport: null, client: null },
    ],
    provenanceHint: null,
  };
}

function mcpStdioFixture(): Component {
  return {
    id: "mcp:filesystem",
    name: "Filesystem",
    kind: "mcp",
    description: "Reads and writes files on the local machine",
    scope: "user",
    locations: [
      {
        path: "C:/fixtures/mcp-stdio.json",
        root: "codex-mcp",
        origin: "file",
        mcpTransport: { stdio: { command: "npx", arg_count: 3, env_keys: ["API_TOKEN", "MCP_ROOT"] } },
        client: null,
      },
    ],
    provenanceHint: null,
  };
}

function mcpRemoteFixture(): Component {
  return {
    id: "mcp:search",
    name: "Search",
    kind: "mcp",
    description: "Remote search endpoint",
    scope: "user",
    locations: [
      {
        path: "C:/fixtures/mcp-remote.json",
        root: "opencode-mcp",
        origin: "file",
        mcpTransport: { remote: { url: "https://mcp.example.com", header_keys: ["Authorization"] } },
        client: null,
      },
    ],
    provenanceHint: null,
  };
}

function mcpDegradedFixture(): Component {
  return {
    id: "mcp:broken",
    name: "Broken",
    kind: "mcp",
    description: null,
    scope: "user",
    locations: [
      { path: "C:/fixtures/mcp-broken.json", root: "claude-mcp", origin: "file", mcpTransport: null, client: null },
    ],
    provenanceHint: null,
  };
}

function componentFixtures(kind: "skill" | "agent", count: number): Component[] {
  return Array.from({ length: count }, (_, index) => {
    const number = String(index + 1).padStart(2, "0");
    return {
      id: `${kind}:${number}`,
      name: `${kind === "skill" ? "Skill" : "Agent"} ${number}`,
      kind,
      description: null,
      scope: "user",
      locations: [
        {
          path: `C:/fixtures/${kind}-${number}`,
          root: `claude-${kind}s`,
          origin: "file",
          mcpTransport: null,
          client: null,
        },
      ],
      provenanceHint: null,
    };
  });
}

function reportFixture(components: Component[] = [skillFixture()]): ScanReport {
  return {
    components,
    installations: [],
    rootsScanned: [],
    issues: [],
    clientPresence: null,
    durationMs: 4,
  };
}

function cleanReportFixture(): ScanReport {
  return {
    ...reportFixture([skillFixture(), agentFixture()]),
    rootsScanned: [{ id: "claude-skills", path: "C:/roots/claude", kind: "skill", status: "found", client: null }],
    installations: [{ client: "claudeCode", version: "1.0.0", path: "C:/clients/claude" }],
    clientPresence: [
      {
        slot: "claudeCodeNpm",
        label: "Claude Code CLI (npm)",
        probedPaths: ["C:/clients/claude"],
        status: "detected",
        installations: [{ client: "claudeCode", version: "1.0.0", path: "C:/clients/claude" }],
      },
    ],
    durationMs: 42,
  };
}

function notFoundOnlyReportFixture(): ScanReport {
  return {
    ...reportFixture(),
    rootsScanned: [
      { id: "claude-skills", path: "C:/roots/claude", kind: "skill", status: "notFound", client: null },
    ],
    issues: [],
  };
}

function issuesOnlyReportFixture(): ScanReport {
  return {
    ...reportFixture(),
    rootsScanned: [{ id: "claude-skills", path: "C:/roots/claude", kind: "skill", status: "found", client: null }],
    issues: [
      {
        severity: "error",
        path: "C:/fixtures/broken-skill/SKILL.md",
        reason: "Malformed frontmatter",
      },
    ],
  };
}

function mixedReportFixture(): ScanReport {
  return {
    ...reportFixture(),
    rootsScanned: [
      { id: "claude-skills", path: "C:/roots/claude", kind: "skill", status: "notFound", client: null },
    ],
    issues: [
      { severity: "warning", path: null, reason: "search root claude-skills was not found" },
      {
        severity: "error",
        path: "C:/fixtures/broken-skill/SKILL.md",
        reason: "Malformed frontmatter",
      },
    ],
    clientPresence: [
      {
        slot: "claudeCodeNpm",
        label: "Claude Code CLI (npm)",
        probedPaths: ["C:/Users/example/AppData/Roaming/npm"],
        status: "notDetected",
        installations: [],
      },
    ],
  };
}

function threeRowClientPresenceFixture(): ScanReport {
  return {
    ...reportFixture(),
    rootsScanned: [{ id: "claude-skills", path: "C:/roots/claude", kind: "skill", status: "found", client: null }],
    clientPresence: [
      {
        slot: "claudeCodeNpm",
        label: "Claude Code CLI (npm)",
        probedPaths: ["C:/clients/claude-npm"],
        status: "detected",
        installations: [
          { client: "claudeCode", version: "1.0.0", path: "C:/clients/claude-npm" },
        ],
      },
      {
        slot: "claudeCodeBundled",
        label: "Claude Code (bundled in Claude Desktop)",
        probedPaths: ["C:/clients/pkg-a", "C:/clients/legacy"],
        status: "detected",
        installations: [
          { client: "claudeCode", version: "2.0.0", path: "C:/clients/pkg-a" },
          { client: "claudeCode", version: "2.1.0", path: "C:/clients/legacy" },
        ],
      },
      {
        slot: "openCodeNpm",
        label: "OpenCode (npm)",
        probedPaths: ["C:/clients/opencode-npm"],
        status: "notDetected",
        installations: [],
      },
    ],
  };
}

async function flushApp(): Promise<void> {
  await tick();
  await Promise.resolve();
  await tick();
}


function visibleText(): string {
  return document.body.textContent ?? "";
}

function navigateTo(label: string): void {
  const entry = Array.from(document.querySelectorAll("aside button")).find(
    (candidate) => candidate.textContent?.trim() === label,
  );
  if (entry === undefined) {
    throw new Error(`Sidebar entry was not rendered: ${label}`);
  }
  (entry as HTMLButtonElement).click();
}

function languageSelector(): HTMLSelectElement {
  const selector = document.querySelector<HTMLSelectElement>('select[aria-label="Language"]');
  if (selector === null) {
    throw new Error("Language selector was not rendered");
  }
  return selector;
}

/// Locale-agnostic equivalent of `languageSelector`: the sidebar's language
/// `<select>`'s `aria-label` is itself translated (`Idioma` in Spanish), so
/// it cannot be located by its English text once the app has already
/// mounted in Spanish. Structural instead: it is the sidebar's only
/// `<select>`.
function sidebarLanguageSelector(): HTMLSelectElement {
  const selector = document.querySelector<HTMLSelectElement>("aside select");
  if (selector === null) {
    throw new Error("Language selector was not rendered");
  }
  return selector;
}

describe("App locale switching", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    Object.defineProperty(window.navigator, "languages", {
      configurable: true,
      value: ["en-US"],
    });
    document.documentElement.lang = "";
    document.title = "";
    document.body.innerHTML = "";
    mockedScan.mockResolvedValue(reportFixture());
    mockedFetchUserSettings.mockResolvedValue({ locale: null, enabled: true, disclosureSeen: true });
    mockedFetchFreshness.mockResolvedValue({ enabled: true, checks: [] });
  });

  it("updates visible chrome, document metadata, and avoids rescanning when the selector changes", async () => {
    const app = mount(App, { target: document.body });
    await flushApp();

    expect(mockedScan).toHaveBeenCalledTimes(1);
    expect(mockedRescan).not.toHaveBeenCalled();
    expect(document.documentElement.lang).toBe("en");
    expect(document.title).toBe("Vertice v0.1.0 — Home");
    expect(visibleText()).toContain("Welcome to Vertice");

    navigateTo("Skills");
    await flushApp();

    expect(document.title).toBe("Vertice v0.1.0 — Skills");
    expect(visibleText()).toContain("Language");
    expect(document.querySelector<HTMLInputElement>('input[type="search"]')?.placeholder).toBe(
      "Search by name",
    );
    expect(visibleText()).toContain("Reload");
    expect(visibleText()).not.toContain("Duplicate");
    expect(visibleText()).toContain("Formatter");
    expect(visibleText()).toContain("Formats source files");

    const selector = languageSelector();
    selector.value = "es";
    selector.dispatchEvent(new window.Event("change", { bubbles: true }));
    await flushApp();

    expect(mockedScan).toHaveBeenCalledTimes(1);
    expect(mockedRescan).not.toHaveBeenCalled();
    expect(document.documentElement.lang).toBe("es");
    expect(document.title).toBe("Vertice v0.1.0 — Skills");
    expect(visibleText()).toContain("Idioma");
    expect(document.querySelector<HTMLInputElement>('input[type="search"]')?.placeholder).toBe(
      "Buscar por nombre",
    );
    expect(visibleText()).toContain("Recargar");
    expect(visibleText()).not.toContain("Duplicado");
    expect(
      Array.from(document.querySelectorAll("[title]")).some(
        (element) => element.getAttribute("title") === "Encontrado en 2 ubicaciones",
      ),
    ).toBe(false);
    expect(visibleText()).toContain("Formatter");
    expect(visibleText()).toContain("Formats source files");

    unmount(app);
  });

  it("falls back to the browser locale by default when no initialLocale prop is provided", async () => {
    Object.defineProperty(window.navigator, "languages", {
      configurable: true,
      value: ["es-ES"],
    });

    const app = mount(App, { target: document.body });
    await flushApp();

    expect(document.documentElement.lang).toBe("es");
    expect(visibleText()).toContain("Bienvenido a Vertice");
    expect(sidebarLanguageSelector().value).toBe("es");

    const selector = sidebarLanguageSelector();
    selector.value = "en";
    selector.dispatchEvent(new window.Event("change", { bubbles: true }));
    await flushApp();

    expect(document.documentElement.lang).toBe("en");
    expect(visibleText()).toContain("Welcome to Vertice");

    unmount(app);
  });

  it("mounts with the resolved initialLocale prop: Spanish chrome and documentElement.lang", async () => {
    const app = mount(App, { target: document.body, props: { initialLocale: "es" } });
    await flushApp();

    expect(document.documentElement.lang).toBe("es");
    expect(visibleText()).toContain("Bienvenido a Vertice");
    expect(sidebarLanguageSelector().value).toBe("es");

    unmount(app);
  });

  it("persists only the locale field when the Sidebar language selector changes, never enabled or disclosureSeen", async () => {
    mockedSetUserSettings.mockResolvedValue({ locale: "es", enabled: true, disclosureSeen: true });

    const app = mount(App, { target: document.body });
    await flushApp();

    const selector = languageSelector();
    selector.value = "es";
    selector.dispatchEvent(new window.Event("change", { bubbles: true }));
    await flushApp();

    expect(mockedSetUserSettings).toHaveBeenCalledWith({ locale: "es" });

    unmount(app);
  });

  it("renders localized scan failure chrome with the raw internal reason after loading settles", async () => {
    const rawReason = "ENOENT: raw core diagnostic";
    Object.defineProperty(window.navigator, "languages", {
      configurable: true,
      value: ["es-ES"],
    });
    mockedScan.mockRejectedValue({ kind: "internal", detail: { reason: rawReason } });

    const app = mount(App, { target: document.body });
    await flushApp();
    // Spanish by default: `navigator.languages` is `es-ES` and no
    // `initialLocale` prop was provided, so no selector interaction is
    // needed to reach Spanish chrome.
    navigateTo("Agentes");
    await flushApp();

    const alert = document.querySelector('[role="alert"]');
    expect(mockedScan).toHaveBeenCalledTimes(1);
    expect(mockedRescan).not.toHaveBeenCalled();
    expect(document.querySelector('[role="status"]')).toBeNull();
    expect(alert?.textContent).toContain("Falló el escaneo.");
    expect(alert?.textContent).toContain(`Fallo interno del escaneo: ${rawReason}`);
    expect(alert?.textContent).toContain(rawReason);
    expect(visibleText()).toContain("Idioma");
    expect(visibleText()).not.toContain("Escaneando componentes instalados...");

    unmount(app);
  });

  it("renders a successful empty report as an empty status distinct from failure", async () => {
    mockedScan.mockResolvedValue(reportFixture([]));

    const app = mount(App, { target: document.body });
    await flushApp();
    navigateTo("Skills");
    await flushApp();

    const status = document.querySelector('[role="status"]');
    expect(mockedScan).toHaveBeenCalledTimes(1);
    expect(mockedRescan).not.toHaveBeenCalled();
    expect(document.querySelector('[role="alert"]')).toBeNull();
    expect(status?.textContent).toContain("No components to show.");
    expect(status?.textContent).not.toContain("Scanning for installed components...");
    expect(visibleText()).not.toContain("Scan failed.");

    unmount(app);
  });
});

describe("App per-kind pages", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    Object.defineProperty(window.navigator, "languages", {
      configurable: true,
      value: ["en-US"],
    });
    document.title = "";
    document.body.innerHTML = "";
  });

  it("lists only agents on the Agents route and only skills on the Skills route, scanning once", async () => {
    mockedScan.mockResolvedValue(reportFixture([skillFixture(), agentFixture()]));

    const app = mount(App, { target: document.body });
    await flushApp();

    navigateTo("Agents");
    await flushApp();
    expect(visibleText()).toContain("Reviewer");
    expect(visibleText()).not.toContain("Formatter");

    navigateTo("Skills");
    await flushApp();
    expect(visibleText()).toContain("Formatter");
    expect(visibleText()).not.toContain("Reviewer");

    expect(mockedScan).toHaveBeenCalledTimes(1);
    expect(mockedRescan).not.toHaveBeenCalled();

    unmount(app);
  });

  it("lists only MCP servers on the MCP route and shows no placeholder, scanning once", async () => {
    mockedScan.mockResolvedValue(
      reportFixture([skillFixture(), agentFixture(), mcpStdioFixture(), mcpRemoteFixture()]),
    );

    const app = mount(App, { target: document.body });
    await flushApp();

    navigateTo("MCP");
    await flushApp();

    expect(document.title).toBe("Vertice v0.1.0 — MCP");
    expect(document.querySelector('[data-testid="placeholder-page"]')).toBeNull();
    expect(visibleText()).toContain("Filesystem");
    expect(visibleText()).toContain("Search");
    expect(visibleText()).not.toContain("Formatter");
    expect(visibleText()).not.toContain("Reviewer");

    expect(mockedScan).toHaveBeenCalledTimes(1);
    expect(mockedRescan).not.toHaveBeenCalled();

    unmount(app);
  });

  it("opens the MCP detail with stdio transport: command, argument count, and env key names only", async () => {
    mockedScan.mockResolvedValue(reportFixture([mcpStdioFixture()]));

    const app = mount(App, { target: document.body });
    await flushApp();
    navigateTo("MCP");
    await flushApp();

    const row = Array.from(document.querySelectorAll("button")).find((candidate) =>
      candidate.textContent?.includes("Filesystem"),
    );
    expect(row).toBeDefined();
    (row as HTMLButtonElement).click();
    await flushApp();

    const detail = document.querySelector("section");
    expect(detail?.textContent).toContain("Filesystem");
    expect(detail?.textContent).toContain("C:/fixtures/mcp-stdio.json");
    expect(detail?.textContent).toContain("Stdio");
    expect(detail?.textContent).toContain("npx");
    expect(detail?.textContent).toContain("3 arguments configured");
    expect(detail?.textContent).toContain("Environment key names");
    expect(detail?.textContent).toContain("API_TOKEN");
    expect(detail?.textContent).toContain("MCP_ROOT");
    expect(detail?.textContent).toContain("Names only");

    unmount(app);
  });

  it("opens the MCP detail with remote transport: sanitized endpoint and header key names only", async () => {
    mockedScan.mockResolvedValue(reportFixture([mcpRemoteFixture()]));

    const app = mount(App, { target: document.body });
    await flushApp();
    navigateTo("MCP");
    await flushApp();

    const row = Array.from(document.querySelectorAll("button")).find((candidate) =>
      candidate.textContent?.includes("Search"),
    );
    expect(row).toBeDefined();
    (row as HTMLButtonElement).click();
    await flushApp();

    const detail = document.querySelector("section");
    expect(detail?.textContent).toContain("Search");
    expect(detail?.textContent).toContain("Remote");
    expect(detail?.textContent).toContain("https://mcp.example.com");
    expect(detail?.textContent).toContain("Header key names");
    expect(detail?.textContent).toContain("Authorization");
    expect(detail?.textContent).toContain("Names only");

    unmount(app);
  });

  it("marks a null transport as an un-capturable detail, never as a missing MCP", async () => {
    mockedScan.mockResolvedValue(reportFixture([mcpDegradedFixture()]));

    const app = mount(App, { target: document.body });
    await flushApp();
    navigateTo("MCP");
    await flushApp();

    const row = Array.from(document.querySelectorAll("button")).find((candidate) =>
      candidate.textContent?.includes("Broken"),
    );
    expect(row).toBeDefined();
    (row as HTMLButtonElement).click();
    await flushApp();

    const detail = document.querySelector("section");
    expect(detail?.textContent).toContain("Broken");
    expect(detail?.textContent).toContain(
      "Configured here, but its connection detail could not be safely captured.",
    );
    expect(detail?.textContent).not.toContain("Stdio");
    expect(detail?.textContent).not.toContain("Remote");

    unmount(app);
  });

  it("paginates filtered Skills and Agents, including page size and first/last controls", async () => {
    mockedScan.mockResolvedValue(
      reportFixture([...componentFixtures("skill", 23), ...componentFixtures("agent", 12)]),
    );

    const app = mount(App, { target: document.body });
    await flushApp();

    navigateTo("Skills");
    await flushApp();
    expect(visibleText()).toContain("Showing 1–5 of 23 components");
    expect(visibleText()).toContain("Skill 01");
    expect(visibleText()).not.toContain("Skill 06");

    const lastPage = document.querySelector<HTMLButtonElement>('[aria-label="Go to last page"]');
    lastPage?.click();
    await flushApp();
    expect(visibleText()).toContain("Showing 21–23 of 23 components");
    expect(visibleText()).toContain("Skill 23");
    expect(visibleText()).not.toContain("Skill 01");

    const search = document.querySelector<HTMLInputElement>('input[type="search"]');
    if (search === null) {
      throw new Error("Skill search input was not rendered");
    }
    search.value = "Skill 02";
    search.dispatchEvent(new window.Event("input", { bubbles: true }));
    await flushApp();
    expect(visibleText()).toContain("Showing 1–1 of 1 components");
    expect(visibleText()).toContain("Skill 02");

    search.value = "";
    search.dispatchEvent(new window.Event("input", { bubbles: true }));
    await flushApp();
    expect(visibleText()).toContain("Showing 1–5 of 23 components");

    const pageSize = document.querySelector<HTMLSelectElement>('[aria-label="Components per page"]');
    if (pageSize === null) {
      throw new Error("Page size selector was not rendered");
    }
    expect(Array.from(pageSize.options, (option) => option.value)).toEqual(["5", "10", "15"]);
    pageSize.value = "15";
    pageSize.dispatchEvent(new window.Event("change", { bubbles: true }));
    await flushApp();
    expect(visibleText()).toContain("Showing 1–15 of 23 components");
    expect(visibleText()).toContain("Skill 01");
    expect(visibleText()).toContain("Skill 15");
    expect(visibleText()).not.toContain("Skill 16");

    navigateTo("Agents");
    await flushApp();
    expect(visibleText()).toContain("Showing 1–5 of 12 components");
    expect(visibleText()).toContain("Agent 01");
    expect(visibleText()).not.toContain("Agent 06");

    unmount(app);
  });

  it("keeps Agents, Skills and MCP search queries independent across navigation", async () => {
    mockedScan.mockResolvedValue(
      reportFixture([skillFixture(), agentFixture(), mcpStdioFixture()]),
    );

    const app = mount(App, { target: document.body });
    await flushApp();

    navigateTo("Agents");
    await flushApp();
    const agentsSearch = document.querySelector<HTMLInputElement>('input[type="search"]');
    if (agentsSearch === null) {
      throw new Error("Agents search input was not rendered");
    }
    agentsSearch.value = "nothing-matches";
    agentsSearch.dispatchEvent(new window.Event("input", { bubbles: true }));
    await flushApp();
    expect(visibleText()).not.toContain("Reviewer");

    navigateTo("Home");
    await flushApp();
    navigateTo("Skills");
    await flushApp();
    expect(document.querySelector<HTMLInputElement>('input[type="search"]')?.value).toBe("");
    expect(visibleText()).toContain("Formatter");

    navigateTo("MCP");
    await flushApp();
    expect(document.querySelector<HTMLInputElement>('input[type="search"]')?.value).toBe("");
    expect(visibleText()).toContain("Filesystem");

    navigateTo("Agents");
    await flushApp();
    expect(document.querySelector<HTMLInputElement>('input[type="search"]')?.value).toBe(
      "nothing-matches",
    );
    expect(visibleText()).not.toContain("Reviewer");

    unmount(app);
  });

  it("renders a page-size selector alongside the language selector on either page", async () => {
    mockedScan.mockResolvedValue(
      reportFixture([skillFixture(), agentFixture(), mcpStdioFixture()]),
    );

    const app = mount(App, { target: document.body });
    await flushApp();

    for (const section of ["Agents", "Skills", "MCP"]) {
      navigateTo(section);
      await flushApp();
      const selects = Array.from(document.querySelectorAll("select"));
      expect(selects, section).toHaveLength(2);
      expect(selects.map((select) => select.getAttribute("aria-label")), section).toEqual([
        "Language",
        "Components per page",
      ]);
    }

    unmount(app);
  });

  it("keeps an all-outdated freshness report out of the incident channel and the Home block", async () => {
    // `inventory-ui`: an out-of-date client is NOT a scan incident. The
    // previous change fought to give absence its own carrier; freshness
    // must not smuggle a second meaning back into that channel.
    mockedScan.mockResolvedValue(cleanReportFixture());
    mockedFetchFreshness.mockResolvedValue({
      enabled: true,
      checks: [
        {
          subject: { clientInstallation: { slot: "claudeCodeNpm", path: "C:/clients/claude" } },
          installed: "1.0.0",
          verdict: { outdated: { latest: "9.9.9" } },
        },
      ],
    });

    const app = mount(App, { target: document.body });
    await flushApp();

    // Visit the clients page so the freshness report is actually fetched.
    navigateTo("AI Clients");
    await flushApp();
    await flushApp();
    expect(mockedFetchFreshness).toHaveBeenCalled();

    navigateTo("Agents");
    await flushApp();
    expect(document.querySelector('[data-testid="incident-indicator"]')).toBeNull();

    navigateTo("Skills");
    await flushApp();
    expect(document.querySelector('[data-testid="incident-indicator"]')).toBeNull();

    navigateTo("Home");
    await flushApp();
    expect(visibleText()).not.toContain("The scan failed");

    unmount(app);
  });

  it("shows no incident indicator on either page for a not-found root alone with zero issues (correctness-critical)", async () => {
    mockedScan.mockResolvedValue(notFoundOnlyReportFixture());

    const app = mount(App, { target: document.body });
    await flushApp();

    navigateTo("Agents");
    await flushApp();
    expect(document.querySelector('[data-testid="incident-indicator"]')).toBeNull();

    navigateTo("Skills");
    await flushApp();
    expect(document.querySelector('[data-testid="incident-indicator"]')).toBeNull();

    unmount(app);
  });

  it("shows the incident indicator for non-empty issues and hides it for a fully clean report", async () => {
    mockedScan.mockResolvedValue(issuesOnlyReportFixture());

    const app = mount(App, { target: document.body });
    await flushApp();

    navigateTo("Agents");
    await flushApp();
    expect(document.querySelector('[data-testid="incident-indicator"]')).not.toBeNull();

    navigateTo("Skills");
    await flushApp();
    expect(document.querySelector('[data-testid="incident-indicator"]')).not.toBeNull();

    unmount(app);
    document.body.innerHTML = "";
    mockedScan.mockResolvedValue(cleanReportFixture());
    const cleanApp = mount(App, { target: document.body });
    await flushApp();

    navigateTo("Agents");
    await flushApp();
    expect(document.querySelector('[data-testid="incident-indicator"]')).toBeNull();

    navigateTo("Skills");
    await flushApp();
    expect(document.querySelector('[data-testid="incident-indicator"]')).toBeNull();

    unmount(cleanApp);
  });

  it("re-renders the incident indicator copy in Spanish while component payloads stay verbatim", async () => {
    mockedScan.mockResolvedValue(issuesOnlyReportFixture());

    const app = mount(App, { target: document.body });
    await flushApp();
    navigateTo("Skills");
    await flushApp();

    const selector = languageSelector();
    selector.value = "es";
    selector.dispatchEvent(new window.Event("change", { bubbles: true }));
    await flushApp();

    expect(visibleText()).toContain("incidencias del escaneo");
    expect(visibleText()).toContain("Formatter");

    unmount(app);
  });

  it("marks an embedded location without mistaking a null file path for embedded and localizes chrome", async () => {
    const nullPathFileComponent: Component = {
      ...agentFixture(),
      id: "agent:null-path",
      name: "Null Path Agent",
      locations: [{ path: null, root: "claude-agents", origin: "file", mcpTransport: null, client: null }],
    };
    mockedScan.mockResolvedValue(reportFixture([agentFixture(), nullPathFileComponent]));

    const app = mount(App, { target: document.body });
    await flushApp();
    navigateTo("Agents");
    await flushApp();

    const reviewerButton = Array.from(document.querySelectorAll("button")).find((candidate) =>
      candidate.textContent?.includes("Reviewer"),
    );
    expect(reviewerButton).toBeDefined();
    (reviewerButton as HTMLButtonElement).click();
    await flushApp();

    expect(visibleText()).toContain("Embedded (non-actionable)");
    expect(document.querySelectorAll('[data-testid="embedded-status"]')).toHaveLength(1);

    const selector = languageSelector();
    selector.value = "es";
    selector.dispatchEvent(new window.Event("change", { bubbles: true }));
    await flushApp();

    expect(visibleText()).toContain("Integrado (sin acciones disponibles)");

    unmount(app);
  });

  it("marks an embedded location with a non-null path as non-actionable without rendering an action control", async () => {
    const embeddedPath = "C:/fixtures/embedded/README.md";
    const embeddedComponent: Component = {
      ...skillFixture(),
      id: "skill:embedded-path",
      name: "Embedded Path Skill",
      locations: [{ path: embeddedPath, root: "builtin-skills", origin: "embedded", mcpTransport: null, client: null }],
    };
    mockedScan.mockResolvedValue(reportFixture([embeddedComponent]));

    const app = mount(App, { target: document.body });
    await flushApp();
    navigateTo("Skills");
    await flushApp();

    const skillButton = Array.from(document.querySelectorAll("button")).find((candidate) =>
      candidate.textContent?.includes(embeddedComponent.name),
    );
    expect(skillButton).toBeDefined();
    (skillButton as HTMLButtonElement).click();
    await flushApp();

    const detail = document.querySelector("section");
    expect(detail?.textContent).toContain(embeddedComponent.name);
    expect(detail?.textContent).toContain(embeddedPath);
    expect(detail?.textContent).toContain("Embedded (non-actionable)");
    expect(detail?.querySelector('[data-testid="embedded-status"]')).not.toBeNull();

    unmount(app);
  });
});

describe("App scan route", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    Object.defineProperty(window.navigator, "languages", {
      configurable: true,
      value: ["en-US"],
    });
    document.title = "";
    document.body.innerHTML = "";
    mockedFetchLogFilePath.mockResolvedValue(
      "C:\\Users\\raul\\AppData\\Roaming\\com.vertice.app\\vertice.log",
    );
  });

  it("renders the application log path as selectable text, with no reveal-in-file-manager action, in en and es", async () => {
    mockedScan.mockResolvedValue(cleanReportFixture());
    const logPath = "C:\\Users\\raul\\AppData\\Roaming\\com.vertice.app\\vertice.log";
    mockedFetchLogFilePath.mockResolvedValue(logPath);

    const app = mount(App, { target: document.body });
    await flushApp();
    navigateTo("Scan");
    await flushApp();

    const logPathElement = document.querySelector('[data-testid="log-path"]');
    expect(logPathElement?.tagName.toLowerCase()).toBe("code");
    expect(logPathElement?.textContent).toBe(logPath);
    expect(
      Array.from(document.querySelectorAll("button")).some((button) =>
        /reveal|open|show in/i.test(button.textContent ?? ""),
      ),
    ).toBe(false);

    const selector = languageSelector();
    selector.value = "es";
    selector.dispatchEvent(new window.Event("change", { bubbles: true }));
    await flushApp();

    const logPathElementEs = document.querySelector('[data-testid="log-path"]');
    expect(logPathElementEs?.textContent).toBe(logPath);

    unmount(app);
  });

  it("renders roots, supported clients, duration, and a healthy verdict for a clean report, never a blank panel", async () => {
    mockedScan.mockResolvedValue(cleanReportFixture());

    const app = mount(App, { target: document.body });
    await flushApp();
    navigateTo("Scan");
    await flushApp();

    expect(visibleText()).toContain("Scan completed with no incidents.");
    expect(visibleText()).toContain("C:/roots/claude");
    expect(visibleText()).toContain("Claude Code CLI (npm)");
    expect(visibleText()).toContain("1.0.0");
    const duration = document.querySelector('[data-testid="scan-duration"]');
    expect(duration?.textContent).toContain("Duration");
    expect(duration?.textContent).toContain("42 ms");

    unmount(app);
  });

  it("renders every mixed-report diagnostic exactly once without the duplicate root warning, absence surfaced only in the clients table", async () => {
    mockedScan.mockResolvedValue(mixedReportFixture());

    const app = mount(App, { target: document.body });
    await flushApp();
    navigateTo("Scan");
    await flushApp();

    const diagnostics = document.querySelector('[data-testid="scan-diagnostics"]');
    expect(diagnostics?.textContent).toContain("Malformed frontmatter");
    expect(diagnostics?.textContent).toContain("C:/fixtures/broken-skill/SKILL.md");
    expect(diagnostics?.textContent).not.toContain("Claude Code CLI (npm) not detected");
    expect(diagnostics?.textContent?.match(/search root claude-skills was not found/g)).toBeNull();
    expect(visibleText()).toContain("C:/roots/claude");
    expect(visibleText()).toContain("Not found");
    expect(visibleText()).toContain("Claude Code CLI (npm)");
    expect(visibleText()).toContain("Not detected");

    unmount(app);
  });

  it("renders a three-row supported-clients table including a NotDetected row, without a path column", async () => {
    mockedScan.mockResolvedValue(threeRowClientPresenceFixture());

    const app = mount(App, { target: document.body });
    await flushApp();
    navigateTo("Scan");
    await flushApp();

    expect(visibleText()).toContain("Claude Code CLI (npm)");
    expect(visibleText()).toContain("Claude Code (bundled in Claude Desktop)");
    expect(visibleText()).toContain("OpenCode (npm)");
    expect(visibleText()).toContain("Not detected");
    expect(visibleText()).not.toContain("C:/clients/claude-npm");

    unmount(app);
  });

  it("renders two coexisting versions in one row, each with its own path as a title tooltip", async () => {
    mockedScan.mockResolvedValue(threeRowClientPresenceFixture());

    const app = mount(App, { target: document.body });
    await flushApp();
    navigateTo("Scan");
    await flushApp();

    expect(visibleText()).toContain("2.0.0");
    expect(visibleText()).toContain("2.1.0");
    const pkgA = Array.from(document.querySelectorAll("[title]")).find(
      (el) => el.getAttribute("title") === "C:/clients/pkg-a",
    );
    const legacy = Array.from(document.querySelectorAll("[title]")).find(
      (el) => el.getAttribute("title") === "C:/clients/legacy",
    );
    expect(pkgA).not.toBeUndefined();
    expect(legacy).not.toBeUndefined();

    unmount(app);
  });

  it("renders the unsupported-platform message and no client rows when clientPresence is null", async () => {
    mockedScan.mockResolvedValue(reportFixture());

    const app = mount(App, { target: document.body });
    await flushApp();
    navigateTo("Scan");
    await flushApp();

    expect(visibleText()).toContain(
      "Client installation detection is not supported on this platform.",
    );
    expect(visibleText()).not.toContain("Detected");
    expect(visibleText()).not.toContain("Not detected");

    unmount(app);
  });

  it("invokes rescan from the scan route and disables the control while loading", async () => {
    mockedScan.mockResolvedValue(cleanReportFixture());
    let resolveRescan: (report: ScanReport) => void = () => {};
    mockedRescan.mockImplementation(
      () =>
        new Promise<ScanReport>((resolve) => {
          resolveRescan = resolve;
        }),
    );

    const app = mount(App, { target: document.body });
    await flushApp();
    navigateTo("Scan");
    await flushApp();

    const rescanButton = Array.from(document.querySelectorAll("button")).find(
      (candidate) => candidate.textContent?.trim() === "Reload",
    );
    if (rescanButton === undefined) {
      throw new Error("Rescan button was not rendered");
    }
    rescanButton.click();
    await flushApp();

    expect(mockedRescan).toHaveBeenCalledTimes(1);
    expect(rescanButton.textContent?.trim()).toBe("Reloading...");
    expect(rescanButton.disabled).toBe(true);

    resolveRescan(cleanReportFixture());
    await flushApp();

    expect(rescanButton.disabled).toBe(false);

    unmount(app);
  });

  it("does not render a fake duration before a scan report is available", async () => {
    mockedScan.mockRejectedValue({ kind: "internal", detail: { reason: "boom" } });

    const app = mount(App, { target: document.body });
    await flushApp();
    navigateTo("Scan");
    await flushApp();

    expect(document.querySelector('[data-testid="scan-duration"]')).toBeNull();
    expect(visibleText()).not.toContain("0 ms");

    unmount(app);
  });
});

describe("App shell navigation", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    Object.defineProperty(window.navigator, "languages", {
      configurable: true,
      value: ["en-US"],
    });
    document.title = "";
    document.body.innerHTML = "";
    mockedScan.mockResolvedValue(reportFixture());
    mockedFetchUserSettings.mockResolvedValue({ locale: null, enabled: true, disclosureSeen: true });
    mockedFetchFreshness.mockResolvedValue({ enabled: true, checks: [] });
  });

  it("lands on the greeting page with live counts and every sidebar destination", async () => {
    const app = mount(App, { target: document.body });
    await flushApp();

    const sidebar = document.querySelector("aside");
    const labels = Array.from(sidebar?.querySelectorAll("button") ?? []).map((entry) =>
      entry.textContent?.trim(),
    );

    expect(labels).toEqual([
      "Home",
      "Agents",
      "Skills",
      "MCP",
      "AI Clients",
      "AI Subscriptions",
      "Prompts",
      "Scan",
    ]);
    expect(visibleText()).toContain("Welcome to Vertice");
    const brandMarks = document.querySelectorAll('[data-testid="brand-mark"]');
    expect(brandMarks).toHaveLength(2);
    expect(sidebar?.querySelector('[data-testid="brand-mark"]')?.getAttribute("aria-hidden")).toBe(
      "true",
    );
    expect(sidebar?.querySelector(".brand-gradient")?.textContent?.trim()).not.toBe("V");
    expect(sidebar?.querySelector('[aria-current="page"]')?.textContent?.trim()).toBe("Home");
    expect(document.querySelector('[data-testid="placeholder-page"]')).toBeNull();
    expect(document.querySelector('input[type="search"]')).toBeNull();

    unmount(app);
  });

  it("renders AI client cards with scan detection and empty usage bars", async () => {
    mockedScan.mockResolvedValue(cleanReportFixture());
    const app = mount(App, { target: document.body });
    await flushApp();

    navigateTo("AI Clients");
    await flushApp();

    expect(visibleText()).toContain("Claude Code");
    expect(visibleText()).toContain("OpenCode");
    expect(visibleText()).toContain("Codex");
    expect(visibleText()).toContain("Detected");
    expect(document.querySelectorAll("article")).toHaveLength(3);
    expect(document.querySelectorAll("article .w-0")).toHaveLength(6);

    unmount(app);
  });

  it("renders the Prompts route as a local prompt library", async () => {
    mockedScan.mockResolvedValue(cleanReportFixture());
    mockedFetchPrompts.mockResolvedValue([
      {
        id: "prompt-1",
        title: "Review prompt",
        body: "Explain the tradeoffs.",
        tags: ["review"],
        bestForContext: "Pull requests",
        updatedAt: "2026-08-26T14:00:00Z",
      },
    ]);
    mockedCreatePrompt.mockRejectedValue(new Error("not configured"));
    mockedUpdatePrompt.mockRejectedValue(new Error("not configured"));
    mockedDeletePrompt.mockRejectedValue(new Error("not configured"));

    const app = mount(App, { target: document.body });
    await flushApp();
    navigateTo("Prompts");
    await flushApp();

    expect(document.querySelector('[data-testid="placeholder-page"]')).toBeNull();
    expect(visibleText()).toContain("Review prompt");
    expect(document.title).toBe("Vertice v0.1.0 \u2014 Prompts");

    unmount(app);
  });

  it("navigates from every Home metric tile to its matching destination", async () => {
    mockedScan.mockResolvedValue(cleanReportFixture());

    const app = mount(App, { target: document.body });
    await flushApp();

    for (const [key, title] of [
      ["skills", "Skills"],
      ["agents", "Agents"],
      ["clients", "AI Clients"],
      ["mcps", "MCP"],
    ] as const) {
      const tile = document.querySelector<HTMLButtonElement>(`[data-testid="home-stat-${key}"]`);
      expect(tile, key).not.toBeNull();
      tile?.click();
      await flushApp();
      expect(document.title, key).toBe(`Vertice v0.1.0 — ${title}`);

      navigateTo("Home");
      await flushApp();
    }

    unmount(app);
  });

  it("counts scanned MCPs on the Home hero and no longer offers a scan-roots tile", async () => {
    mockedScan.mockResolvedValue(
      reportFixture([mcpStdioFixture(), mcpRemoteFixture(), mcpDegradedFixture(), skillFixture()]),
    );

    const app = mount(App, { target: document.body });
    await flushApp();

    const mcpsTile = document.querySelector('[data-testid="home-stat-mcps"]');
    expect(mcpsTile).not.toBeNull();
    expect(mcpsTile?.textContent).toContain("MCPs");
    expect(mcpsTile?.textContent).toContain("3");
    expect(document.querySelector('[data-testid="home-stat-roots"]')).toBeNull();

    unmount(app);
  });

  it("opens the agents page from the greeting call to action", async () => {
    mockedScan.mockResolvedValue(reportFixture([agentFixture()]));

    const app = mount(App, { target: document.body });
    await flushApp();

    const cta = Array.from(document.querySelectorAll("main button")).find(
      (candidate) => candidate.textContent?.trim() === "Open agents",
    );
    if (cta === undefined) {
      throw new Error("Greeting call to action was not rendered");
    }
    (cta as HTMLButtonElement).click();
    await flushApp();

    expect(document.title).toBe("Vertice v0.1.0 — Agents");
    expect(visibleText()).toContain("Reviewer");

    unmount(app);
  });

  it("shows the failed state and a retry on Home, never a pending placeholder, and retry invokes rescan", async () => {
    mockedScan.mockRejectedValue({ kind: "internal", detail: { reason: "boom" } });
    mockedRescan.mockResolvedValue(reportFixture());

    const app = mount(App, { target: document.body });
    await flushApp();

    expect(visibleText()).toContain("The scan failed.");
    expect(visibleText()).not.toContain("—");

    const retry = Array.from(document.querySelectorAll("main button")).find(
      (candidate) => candidate.textContent?.trim() === "Retry scan",
    );
    if (retry === undefined) {
      throw new Error("Retry action was not rendered");
    }
    (retry as HTMLButtonElement).click();
    await flushApp();

    expect(mockedRescan).toHaveBeenCalledTimes(1);
    expect(visibleText()).toContain("Welcome to Vertice");

    unmount(app);
  });
});
