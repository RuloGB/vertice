import type { Component } from "../bindings/Component";
import type { ClientKind } from "../bindings/ClientKind";

type SharedRootConsumers = Partial<Record<Component["kind"], Partial<Record<string, ClientKind[]>>>>;

const SHARED_ROOT_CONSUMERS: SharedRootConsumers = {
  skill: {
    "agents-skills": ["openCode", "codex"],
  },
};

/**
 * Duplicate mark for the inventory UI: true only when one supported client can
 * consume both a shared-root copy and that same client's specific-root copy.
 * This is intentionally narrower than consolidation (`locations.length > 1`):
 * never regroup by name, inspect paths, compare files, or branch on provenance.
 */
export function isDuplicate(component: Component): boolean {
  const rootConsumers = SHARED_ROOT_CONSUMERS[component.kind];
  if (!rootConsumers) {
    return false;
  }

  const clientLocations = new Set(
    component.locations.flatMap(({ client }) => (client === null ? [] : [client])),
  );

  return component.locations.some(({ client, root }) => {
    if (client !== null) {
      return false;
    }
    return (rootConsumers[root] ?? []).some((consumer) => clientLocations.has(consumer));
  });
}
