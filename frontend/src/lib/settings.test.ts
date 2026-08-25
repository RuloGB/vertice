import { invoke } from "@tauri-apps/api/core";
import { describe, expect, it, vi } from "vitest";
import { fetchUserSettings, setUserSettings } from "./settings";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const mockedInvoke = vi.mocked(invoke);

describe("fetchUserSettings", () => {
  it("invokes the user_settings command", () => {
    mockedInvoke.mockResolvedValue({ locale: null, enabled: true, disclosureSeen: false });

    void fetchUserSettings();

    expect(mockedInvoke).toHaveBeenCalledWith("user_settings");
  });
});

describe("setUserSettings", () => {
  it("sends omitted fields as null, pinning the partial-patch wire shape", () => {
    mockedInvoke.mockResolvedValue({ locale: "es", enabled: true, disclosureSeen: false });

    void setUserSettings({ locale: "es" });

    expect(mockedInvoke).toHaveBeenCalledWith("set_user_settings", {
      locale: "es",
      enabled: null,
      disclosureSeen: null,
    });
  });

  it("sends every field verbatim when the caller provides all three", () => {
    mockedInvoke.mockResolvedValue({ locale: "en", enabled: false, disclosureSeen: true });

    void setUserSettings({ locale: "en", enabled: false, disclosureSeen: true });

    expect(mockedInvoke).toHaveBeenCalledWith("set_user_settings", {
      locale: "en",
      enabled: false,
      disclosureSeen: true,
    });
  });
});
