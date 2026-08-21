import { describe, expect, it } from "vitest";
import indexHtml from "../index.html?raw";

describe("bootstrap metadata", () => {
  it("keeps index.html aligned with the English default language and base title", () => {
    expect(indexHtml).toContain('<html lang="en">');
    expect(indexHtml).toContain("<title>Vertice v0.1.0</title>");
    expect(indexHtml).not.toContain('<html lang="es">');
  });
});
