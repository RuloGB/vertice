import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import type { ScanReport } from "../bindings/ScanReport";
import { isScanError, rescan, scan } from "./scan";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const mockedInvoke = vi.mocked(invoke);

function reportFixture(): ScanReport {
  return {
    components: [],
    installations: [],
    rootsScanned: [
      {
        id: "claude-skills",
        path: "C:\\Users\\raul\\.claude\\skills",
        kind: "skill",
        status: "found",
      },
    ],
    issues: [],
    durationMs: 3,
  };
}

describe("scan command wrapper", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("invokes the scan command and resolves with the report unmodified", async () => {
    const report = reportFixture();
    mockedInvoke.mockResolvedValue(report);

    await expect(scan()).resolves.toBe(report);
    expect(mockedInvoke).toHaveBeenCalledWith("scan");
  });

  it("invokes the rescan command and resolves with the report unmodified", async () => {
    const report = reportFixture();
    mockedInvoke.mockResolvedValue(report);

    await expect(rescan()).resolves.toBe(report);
    expect(mockedInvoke).toHaveBeenCalledWith("rescan");
  });

  it("rejects with a typed ScanError payload when no roots are configured", async () => {
    mockedInvoke.mockRejectedValue({ kind: "noRootsConfigured" });

    const failure: unknown = await scan().catch((error: unknown) => error);

    expect(isScanError(failure)).toBe(true);
  });

  it("rejects with a typed ScanError payload on an internal failure", async () => {
    mockedInvoke.mockRejectedValue({
      kind: "internal",
      detail: { reason: "join failure: task panicked" },
    });

    const failure: unknown = await rescan().catch((error: unknown) => error);

    expect(isScanError(failure)).toBe(true);
  });
});

describe("isScanError", () => {
  it.each([null, undefined, "scan failed", { kind: "unknown" }, { kind: "internal" }])(
    "rejects values that are not a ScanError: %o",
    (value) => {
      expect(isScanError(value)).toBe(false);
    },
  );

  it("accepts both ScanError variants", () => {
    expect(isScanError({ kind: "noRootsConfigured" })).toBe(true);
    expect(isScanError({ kind: "internal", detail: { reason: "task panicked" } })).toBe(true);
  });
});
