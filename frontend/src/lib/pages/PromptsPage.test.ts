// @vitest-environment jsdom

import { tick } from "svelte";
import { mount, unmount } from "svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Prompt } from "../../bindings/Prompt";
import { createPrompt, deletePrompt, fetchPrompts, updatePrompt } from "../prompts";
import { clearAll } from "../toast.svelte";
import PromptsPage from "./PromptsPageHarness.svelte";

vi.mock("../prompts", () => ({
  createPrompt: vi.fn(),
  deletePrompt: vi.fn(),
  fetchPrompts: vi.fn(),
  isPromptError: (error: unknown) =>
    typeof error === "object" &&
    error !== null &&
    ("invalidInput" in error || "notFound" in error || "storeUnavailable" in error),
  updatePrompt: vi.fn(),
}));

const mockedCreatePrompt = vi.mocked(createPrompt);
const mockedDeletePrompt = vi.mocked(deletePrompt);
const mockedFetchPrompts = vi.mocked(fetchPrompts);
const mockedUpdatePrompt = vi.mocked(updatePrompt);

function prompt(overrides: Partial<Prompt> = {}): Prompt {
  return {
    id: overrides.id ?? "prompt-1",
    title: overrides.title ?? "Review prompt",
    body: overrides.body ?? "Explain the tradeoffs.",
    tags: overrides.tags ?? ["review"],
    bestForContext: overrides.bestForContext ?? "Pull requests",
    updatedAt: overrides.updatedAt ?? "2026-08-26T14:00:00Z",
  };
}

async function flush(): Promise<void> {
  await tick();
  await Promise.resolve();
  await tick();
}

function visibleText(): string {
  return document.body.textContent ?? "";
}

function promptSet(count: number): Prompt[] {
  return Array.from({ length: count }, (_, index) => {
    const number = String(index + 1).padStart(2, "0");
    return prompt({
      id: `prompt-${number}`,
      title: `Prompt ${number}`,
      body: index % 2 === 0 ? `Odd body ${number}` : `Even body ${number}`,
      tags: index % 2 === 0 ? ["odd"] : ["even"],
      bestForContext: `Context ${number}`,
      updatedAt: `2026-08-26T14:${number}:00Z`,
    });
  });
}

function accessibleName(control: HTMLElement): string {
  return control.getAttribute("aria-label") ?? control.textContent?.trim() ?? "";
}

function buttonsByRole(name: string): HTMLButtonElement[] {
  return Array.from(document.querySelectorAll<HTMLButtonElement>("button")).filter(
    (button) => accessibleName(button) === name,
  );
}

function buttonByRole(name: string): HTMLButtonElement {
  const button = buttonsByRole(name)[0];
  expect(button, `Expected button role=button name=${name}`).toBeTruthy();
  return button;
}

function selectByLabel(label: string): HTMLSelectElement {
  const select = document.querySelector<HTMLSelectElement>(`select[aria-label="${label}"]`);
  expect(select, `Expected select with label ${label}`).toBeTruthy();
  return select!;
}

function promptCards(): HTMLElement[] {
  return Array.from(document.querySelectorAll<HTMLElement>('[data-testid="prompt-card"]'));
}

function changeInputValue(input: HTMLInputElement, value: string): void {
  input.value = value;
  input.dispatchEvent(new Event("input", { bubbles: true }));
}

function changeSelectValue(select: HTMLSelectElement, value: string): void {
  select.value = value;
  select.dispatchEvent(new Event("change", { bubbles: true }));
}

function confirmDialogConfirmButton(): HTMLButtonElement {
  const dialog = document.querySelector('[role="dialog"]');
  expect(dialog, "Expected confirm dialog to be open").toBeTruthy();
  const buttons = dialog!.querySelectorAll<HTMLButtonElement>("button");
  const confirm = Array.from(buttons).find((btn) => btn.textContent?.trim() === "Delete");
  expect(confirm, "Expected confirm button in dialog").toBeTruthy();
  return confirm!;
}

beforeEach(() => {
  document.body.innerHTML = "";
  vi.clearAllMocks();
  clearAll();
  Object.assign(navigator, {
    clipboard: {
      writeText: vi.fn().mockResolvedValue(undefined),
    },
  });
});

