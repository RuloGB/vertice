import type { Component } from "../bindings/Component";
import type { ComponentKind } from "../bindings/ComponentKind";

/** View-only filter applied to the in-memory report; never triggers a scan. */
export interface ComponentFilter {
  /** Component kind to keep; the literal `"all"` keeps every kind. */
  kind: "all" | ComponentKind;
  /** Case-insensitive name substring; an empty string matches every name. */
  query: string;
}

/**
 * Pure, non-mutating view filter over consolidated report components.
 * Keeps a component when its kind matches (or the kind is `"all"`) and its
 * name contains the query, compared case-insensitively. The source array is
 * never modified; a new array is always returned.
 */
export function filterComponents(components: Component[], filter: ComponentFilter): Component[] {
  const query = filter.query.toLowerCase();
  return components.filter(
    (component) =>
      (filter.kind === "all" || component.kind === filter.kind) &&
      component.name.toLowerCase().includes(query),
  );
}
