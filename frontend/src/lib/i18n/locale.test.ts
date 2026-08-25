import { describe, expect, it, vi } from "vitest";
import { catalogs, type Catalog } from "./catalogs";
import { createI18n, formatMessage, resolveLocale } from "./locale.svelte";

function flattenKeys(value: Record<string, unknown>, prefix = ""): string[] {
  return Object.entries(value).flatMap(([key, child]) => {
    const path = prefix ? `${prefix}.${key}` : key;
    if (typeof child === "object" && child !== null) {
      return flattenKeys(child as Record<string, unknown>, path);
    }
    return [path];
  });
}

function messageAt(catalog: Catalog, key: string): string {
  return key.split(".").reduce<unknown>((node, part) => {
    if (typeof node !== "object" || node === null || !(part in node)) {
      throw new Error(`Missing key ${key}`);
    }
    return (node as Record<string, unknown>)[part];
  }, catalog) as string;
}

describe("resolveLocale", () => {
  it("maps Spanish browser variants to Spanish", () => {
    expect(resolveLocale(["es-MX", "en-US"])).toBe("es");
  });

  it("maps English browser variants to English", () => {
    expect(resolveLocale("en-GB")).toBe("en");
  });

  it("falls back to English for unsupported or missing locales", () => {
    expect(resolveLocale(["pt-BR", "fr-FR"])).toBe("en");
    expect(resolveLocale(null)).toBe("en");
  });
});

describe("catalogs", () => {
  it("keeps English and Spanish catalogs in exact key parity", () => {
    expect(flattenKeys(catalogs.es)).toEqual(flattenKeys(catalogs.en));
  });

  it("localizes the scan route's log-path label and hint in both locales", () => {
    for (const locale of ["en", "es"] as const) {
      expect(catalogs[locale].scan.logPathLabel.trim()).not.toBe("");
      expect(catalogs[locale].scan.logPathHint.trim()).not.toBe("");
    }
  });

  it("keeps every catalog message non-blank", () => {
    for (const locale of ["en", "es"] as const) {
      for (const key of flattenKeys(catalogs[locale])) {
        expect(messageAt(catalogs[locale], key).trim(), `${locale}.${key}`).not.toBe("");
      }
    }
  });

  it("includes chrome copy for filters, lifecycle, duplicates, kinds, aria labels, and nullable paths", () => {
    expect(catalogs.en.toolbar.searchPlaceholder).toBe("Search by name");
    expect(catalogs.es.toolbar.searchPlaceholder).toBe("Buscar por nombre");
    expect(catalogs.en.location.noPath).toBe("(no path on disk)");
    expect(catalogs.es.location.noPath).toBe("(sin ruta en disco)");
  });

  it("retires the keys scoped only to the removed combined inventory route", () => {
    for (const locale of ["en", "es"] as const) {
      const keys = flattenKeys(catalogs[locale]);

      expect(keys).not.toContain("nav.inventory");
      expect(keys).not.toContain("area.inventory");
      expect(keys).not.toContain("toolbar.allKinds");
      expect(keys).not.toContain("toolbar.kindAriaLabel");
      expect(keys).not.toContain("diagnostics.unavailableRoots");
      expect(keys).not.toContain("diagnostics.missingClient");
      expect(keys).not.toContain("scan.installationsTitle");
      expect(keys).not.toContain("scan.installationsEmpty");
      expect(keys.some((key) => key.startsWith("inventory."))).toBe(false);
    }
  });

  it("adds the components namespace replacing the retired inventory keys", () => {
    for (const locale of ["en", "es"] as const) {
      const catalog = catalogs[locale] as unknown as Record<string, Record<string, string>>;

      expect(catalog.components.loading).toBeTruthy();
      expect(catalog.components.empty).toBeTruthy();
      expect(catalog.components.duplicate).toBeTruthy();
      expect(catalog.components.duplicateTitle).toBeTruthy();
      expect(catalog.components.embedded).toBeTruthy();
    }
  });

  it("adds scan-route, incident, and Home scan-status keys in both locales", () => {
    for (const locale of ["en", "es"] as const) {
      const catalog = catalogs[locale] as unknown as Record<string, Record<string, string>>;

      expect(catalog.nav.scan).toBeTruthy();
      expect(catalog.area.scan).toBeTruthy();
      expect(catalog.scan.verdictHealthy).toBeTruthy();
      expect(catalog.scan.verdictIssues).toBeTruthy();
      expect(catalog.scan.rootsTitle).toBeTruthy();
      expect(catalog.scan.rootFound).toBeTruthy();
      expect(catalog.scan.rootNotFound).toBeTruthy();
      expect(catalog.scan.clientsTitle).toBeTruthy();
      expect(catalog.scan.clientDetected).toBeTruthy();
      expect(catalog.scan.clientNotDetected).toBeTruthy();
      expect(catalog.scan.clientVersionUnavailable).toBeTruthy();
      expect(catalog.scan.clientsUnsupportedPlatform).toBeTruthy();
      expect(catalog.scan.durationLabel).toBeTruthy();
      expect(catalog.scan.durationValue).toBeTruthy();
      expect(catalog.incident.label).toBeTruthy();
      expect(catalog.incident.count).toBeTruthy();
      expect(catalog.incident.action).toBeTruthy();
      expect(catalog.home.scanTitle).toBeTruthy();
      expect(catalog.home.scanHealthy).toBeTruthy();
      expect(catalog.home.scanIssues).toBeTruthy();
      expect(catalog.home.scanFailed).toBeTruthy();
      expect(catalog.home.scanRetry).toBeTruthy();
      expect(catalog.home.scanOpen).toBeTruthy();
      expect(catalog.home.scanPending).toBeTruthy();
    }
  });

  it("adds freshness badge, disclosure, and opt-out setting keys in both locales", () => {
    for (const locale of ["en", "es"] as const) {
      const catalog = catalogs[locale] as unknown as Record<string, Record<string, string>>;

      expect(catalog.freshness.upToDate).toBeTruthy();
      expect(catalog.freshness.outdated).toBeTruthy();
      expect(catalog.freshness.unknown).toBeTruthy();
      expect(catalog.freshness.pending).toBeTruthy();
      expect(catalog.freshness.disclosureTitle).toBeTruthy();
      expect(catalog.freshness.disclosureBody).toBeTruthy();
      expect(catalog.freshness.disclosureDismiss).toBeTruthy();
      expect(catalog.freshness.settingLabel).toBeTruthy();
      expect(catalog.freshness.settingDescription).toBeTruthy();
      expect(catalog.freshness.settingToggleAria).toBeTruthy();
    }
  });
});

