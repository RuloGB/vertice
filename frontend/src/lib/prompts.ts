import { invoke } from "@tauri-apps/api/core";
import type { Prompt } from "../bindings/Prompt";
import type { PromptDraft } from "../bindings/PromptDraft";
import type { PromptError } from "../bindings/PromptError";
import type { PromptUpdate } from "../bindings/PromptUpdate";

export function fetchPrompts(): Promise<Prompt[]> {
  return invoke<Prompt[]>("list_prompts");
}

export function createPrompt(draft: PromptDraft): Promise<Prompt> {
  return invoke<Prompt>("create_prompt", { draft });
}

export function updatePrompt(update: PromptUpdate): Promise<Prompt> {
  return invoke<Prompt>("update_prompt", { update });
}

export function deletePrompt(id: string): Promise<void> {
  return invoke<void>("delete_prompt", { id });
}

export function isPromptError(error: unknown): error is PromptError {
  if (typeof error !== "object" || error === null) return false;
  return "invalidInput" in error || "notFound" in error || "storeUnavailable" in error;
}
