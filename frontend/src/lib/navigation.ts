/**
 * Sidebar navigation model. Pure data: no routing library, no history access,
 * no IPC. The shell owns the selected route as component state and renders the
 * matching page, so this module stays trivially testable.
 */

/** Every destination reachable from the sidebar, in declaration order. */
export const ROUTE_IDS = [
  "home",
  "agents",
  "skills",
  "clients",
  "mcp",
  "prompts",
  "scan",
  "subscriptions",
] as const;

export type RouteId = (typeof ROUTE_IDS)[number];

/** Landing page shown on startup. */
export const DEFAULT_ROUTE: RouteId = "home";

export type NavGroupId = "overview" | "library" | "data";

export interface NavGroup {
  readonly id: NavGroupId;
  readonly routes: readonly RouteId[];
}

/** Sidebar sections, in render order. Every route belongs to exactly one group. */
export const NAV_GROUPS: readonly NavGroup[] = [
  { id: "overview", routes: ["home"] },
  { id: "library", routes: ["agents", "skills", "clients", "mcp", "prompts"] },
  { id: "data", routes: ["scan", "subscriptions"] },
];

/**
 * Routes that render content of their own. Everything else renders an explicit
 * empty state instead of pretending to have data. Note that content is not the
 * same as a live backend read: `agents`, `skills`, and `scan` all reflect the
 * same startup scan, while `subscriptions` is populated from a local sample
 * until a billing source exists.
 */
const ROUTES_WITH_CONTENT: ReadonlySet<RouteId> = new Set<RouteId>([
  "home",
  "agents",
  "skills",
  "clients",
  "mcp",
  "scan",
  "subscriptions",
]);

export function isRouteId(value: string): value is RouteId {
  return (ROUTE_IDS as readonly string[]).includes(value);
}

export function hasContent(route: RouteId): boolean {
  return ROUTES_WITH_CONTENT.has(route);
}

export function navLabelKey(route: RouteId): `nav.${RouteId}` {
  return `nav.${route}`;
}

export function areaLabelKey(route: RouteId): `area.${RouteId}` {
  return `area.${route}`;
}

export function navGroupLabelKey(group: NavGroupId): `navGroup.${NavGroupId}` {
  return `navGroup.${group}`;
}
