import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { fetchLogFilePath } from "./appLog";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const mockedInvoke = vi.mocked(invoke);

describe("fetchLogFilePath", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("invokes the log_file_path command and resolves with the path unmodified", async () => {
    const path = "C:\\Users\\raul\\AppData\\Roaming\\com.vertice.app\\vertice.log";
    mockedInvoke.mockResolvedValue(path);

    await expect(fetchLogFilePath()).resolves.toBe(path);
    expect(mockedInvoke).toHaveBeenCalledWith("log_file_path");
  });
});
