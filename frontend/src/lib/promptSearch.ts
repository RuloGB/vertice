import type { Prompt } from "../bindings/Prompt";

export function normalizePromptSearchTerm(value: string): string {
  return value
    .trim()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase();
}

export function filterPrompts(prompts: readonly Prompt[], query: string): Prompt[] {
  const needle = normalizePromptSearchTerm(query);
  if (needle === "") return [...prompts];
  return prompts.filter((prompt) => searchableText(prompt).some((value) => value.includes(needle)));
}

function searchableText(prompt: Prompt): string[] {
  return [
    prompt.title,
    prompt.body,
    prompt.bestForContext ?? "",
    ...prompt.tags,
  ].map(normalizePromptSearchTerm);
}
