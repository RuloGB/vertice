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