import { invoke } from "@tauri-apps/api/core";
import { describe, expect, it, vi } from "vitest";
import { createPrompt, deletePrompt, fetchPrompts, isPromptError, updatePrompt } from "./prompts";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const mockedInvoke = vi.mocked(invoke);

describe("prompts IPC wrappers", () => {
  it("invokes typed prompt commands with explicit payloads", () => {
    void fetchPrompts();
    void createPrompt({ title: "Title", body: "Body", tags: [], bestForContext: null });
    void updatePrompt({ id: "id", title: "Title", body: "Body", tags: [], bestForContext: null });
    void deletePrompt("id");

    expect(mockedInvoke).toHaveBeenNthCalledWith(1, "list_prompts");
    expect(mockedInvoke).toHaveBeenNthCalledWith(2, "create_prompt", {
      draft: { title: "Title", body: "Body", tags: [], bestForContext: null },
    });
    expect(mockedInvoke).toHaveBeenNthCalledWith(3, "update_prompt", {
      update: { id: "id", title: "Title", body: "Body", tags: [], bestForContext: null },
    });
    expect(mockedInvoke).toHaveBeenNthCalledWith(4, "delete_prompt", { id: "id" });
  });

  it("narrows every prompt error variant and rejects unrelated objects", () => {
    expect(isPromptError({ invalidInput: { field: "title" } })).toBe(true);
    expect(isPromptError({ notFound: { id: "id" } })).toBe(true);
    expect(isPromptError({ storeUnavailable: { reason: "bad json" } })).toBe(true);
    expect(isPromptError({ kind: "internal" })).toBe(false);
  });
});
