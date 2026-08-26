import { describe, expect, it } from "vitest";
import { catalogs } from "./i18n/catalogs";
import {
  areaLabelKey,
  DEFAULT_ROUTE,
  hasContent,
  isRouteId,
  NAV_GROUPS,
  navGroupLabelKey,
  navLabelKey,
  ROUTE_IDS,
  type RouteId,
} from "./navigation";

describe("navigation model", () => {
  it("lands on the greeting page by default", () => {
    expect(DEFAULT_ROUTE).toBe("home");
  });

  it("exposes every route exactly once across the sidebar groups", () => {
    const grouped = NAV_GROUPS.flatMap((group) => group.routes);

    expect([...grouped].sort()).toEqual([...ROUTE_IDS].sort());
    expect(new Set(grouped).size).toBe(grouped.length);
  });

  it("narrows only known route identifiers", () => {
    expect(isRouteId("scan")).toBe(true);
    expect(isRouteId("inventory")).toBe(false);
    expect(isRouteId("Scan")).toBe(false);
    expect(isRouteId("settings")).toBe(false);
  });

  it("separates sections that render content from the empty placeholders", () => {
    for (const route of ["home", "agents", "skills", "mcp", "scan", "subscriptions"] satisfies RouteId[]) {
      expect(hasContent(route), route).toBe(true);
    }
    for (const route of ["prompts"] satisfies RouteId[]) {
      expect(hasContent(route), route).toBe(false);
    }
  });

  it("resolves label keys that exist in both catalogs", () => {
    for (const locale of ["en", "es"] as const) {
      for (const route of ROUTE_IDS) {
        expect(catalogs[locale].nav[route], `${locale} ${navLabelKey(route)}`).toBeTruthy();
        expect(catalogs[locale].area[route], `${locale} ${areaLabelKey(route)}`).toBeTruthy();
      }
      for (const group of NAV_GROUPS) {
        expect(
          catalogs[locale].navGroup[group.id],
          `${locale} ${navGroupLabelKey(group.id)}`,
        ).toBeTruthy();
      }
    }
  });
});