describe("formatMessage", () => {
  it("interpolates named placeholders while preserving raw diagnostic payloads", () => {
    const reason = "join failure: task panicked";

    expect(formatMessage(catalogs.en.failure.internalReason, { reason })).toBe(
      "Internal scan failure: join failure: task panicked",
    );
    expect(formatMessage(catalogs.es.failure.internalReason, { reason })).toBe(
      "Fallo interno del escaneo: join failure: task panicked",
    );
  });

  it("interpolates a freshness verdict's reference version verbatim in both catalogs", () => {
    const latest = "2.1.241";

    expect(formatMessage(catalogs.en.freshness.outdated, { latest })).toBe(
      "Update available: 2.1.241",
    );
    expect(formatMessage(catalogs.es.freshness.outdated, { latest })).toBe(
      "Actualización disponible: 2.1.241",
    );
  });

  it("interpolates duplicate location counts in both catalogs", () => {
    expect(formatMessage(catalogs.en.components.duplicateTitle, { count: 3 })).toBe(
      "Found at 3 locations",
    );
    expect(formatMessage(catalogs.es.components.duplicateTitle, { count: 3 })).toBe(
      "Encontrado en 3 ubicaciones",
    );
  });
});

describe("createI18n", () => {
  it("switches all translations from one shared locale source", () => {
    const i18n = createI18n("en");

    expect(i18n.locale).toBe("en");
    expect(i18n.t("toolbar.reload")).toBe("Reload");

    i18n.setLocale("es");

    expect(i18n.locale).toBe("es");
    expect(i18n.t("toolbar.reload")).toBe("Recargar");
  });

  it("invokes the onLocaleChange callback exactly once with the new locale, after switching translations", () => {
    const onLocaleChange = vi.fn();
    const i18n = createI18n("en", onLocaleChange);

    i18n.setLocale("es");

    expect(i18n.locale).toBe("es");
    expect(onLocaleChange).toHaveBeenCalledTimes(1);
    expect(onLocaleChange).toHaveBeenCalledWith("es");
  });

  it("still switches translations even when the onLocaleChange callback throws", () => {
    const onLocaleChange = vi.fn().mockImplementation(() => {
      throw new Error("write-through failed");
    });
    const i18n = createI18n("en", onLocaleChange);

    expect(() => i18n.setLocale("es")).not.toThrow();

    expect(i18n.locale).toBe("es");
    expect(i18n.t("toolbar.reload")).toBe("Recargar");
  });
});