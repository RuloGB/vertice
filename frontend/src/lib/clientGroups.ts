import type { ClientKind } from "../bindings/ClientKind";
import type { Location } from "../bindings/Location";

/** One deduplicated client group with its location count. */
export interface ClientGroup {
  /** Owning client of the root that produced these locations; null = shared root. */
  client: ClientKind | null;
  count: number;
}

/** Hardcoded proper nouns — never i18n keys (design §5/V8). */
export const CLIENT_LABEL: Record<ClientKind, string> = {
  claudeCode: "Claude Code",
  openCode: "OpenCode",
  codex: "Codex",
};

/** Fixed display order: claudeCode → openCode → codex → shared(null) last. */
const CLIENT_ORDER: (ClientKind | null)[] = ["claudeCode", "openCode", "codex", null];

/**
 * Deduplicate locations by `client`. Order is fixed and total:
 * claudeCode → openCode → codex → shared(null) last, regardless of
 * location order. Groups with count 0 are never emitted.
 */
export function groupLocationsByClient(locations: Location[]): ClientGroup[] {
  const counts = new Map<ClientKind | null, number>();

  for (const location of locations) {
    const key = location.client;
    counts.set(key, (counts.get(key) ?? 0) + 1);
  }

  const groups: ClientGroup[] = [];
  for (const client of CLIENT_ORDER) {
    const count = counts.get(client) ?? 0;
    if (count > 0) {
      groups.push({ client, count });
    }
  }

  return groups;
}
