import { describe, expect, it } from "vitest";
import { appTitle } from "./appTitle";

describe("appTitle", () => {
  it("joins the product name and version with a v-prefixed tag", () => {
    expect(appTitle("Vertice", "0.1.0")).toBe("Vertice v0.1.0");
  });

  it("reflects a different product name and version (triangulation)", () => {
    expect(appTitle("Vertice PoC", "2.3.1")).toBe("Vertice PoC v2.3.1");
  });
});
