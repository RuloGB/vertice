import type { Component } from "../bindings/Component";

/**
 * Duplicate mark for the inventory UI: a consolidated component is duplicated
 * if and only if it was found at more than one location. T8 already merges
 * same-identity discoveries into one `Component`, so location count alone is
 * authoritative; never re-group by name or compare file contents here.
 */
export function isDuplicate(component: Component): boolean {
  return component.locations.length > 1;
}
