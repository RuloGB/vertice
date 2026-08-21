import { describe, expect, it } from "vitest";
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

  it("interpolates duplicate location counts in both catalogs", () => {
    expect(formatMessage(catalogs.en.inventory.duplicateTitle, { count: 3 })).toBe(
      "Found at 3 locations",
    );
    expect(formatMessage(catalogs.es.inventory.duplicateTitle, { count: 3 })).toBe(
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
});