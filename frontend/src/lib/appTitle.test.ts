import { describe, expect, it } from "vitest";
import { appTitle } from "./appTitle";

describe("appTitle", () => {
  it("joins the product name and version with a v-prefixed tag", () => {
    expect(appTitle("Vertice", "0.1.0")).toBe("Vertice v0.1.0");
  });

  it("adds a localized area label when provided", () => {
    expect(appTitle("Vertice", "0.1.0", "Inventario")).toBe("Vertice v0.1.0 — Inventario");
  });
});