describe("PromptsPage", () => {
  it("shows loading then an empty state with a create action", async () => {
    mockedFetchPrompts.mockResolvedValue([]);

    const app = mount(PromptsPage, { target: document.body });
    expect(visibleText()).toContain("Loading prompts");
    await flush();

    expect(visibleText()).toContain("No prompts yet");
    expect(visibleText()).toContain("Create your first prompt");
    unmount(app);
  });

  it("shows failures distinctly from empty state", async () => {
    mockedFetchPrompts.mockRejectedValue(new Error("disk failed"));

    const app = mount(PromptsPage, { target: document.body });
    await flush();

    expect(document.querySelector('[role="alert"]')?.textContent).toContain("Could not load prompts");
    expect(visibleText()).not.toContain("No prompts yet");
    unmount(app);
  });

  it("blocks empty title and body saves without invoking persistence", async () => {
    mockedFetchPrompts.mockResolvedValue([]);

    const app = mount(PromptsPage, { target: document.body });
    await flush();
    document.querySelector<HTMLButtonElement>("button[data-testid='new-prompt']")?.click();
    await flush();
    document.querySelector<HTMLButtonElement>("button[type='submit']")?.click();
    await flush();

    expect(document.querySelector('[role="alert"]')?.textContent).toContain("Title is required");
    expect(document.querySelector('[role="alert"]')?.textContent).toContain("Body is required");
    expect(mockedCreatePrompt).not.toHaveBeenCalled();
    unmount(app);
  });

  it("creates edits deletes and copies visible prompts with feedback", async () => {
    const first = prompt({ title: "Draft", body: "Use SOLID principles", tags: ["architecture"] });
    const edited = prompt({ ...first, title: "Edited", body: "Use ports and adapters" });
    mockedFetchPrompts.mockResolvedValue([]);
    mockedCreatePrompt.mockResolvedValue(first);
    mockedUpdatePrompt.mockResolvedValue(edited);
    mockedDeletePrompt.mockResolvedValue(undefined);

    const app = mount(PromptsPage, { target: document.body });
    await flush();
    document.querySelector<HTMLButtonElement>("button[data-testid='new-prompt']")?.click();
    await flush();
    document.querySelector<HTMLInputElement>("#prompt-title")!.value = "Draft";
    document.querySelector<HTMLInputElement>("#prompt-title")!.dispatchEvent(new Event("input", { bubbles: true }));
    document.querySelector<HTMLTextAreaElement>("#prompt-body")!.value = "Use SOLID principles";
    document.querySelector<HTMLTextAreaElement>("#prompt-body")!.dispatchEvent(new Event("input", { bubbles: true }));
    document.querySelector<HTMLButtonElement>("button[type='submit']")?.click();
    await flush();

    expect(visibleText()).toContain("Draft");
    document.querySelector<HTMLButtonElement>("button[data-testid='copy-prompt']")?.click();
    await flush();
    expect(navigator.clipboard.writeText).toHaveBeenCalledWith("Use SOLID principles");
    expect(visibleText()).toContain("Prompt copied");

    document.querySelector<HTMLButtonElement>("button[data-testid='edit-prompt']")?.click();
    await flush();
    document.querySelector<HTMLInputElement>("#prompt-title")!.value = "Edited";
    document.querySelector<HTMLInputElement>("#prompt-title")!.dispatchEvent(new Event("input", { bubbles: true }));
    document.querySelector<HTMLTextAreaElement>("#prompt-body")!.value = "Use ports and adapters";
    document.querySelector<HTMLTextAreaElement>("#prompt-body")!.dispatchEvent(new Event("input", { bubbles: true }));
    document.querySelector<HTMLButtonElement>("button[type='submit']")?.click();
    await flush();

    expect(mockedUpdatePrompt).toHaveBeenCalledWith(expect.objectContaining({ id: first.id, title: "Edited" }));
    expect(visibleText()).toContain("Edited");

    document.querySelector<HTMLButtonElement>("button[data-testid='delete-prompt']")?.click();
    await flush();
    confirmDialogConfirmButton().click();
    await flush();
    expect(mockedDeletePrompt).toHaveBeenCalledWith(first.id);
    expect(visibleText()).toContain("Prompt deleted");
    unmount(app);
  });



  it("moves rejected creates into the retryable failure state without treating the library as empty", async () => {
    mockedFetchPrompts.mockResolvedValue([]);
    mockedCreatePrompt.mockRejectedValue({ storeUnavailable: { reason: "disk full" } });

    const app = mount(PromptsPage, { target: document.body });
    await flush();
    document.querySelector<HTMLButtonElement>("button[data-testid='new-prompt']")?.click();
    await flush();
    document.querySelector<HTMLInputElement>("#prompt-title")!.value = "Draft";
    document.querySelector<HTMLInputElement>("#prompt-title")!.dispatchEvent(new Event("input", { bubbles: true }));
    document.querySelector<HTMLTextAreaElement>("#prompt-body")!.value = "Body";
    document.querySelector<HTMLTextAreaElement>("#prompt-body")!.dispatchEvent(new Event("input", { bubbles: true }));
    document.querySelector<HTMLButtonElement>("button[type='submit']")?.click();
    await flush();

    expect(document.querySelector('[role="alert"]')?.textContent).toContain("Could not save the prompt");
    expect(document.querySelector('[role="alert"]')?.textContent).toContain("The change was not persisted");
    expect(visibleText()).toContain("Retry");
    expect(visibleText()).not.toContain("No prompts yet");
    expect(visibleText()).not.toContain("Prompt saved");
    unmount(app);
  });

  it("moves rejected updates into the retryable failure state without mutating the visible prompt", async () => {
    const existing = prompt({ title: "Original", body: "Original body" });
    mockedFetchPrompts.mockResolvedValue([existing]);
    mockedUpdatePrompt.mockRejectedValue({ storeUnavailable: { reason: "schema" } });

    const app = mount(PromptsPage, { target: document.body });
    await flush();
    document.querySelector<HTMLButtonElement>("button[data-testid='edit-prompt']")?.click();
    await flush();
    document.querySelector<HTMLInputElement>("#prompt-title")!.value = "Changed";
    document.querySelector<HTMLInputElement>("#prompt-title")!.dispatchEvent(new Event("input", { bubbles: true }));
    document.querySelector<HTMLButtonElement>("button[type='submit']")?.click();
    await flush();

    expect(document.querySelector('[role="alert"]')?.textContent).toContain("Could not save the prompt");
    expect(document.querySelector('[role="alert"]')?.textContent).toContain("The change was not persisted");
    expect(visibleText()).toContain("Retry");
    expect(visibleText()).not.toContain("Changed");
    expect(visibleText()).not.toContain("Prompt saved");
    unmount(app);
  });

  it("moves rejected deletes into the retryable failure state without removing the visible prompt", async () => {
    const existing = prompt({ title: "Keep me", body: "Still stored" });
    mockedFetchPrompts.mockResolvedValue([existing]);
    mockedDeletePrompt.mockRejectedValue({ storeUnavailable: { reason: "rename failed" } });

    const app = mount(PromptsPage, { target: document.body });
    await flush();
    document.querySelector<HTMLButtonElement>("button[data-testid='delete-prompt']")?.click();
    await flush();
    confirmDialogConfirmButton().click();
    await flush();

    expect(mockedDeletePrompt).toHaveBeenCalledWith(existing.id);
    expect(document.querySelector('[role="alert"]')?.textContent).toContain("Could not delete the prompt");
    expect(document.querySelector('[role="alert"]')?.textContent).toContain("The change was not persisted");
    expect(visibleText()).toContain("Retry");
    expect(visibleText()).toContain("Keep me");
    expect(visibleText()).not.toContain("Prompt deleted");
    unmount(app);
  });


  it("exposes stable semantic action buttons with keyboard focus and danger treatment", async () => {
    mockedFetchPrompts.mockResolvedValue([prompt({ title: "Action prompt", body: "Copy this body" })]);

    const app = mount(PromptsPage, { target: document.body });
    await flush();

    const copy = buttonByRole("Copy");
    const edit = buttonByRole("Edit");
    const deleteButton = buttonByRole("Delete");

    for (const [button, name] of [[copy, "Copy"], [edit, "Edit"], [deleteButton, "Delete"]] as const) {
      expect(button.textContent?.trim()).toBe(name);
      button.focus();
      await flush();
      expect(document.activeElement).toBe(button);
      expect(button.className).toContain("focus-visible:outline");
      expect(button.className).toContain("hover:bg");
    }

    expect(deleteButton.className).toContain("border-red");
    expect(deleteButton.className).toContain("text-red");
    expect(deleteButton.className).toContain("focus-visible:outline-red");
    unmount(app);
  });

  it("paginates prompts with default size five and page-size choices that clamp only when needed", async () => {
    mockedFetchPrompts.mockResolvedValue(promptSet(12));

    const app = mount(PromptsPage, { target: document.body });
    await flush();

    expect(promptCards()).toHaveLength(5);
    expect(visibleText()).toContain("Prompt 01");
    expect(visibleText()).toContain("Prompt 05");
    expect(visibleText()).not.toContain("Prompt 06");
    expect(visibleText()).toContain("Showing 1–5 of 12 prompts");

    buttonByRole("Go to next page").click();
    await flush();
    expect(visibleText()).toContain("Page 2 of 3");
    expect(visibleText()).toContain("Prompt 06");
    expect(visibleText()).toContain("Prompt 10");

    const pageSize = selectByLabel("Prompts per page");
    expect(Array.from(pageSize.options).map((option) => option.value)).toEqual(["5", "10", "15"]);

    changeSelectValue(pageSize, "10");
    await flush();
    expect(visibleText()).toContain("Page 2 of 2");
    expect(visibleText()).toContain("Prompt 11");
    expect(visibleText()).toContain("Prompt 12");
    expect(visibleText()).not.toContain("Prompt 01");

    changeSelectValue(pageSize, "15");
    await flush();
    expect(visibleText()).toContain("Page 1 of 1");
    expect(promptCards()).toHaveLength(12);
    unmount(app);
  });

  it("resets only query changes to page one and clamps after result shrink", async () => {
    mockedFetchPrompts.mockResolvedValue(promptSet(12));
    mockedDeletePrompt.mockResolvedValue(undefined);

    const app = mount(PromptsPage, { target: document.body });
    await flush();
    buttonByRole("Go to next page").click();
    await flush();
    expect(visibleText()).toContain("Page 2 of 3");
    expect(visibleText()).toContain("Prompt 06");

    changeInputValue(document.querySelector<HTMLInputElement>("#prompt-search")!, "even");
    await flush();
    expect(visibleText()).toContain("Page 1 of 2");
    expect(visibleText()).toContain("Prompt 02");
    expect(visibleText()).toContain("Prompt 10");
    expect(visibleText()).not.toContain("Prompt 12");

    changeInputValue(document.querySelector<HTMLInputElement>("#prompt-search")!, "");
    await flush();
    buttonByRole("Go to last page").click();
    await flush();
    expect(visibleText()).toContain("Page 3 of 3");
    expect(visibleText()).toContain("Prompt 11");
    expect(visibleText()).toContain("Prompt 12");

    for (const deleteButton of buttonsByRole("Delete")) {
      deleteButton.click();
      await flush();
      confirmDialogConfirmButton().click();
      await flush();
    }

    expect(mockedDeletePrompt).toHaveBeenCalledTimes(2);
    expect(visibleText()).toContain("Page 2 of 2");
    expect(visibleText()).toContain("Prompt 06");
    expect(visibleText()).toContain("Prompt 10");
    expect(visibleText()).not.toContain("Prompt 11");
    unmount(app);
  });


  it("announces copy failures without mutating user content or invoking clients", async () => {
    mockedFetchPrompts.mockResolvedValue([prompt({ title: "TÃ­tulo", body: "Contenido exacto" })]);
    vi.mocked(navigator.clipboard.writeText).mockRejectedValue(new Error("denied"));

    const app = mount(PromptsPage, { target: document.body, props: { locale: "es" } });
    await flush();
    document.querySelector<HTMLButtonElement>("button[data-testid='copy-prompt']")?.click();
    await flush();

    expect(visibleText()).toContain("TÃ­tulo");
    expect(visibleText()).toContain("Contenido exacto");
    expect(visibleText()).toContain("No se pudo copiar el prompt");
    unmount(app);
  });
});
