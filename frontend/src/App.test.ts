// @vitest-environment jsdom

import { tick } from "svelte";
import { mount, unmount } from "svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App.svelte";
import type { Component } from "./bindings/Component";
import type { ScanReport } from "./bindings/ScanReport";
import { rescan, scan } from "./lib/scan";

vi.mock("./lib/scan", () => ({
  isScanError: (error: unknown) =>
    typeof error === "object" && error !== null && "kind" in error,
  rescan: vi.fn(),
  scan: vi.fn(),
}));

const mockedScan = vi.mocked(scan);
const mockedRescan = vi.mocked(rescan);

function componentFixture(): Component {
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

function reportFixture(components: Component[] = [componentFixture()]): ScanReport {
  return {
    components,
    installations: [],
    rootsScanned: [],
    issues: [],
    durationMs: 4,
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

  it("updates visible inventory chrome, document metadata, and avoids rescanning when the selector changes", async () => {
    const app = mount(App, { target: document.body });
    await flushApp();

    expect(mockedScan).toHaveBeenCalledTimes(1);
    expect(mockedRescan).not.toHaveBeenCalled();
    expect(document.documentElement.lang).toBe("en");
    expect(document.title).toBe("Vertice v0.1.0 — Home");
    expect(visibleText()).toContain("Welcome to Vertice");

    navigateTo("Inventory");
    await flushApp();

    expect(document.title).toBe("Vertice v0.1.0 — Inventory");
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
    expect(document.title).toBe("Vertice v0.1.0 — Inventario");
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

  it("renders localized scan failure chrome with the raw internal reason after loading settles", async () => {
    const rawReason = "ENOENT: raw core diagnostic";
    Object.defineProperty(window.navigator, "languages", {
      configurable: true,
      value: ["es-ES"],
    });
    mockedScan.mockRejectedValue({ kind: "internal", detail: { reason: rawReason } });

    const app = mount(App, { target: document.body });
    await flushApp();
    navigateTo("Inventario");
    await flushApp();

    const alert = document.querySelector('[role="alert"]');
    expect(mockedScan).toHaveBeenCalledTimes(1);
    expect(mockedRescan).not.toHaveBeenCalled();
    expect(document.querySelector('[role="status"]')).toBeNull();
    expect(alert?.textContent).toContain("escaneo del inventario.");
    expect(alert?.textContent).toContain(`Fallo interno del escaneo: ${rawReason}`);
    expect(alert?.textContent).toContain(rawReason);
    expect(visibleText()).toContain("Idioma");
    expect(visibleText()).not.toContain("Escaneando componentes instalados...");

    unmount(app);
  });

  it("renders a successful empty report as an empty inventory status distinct from failure", async () => {
    mockedScan.mockResolvedValue(reportFixture([]));

    const app = mount(App, { target: document.body });
    await flushApp();
    navigateTo("Inventory");
    await flushApp();

    const status = document.querySelector('[role="status"]');
    expect(mockedScan).toHaveBeenCalledTimes(1);
    expect(mockedRescan).not.toHaveBeenCalled();
    expect(document.querySelector('[role="alert"]')).toBeNull();
    expect(status?.textContent).toContain("No components to show.");
    expect(status?.textContent).not.toContain("Scanning for installed components...");
    expect(visibleText()).not.toContain("Inventory scan failed.");

    unmount(app);
  });
});
function mixedReportFixture(): ScanReport {
  return {
    ...reportFixture(),
    rootsScanned: [
      { id: "claude-skills", path: "C:/roots/claude", kind: "skill", status: "notFound" },
    ],
    issues: [
      { severity: "warning", path: null, reason: "search root claude-skills was not found" },
      {
        severity: "warning",
        path: "C:/Users/example/AppData/Roaming/npm",
        reason: "Claude Code (npm) not detected",
      },
      {
        severity: "error",
        path: "C:/fixtures/broken-skill/SKILL.md",
        reason: "Malformed frontmatter",
      },
    ],
  };
}

describe("App successful scan diagnostics", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    Object.defineProperty(window.navigator, "languages", {
      configurable: true,
      value: ["en-US"],
    });
    document.body.innerHTML = "";
  });

  it("keeps inventory visible and renders each mixed-report diagnostic exactly once", async () => {
    mockedScan.mockResolvedValue(mixedReportFixture());

    const app = mount(App, { target: document.body });
    await flushApp();
    navigateTo("Inventory");
    await flushApp();

    const diagnostics = document.querySelector('[data-testid="scan-diagnostics"]');
    expect(visibleText()).toContain("Formatter");
    expect(diagnostics?.textContent).toContain("Unavailable scan roots");
    expect(diagnostics?.textContent).toContain("C:/roots/claude");
    expect(diagnostics?.textContent).toContain("Claude Code (npm) not detected");
    expect(diagnostics?.textContent).toContain("C:/Users/example/AppData/Roaming/npm");
    expect(diagnostics?.textContent).toContain("Malformed frontmatter");
    expect(diagnostics?.textContent).toContain("C:/fixtures/broken-skill/SKILL.md");
    expect(diagnostics?.textContent?.match(/search root claude-skills was not found/g)).toBeNull();
    expect(document.querySelector('[role="alert"]')).toBeNull();

    unmount(app);
  });

  it("updates diagnostic chrome after switching to Spanish while keeping issue payloads verbatim", async () => {
    const missingClientReason = "Claude Code (npm) not detected";
    const missingClientPath = "C:/Users/example/AppData/Roaming/npm";
    const recoverableReason = "Malformed frontmatter";
    const recoverablePath = "C:/fixtures/broken-skill/SKILL.md";
    mockedScan.mockResolvedValue(mixedReportFixture());

    const app = mount(App, { target: document.body });
    await flushApp();
    navigateTo("Inventory");
    await flushApp();

    const diagnostics = document.querySelector('[data-testid="scan-diagnostics"]');
    expect(diagnostics?.textContent).toContain("Unavailable scan roots");
    expect(diagnostics?.textContent).toContain("Supported client unavailable");
    expect(diagnostics?.textContent).toContain("Recoverable scan issues");

    const selector = languageSelector();
    selector.value = "es";
    selector.dispatchEvent(new window.Event("change", { bubbles: true }));
    await flushApp();

    expect(diagnostics?.textContent).toContain("Raíces de escaneo no disponibles");
    expect(diagnostics?.textContent).toContain("Cliente compatible no disponible");
    expect(diagnostics?.textContent).toContain("Problemas recuperables del escaneo");
    expect(diagnostics?.textContent).toContain(missingClientReason);
    expect(diagnostics?.textContent).toContain(missingClientPath);
    expect(diagnostics?.textContent).toContain(recoverableReason);
    expect(diagnostics?.textContent).toContain(recoverablePath);

    unmount(app);
  });

  it("renders no diagnostics for a clean report", async () => {
    mockedScan.mockResolvedValue(reportFixture());

    const app = mount(App, { target: document.body });
    await flushApp();
    navigateTo("Inventory");
    await flushApp();

    expect(document.querySelector('[data-testid="scan-diagnostics"]')).toBeNull();

    unmount(app);
  });

  it("marks an embedded location without mistaking a null file path for embedded and localizes chrome", async () => {
    const nullPathFileComponent: Component = {
      ...componentFixture(),
      id: "agent:null-path",
      name: "Null Path Agent",
      kind: "agent",
      locations: [{ path: null, root: "claude-agents", origin: "file" }],
    };
    mockedScan.mockResolvedValue(reportFixture([componentFixture(), nullPathFileComponent]));

    const app = mount(App, { target: document.body });
    await flushApp();
    navigateTo("Inventory");
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
      ...componentFixture(),
      id: "skill:embedded-path",
      name: "Embedded Path Skill",
      locations: [{ path: embeddedPath, root: "builtin-skills", origin: "embedded" }],
    };
    mockedScan.mockResolvedValue(reportFixture([embeddedComponent]));

    const app = mount(App, { target: document.body });
    await flushApp();
    navigateTo("Inventory");
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

  it("keeps every embedded row non-actionable when embedded and file locations coexist", async () => {
    const embeddedAndFileComponent: Component = {
      ...componentFixture(),
      id: "agent:embedded-and-file",
      name: "Embedded and File Agent",
      kind: "agent",
      locations: [
        { path: "C:/fixtures/agent/AGENT.md", root: "claude-agents", origin: "file" },
        { path: "C:/fixtures/agent/default.md", root: "builtin-agents", origin: "embedded" },
      ],
    };
    mockedScan.mockResolvedValue(reportFixture([embeddedAndFileComponent]));

    const app = mount(App, { target: document.body });
    await flushApp();
    navigateTo("Inventory");
    await flushApp();

    const row = Array.from(document.querySelectorAll("article")).find((candidate) =>
      candidate.textContent?.includes(embeddedAndFileComponent.name),
    );
    expect(row?.textContent).toContain("Embedded (non-actionable)");
    expect(row?.querySelector('[data-testid="embedded-status"]')?.textContent).toContain(
      "Embedded (non-actionable)",
    );
    expect(row?.querySelectorAll('button, [role="button"], a[href], input[type="button"], input[type="submit"]'))
      .toHaveLength(0);

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

    expect(labels).toEqual(["Home", "Agents", "Skills", "MCP", "Prompts", "Inventory"]);
    expect(visibleText()).toContain("Welcome to Vertice");
    expect(sidebar?.querySelector('[aria-current="page"]')?.textContent?.trim()).toBe("Home");
    expect(document.querySelector('[data-testid="placeholder-page"]')).toBeNull();
    expect(document.querySelector('input[type="search"]')).toBeNull();

    unmount(app);
  });

  it("renders an explicit empty state for each section with no backend source", async () => {
    const app = mount(App, { target: document.body });
    await flushApp();

    for (const section of ["Agents", "Skills", "MCP", "Prompts"]) {
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

  it("keeps the inventory filter when navigating away and back", async () => {
    const app = mount(App, { target: document.body });
    await flushApp();
    navigateTo("Inventory");
    await flushApp();

    const search = document.querySelector<HTMLInputElement>('input[type="search"]');
    if (search === null) {
      throw new Error("Search input was not rendered");
    }
    search.value = "nothing-matches";
    search.dispatchEvent(new window.Event("input", { bubbles: true }));
    await flushApp();

    expect(visibleText()).not.toContain("Formatter");

    navigateTo("Home");
    await flushApp();
    navigateTo("Inventory");
    await flushApp();

    expect(document.querySelector<HTMLInputElement>('input[type="search"]')?.value).toBe(
      "nothing-matches",
    );
    expect(visibleText()).not.toContain("Formatter");

    unmount(app);
  });

  it("opens the inventory from the greeting page call to action", async () => {
    const app = mount(App, { target: document.body });
    await flushApp();

    const cta = Array.from(document.querySelectorAll("main button")).find(
      (candidate) => candidate.textContent?.trim() === "Open inventory",
    );
    if (cta === undefined) {
      throw new Error("Greeting call to action was not rendered");
    }
    (cta as HTMLButtonElement).click();
    await flushApp();

    expect(document.title).toBe("Vertice v0.1.0 — Inventory");
    expect(visibleText()).toContain("Formatter");

    unmount(app);
  });
});
