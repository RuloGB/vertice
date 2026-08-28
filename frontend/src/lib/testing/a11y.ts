export type SupportedRole =
  | "alert"
  | "button"
  | "combobox"
  | "dialog"
  | "heading"
  | "spinbutton"
  | "status"
  | "textbox";

export type RoleQuery = { name?: string | RegExp };

// Deliberately scoped to the native elements and ARIA roles used by the page
// tests. It relies on DOM label association and ARIA naming attributes rather
// than approximating the complete accessibility-tree algorithm.
function roleOf(element: Element): SupportedRole | null {
  const explicitRole = element.getAttribute("role") as SupportedRole | null;
  if (explicitRole === "alert" || explicitRole === "dialog" || explicitRole === "status") {
    return explicitRole;
  }
  if (/^h[1-6]$/.test(element.tagName.toLowerCase())) return "heading";
  if (element instanceof HTMLButtonElement) return "button";
  if (element instanceof HTMLSelectElement) return "combobox";
  if (element instanceof HTMLInputElement) return element.type === "number" ? "spinbutton" : "textbox";
  return null;
}

function referencedText(element: Element): string | null {
  const ids = element.getAttribute("aria-labelledby");
  if (ids === null) return null;
  return ids
    .split(/\s+/)
    .map((id) => document.getElementById(id)?.textContent?.trim() ?? "")
    .join(" ")
    .trim();
}

function accessibleName(element: Element): string {
  return referencedText(element)
    ?? element.getAttribute("aria-label")?.trim()
    ?? nativeLabel(element)
    ?? element.textContent?.trim()
    ?? "";
}

function nativeLabel(element: Element): string | null {
  if (!(element instanceof HTMLInputElement || element instanceof HTMLSelectElement)) return null;
  return [...document.getElementsByTagName("label")]
    .filter((label) => label.control === element)
    .map((label) => label.textContent?.trim() ?? "")
    .join(" ");
}

function matchesName(actual: string, expected: string | RegExp | undefined): boolean {
  return expected === undefined || (typeof expected === "string" ? actual === expected : expected.test(actual));
}

export function getAllByRole<T extends Element = Element>(role: SupportedRole, query: RoleQuery = {}): T[] {
  return [...document.body.querySelectorAll("*")]
    .filter((element) => roleOf(element) === role && matchesName(accessibleName(element), query.name)) as T[];
}

export function getByRole<T extends Element = Element>(role: SupportedRole, query: RoleQuery = {}): T {
  const matches = getAllByRole<T>(role, query);
  if (matches.length !== 1) {
    throw new Error(`Expected exactly one ${role} matching ${String(query.name)}, found ${matches.length}`);
  }
  return matches[0]!;
}

export function queryByRole<T extends Element = Element>(role: SupportedRole, query: RoleQuery = {}): T | null {
  const matches = getAllByRole<T>(role, query);
  if (matches.length > 1) {
    throw new Error(`Expected at most one ${role} matching ${String(query.name)}, found ${matches.length}`);
  }
  return matches[0] ?? null;
}

export function getByLabel<T extends HTMLInputElement | HTMLSelectElement>(label: string): T {
  const control = [...document.getElementsByTagName("label")]
    .find((candidate) => candidate.textContent?.trim() === label)
    ?.control;
  if (!(control instanceof HTMLInputElement || control instanceof HTMLSelectElement)) {
    throw new Error(`Expected a form control labelled ${label}`);
  }
  return control as T;
}
