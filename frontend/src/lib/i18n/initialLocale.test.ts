import { describe, expect, it, vi } from "vitest";
import { resolveInitialLocale, SETTINGS_TIMEOUT_MS } from "./initialLocale";

describe("resolveInitialLocale", () => {
  it("prefers a persisted supported locale over the browser languages", async () => {
    const load = vi.fn().mockResolvedValue({ locale: "es" });

    const locale = await resolveInitialLocale(load, ["en-US"]);

    expect(locale).toBe("es");
  });

  it("falls through to the browser languages when locale is null", async () => {
    const load = vi.fn().mockResolvedValue({ locale: null });

    const locale = await resolveInitialLocale(load, ["es-MX"]);

    expect(locale).toBe("es");
  });

  it("falls through to the browser languages when the persisted locale is unsupported", async () => {
    const load = vi.fn().mockResolvedValue({ locale: "pt-BR" });

    const locale = await resolveInitialLocale(load, ["en-US"]);

    expect(locale).toBe("en");
  });

  it("falls through to the browser languages when the loader rejects", async () => {
    const load = vi.fn().mockRejectedValue(new Error("IPC unavailable"));

    const locale = await resolveInitialLocale(load, ["es-MX"]);

    expect(locale).toBe("es");
  });

  it("falls through to the browser languages after the timeout when the loader never settles", async () => {
    vi.useFakeTimers();
    try {
      const load = vi.fn().mockImplementation(() => new Promise(() => {}));

      const pending = resolveInitialLocale(load, ["es-MX"]);
      await vi.advanceTimersByTimeAsync(SETTINGS_TIMEOUT_MS);

      await expect(pending).resolves.toBe("es");
    } finally {
      vi.useRealTimers();
    }
  });
});
