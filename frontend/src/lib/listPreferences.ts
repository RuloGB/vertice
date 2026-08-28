const DEFAULT_PAGE_SIZE = 5;
const PAGE_SIZES = new Set([5, 10, 15]);

export type PaginatedListId = "agents" | "skills" | "mcp" | "prompts";

function storageKey(list: PaginatedListId): string {
  return `vertice.list-page-size.${list}`;
}

export function getListPageSize(list: PaginatedListId): number {
  if (typeof localStorage === "undefined") return DEFAULT_PAGE_SIZE;

  const value = Number(localStorage.getItem(storageKey(list)));
  return PAGE_SIZES.has(value) ? value : DEFAULT_PAGE_SIZE;
}

export function setListPageSize(list: PaginatedListId, pageSize: number): void {
  if (typeof localStorage === "undefined" || !PAGE_SIZES.has(pageSize)) return;

  localStorage.setItem(storageKey(list), String(pageSize));
}
