// @vitest-environment jsdom

import { tick } from "svelte";
import { mount, unmount } from "svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App.svelte";
import type { Component } from "./bindings/Component";
import type { ScanReport } from "./bindings/ScanReport";
import { rescan, scan } from "./lib/scan";
import { SAMPLE_SUBSCRIPTIONS } from "./lib/subscriptions";

vi.mock("./lib/scan", () => ({
  isScanError: (error: unknown) =>
    typeof error === "object" && error !== null && "kind" in error,
  rescan: vi.fn(),
  scan: vi.fn(),
}));

const mockedScan = vi.mocked(scan);
const mockedRescan = vi.mocked(rescan);

function skillFixture(): Component {
  return {
    id: "skill:formatter",
    name: "Formatter",
    kind: "skill",
    description: "Formats source files",
    scope: "user",
    locations: [
      { path: "C:/fixtures/formatter", root: "claude-skills", origin: "file" },
      { path: null, root: "embedded-skills", origin: "embedded" },
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
      { path: "C:/fixtures/reviewer", root: "claude-agents", origin: "file" },
      { path: null, root: "embedded-agents", origin: "embedded" },
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
      locations: [{ path: `C:/fixtures/${kind}-${number}`, root: `claude-${kind}s`, origin: "file" }],
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
    rootsScanned: [{ id: "claude-skills", path: "C:/roots/claude", kind: "skill", status: "found" }],
    installations: [{ client: "claudeCode", version: "1.0.0", path: "C:/clients/claude" }],
    clientPresence: [
      {
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
      { id: "claude-skills", path: "C:/roots/claude", kind: "skill", status: "notFound" },
    ],
    issues: [],
  };
}

function issuesOnlyReportFixture(): ScanReport {
  return {
    ...reportFixture(),
    rootsScanned: [{ id: "claude-skills", path: "C:/roots/claude", kind: "skill", status: "found" }],
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
      { id: "claude-skills", path: "C:/roots/claude", kind: "skill", status: "notFound" },
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
    rootsScanned: [{ id: "claude-skills", path: "C:/roots/claude", kind: "skill", status: "found" }],
    clientPresence: [
      {
        label: "Claude Code CLI (npm)",
        probedPaths: ["C:/clients/claude-npm"],
        status: "detected",
        installations: [
          { client: "claudeCode", version: "1.0.0", path: "C:/clients/claude-npm" },
        ],
      },
      {
        label: "Claude Code (bundled in Claude Desktop)",
        probedPaths: ["C:/clients/pkg-a", "C:/clients/legacy"],
        status: "detected",
        installations: [
          { client: "claudeCode", version: "2.0.0", path: "C:/clients/pkg-a" },
          { client: "claudeCode", version: "2.1.0", path: "C:/clients/legacy" },
        ],
      },
      {
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

const nonBreakingSpace = String.fromCharCode(160);

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
    expect(visibleText()).toContain("Duplicate");
    expect(visibleText()).toContain("(no path on disk)");
    expect(visibleText()).toContain("Formatter");
    expect(visibleText()).toContain("C:/fixtures/formatter");

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
    expect(visibleText()).toContain("Duplicado");
    expect(document.querySelector("[title]")?.getAttribute("title")).toBe(
      "Encontrado en 2 ubicaciones",
    );
    expect(visibleText()).toContain("(sin ruta en disco)");
    expect(visibleText()).toContain("Formatter");
    expect(visibleText()).toContain("C:/fixtures/formatter");

    unmount(app);
  });

  it("starts in English even when the browser locale is Spanish, while allowing a switch to Spanish", async () => {
    Object.defineProperty(window.navigator, "languages", {
      configurable: true,
      value: ["es-ES"],
    });

    const app = mount(App, { target: document.body });
    await flushApp();

    expect(document.documentElement.lang).toBe("en");
    expect(visibleText()).toContain("Welcome to Vertice");
    expect(languageSelector().value).toBe("en");

    const selector = languageSelector();
    selector.value = "es";
    selector.dispatchEvent(new window.Event("change", { bubbles: true }));
    await flushApp();

    expect(document.documentElement.lang).toBe("es");
    expect(visibleText()).toContain("Bienvenido a Vertice");

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
    const selector = languageSelector();
    selector.value = "es";
    selector.dispatchEvent(new window.Event("change", { bubbles: true }));
    await flushApp();
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

  it("keeps Agents and Skills search queries independent across navigation", async () => {
    mockedScan.mockResolvedValue(reportFixture([skillFixture(), agentFixture()]));

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

    navigateTo("Agents");
    await flushApp();
    expect(document.querySelector<HTMLInputElement>('input[type="search"]')?.value).toBe(
      "nothing-matches",
    );
    expect(visibleText()).not.toContain("Reviewer");

    unmount(app);
  });

  it("renders a page-size selector alongside the language selector on either page", async () => {
    mockedScan.mockResolvedValue(reportFixture([skillFixture(), agentFixture()]));

    const app = mount(App, { target: document.body });
    await flushApp();

    for (const section of ["Agents", "Skills"]) {
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
      locations: [{ path: null, root: "claude-agents", origin: "file" }],
    };
    mockedScan.mockResolvedValue(reportFixture([agentFixture(), nullPathFileComponent]));

    const app = mount(App, { target: document.body });
    await flushApp();
    navigateTo("Agents");
    await flushApp();

    expect(visibleText()).toContain("Embedded (non-actionable)");
    expect(document.querySelectorAll('[data-testid="embedded-status"]')).toHaveLength(1);
    expect(visibleText()).toContain("Null Path Agent");

    const selector = languageSelector();
    selector.value = "es";
    selector.dispatchEvent(new window.Event("change", { bubbles: true }));
    await flushApp();

    expect(visibleText()).toContain("Integrado (sin acciones disponibles)");
    expect(visibleText()).toContain("Null Path Agent");

    unmount(app);
  });

  it("marks an embedded location with a non-null path as non-actionable without rendering an action control", async () => {
    const embeddedPath = "C:/fixtures/embedded/README.md";
    const embeddedComponent: Component = {
      ...skillFixture(),
      id: "skill:embedded-path",
      name: "Embedded Path Skill",
      locations: [{ path: embeddedPath, root: "builtin-skills", origin: "embedded" }],
    };
    mockedScan.mockResolvedValue(reportFixture([embeddedComponent]));

    const app = mount(App, { target: document.body });
    await flushApp();
    navigateTo("Skills");
    await flushApp();

    const row = Array.from(document.querySelectorAll("article")).find((candidate) =>
      candidate.textContent?.includes(embeddedComponent.name),
    );
    expect(row?.textContent).toContain(embeddedComponent.name);
    expect(row?.textContent).toContain(embeddedPath);
    expect(row?.textContent).toContain("Embedded (non-actionable)");
    expect(row?.querySelector('[data-testid="embedded-status"]')).not.toBeNull();
    expect(row?.querySelectorAll('button, [role="button"], a[href], input[type="button"], input[type="submit"]'))
      .toHaveLength(0);

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
      "AI Clients",
      "MCP",
      "Prompts",
      "Scan",
      "AI Subscriptions",
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

  it("renders an explicit empty state for each section with no backend source", async () => {
    const app = mount(App, { target: document.body });
    await flushApp();

    for (const section of ["MCP", "Prompts"]) {
      navigateTo(section);
      await flushApp();

      const placeholder = document.querySelector('[data-testid="placeholder-page"]');
      expect(placeholder, section).not.toBeNull();
      expect(placeholder?.textContent, section).toContain(section);
      expect(placeholder?.textContent, section).toContain("Nothing to show here yet");
      expect(document.title, section).toBe(`Vertice v0.1.0 — ${section}`);
      expect(document.querySelector('input[type="search"]'), section).toBeNull();
      expect(visibleText(), section).not.toContain("Formatter");
    }

    expect(mockedScan).toHaveBeenCalledTimes(1);
    expect(mockedRescan).not.toHaveBeenCalled();

    unmount(app);
  });

  it("navigates from every Home metric tile to its matching destination", async () => {
    mockedScan.mockResolvedValue(cleanReportFixture());

    const app = mount(App, { target: document.body });
    await flushApp();

    for (const [key, title] of [
      ["skills", "Skills"],
      ["agents", "Agents"],
      ["components", "Scan"],
      ["roots", "Scan"],
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

describe("App subscriptions page", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    Object.defineProperty(window.navigator, "languages", {
      configurable: true,
      value: ["en-US"],
    });
    document.title = "";
    document.body.innerHTML = "";
    mockedScan.mockResolvedValue(reportFixture());
  });

  it("renders one card per active subscription with plan, amount and renewal date", async () => {
    const app = mount(App, { target: document.body });
    await flushApp();
    navigateTo("AI Subscriptions");
    await flushApp();

    const cards = document.querySelectorAll('[data-testid="subscription-card"]');

    expect(document.title).toBe("Vertice v0.1.0 — AI Subscriptions");
    expect(cards).toHaveLength(SAMPLE_SUBSCRIPTIONS.length);
    expect(visibleText()).toContain("Sample data");
    expect(document.querySelector('[data-testid="placeholder-page"]')).toBeNull();

    const claude = Array.from(cards).find((card) => card.textContent?.includes("Claude Pro"));
    expect(claude?.textContent).toContain("Plan: Pro");
    expect(claude?.textContent).toContain("€18.99");
    expect(claude?.textContent).toContain("Monthly");
    expect(claude?.textContent).toContain("/month");
    expect(claude?.textContent).toMatch(/20[0-9]{2}/);

    const copilot = Array.from(cards).find((card) => card.textContent?.includes("GitHub Copilot"));
    expect(copilot?.textContent).toContain("Yearly");
    expect(copilot?.textContent).toContain("/year");

    unmount(app);
  });

  it("orders cards by soonest renewal and never triggers a scan", async () => {
    const app = mount(App, { target: document.body });
    await flushApp();
    navigateTo("AI Subscriptions");
    await flushApp();

    const headings = Array.from(
      document.querySelectorAll('[data-testid="subscription-card"] h2'),
    ).map((heading) => heading.textContent?.trim());

    expect(new Set(headings).size).toBe(SAMPLE_SUBSCRIPTIONS.length);
    expect(mockedScan).toHaveBeenCalledTimes(1);
    expect(mockedRescan).not.toHaveBeenCalled();

    unmount(app);
  });

  it("localizes the subscription chrome and currency format", async () => {
    const app = mount(App, { target: document.body });
    await flushApp();
    navigateTo("AI Subscriptions");
    await flushApp();

    const selector = languageSelector();
    selector.value = "es";
    selector.dispatchEvent(new window.Event("change", { bubbles: true }));
    await flushApp();

    expect(document.title).toBe("Vertice v0.1.0 — Suscripciones de IA");
    expect(visibleText()).toContain("Datos de ejemplo");
    expect(visibleText()).toContain("Gasto mensual");
    expect(visibleText()).toContain("Mensual");
    expect(visibleText().split(nonBreakingSpace).join(" ")).toContain("18,99 €");

    unmount(app);
  });
});
