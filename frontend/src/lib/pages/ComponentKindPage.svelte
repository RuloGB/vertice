<script module lang="ts">
  import type { ScanReport } from "../../bindings/ScanReport";
  import type { RouteId } from "../navigation";

  export type KindPageProps = {
    status: "idle" | "loading" | "ready" | "failed";
    report: ScanReport | null;
    failureMessage: string | null;
    query: string;
    incidents: number;
    onQueryChange: (query: string) => void;
    onReload: () => void;
    onNavigate: (route: RouteId) => void;
  };
</script>

<script lang="ts">
  import type { ComponentKind } from "../../bindings/ComponentKind";
  import { filterComponents } from "../filterComponents";
  import { useI18n } from "../i18n/locale.svelte";
  import ComponentList from "../ComponentList.svelte";
  import ComponentToolbar from "../ComponentToolbar.svelte";
  import IncidentIndicator from "../IncidentIndicator.svelte";
  import { areaLabelKey } from "../navigation";

  const i18n = useI18n();

  let {
    kind,
    status,
    report,
    failureMessage,
    query,
    incidents,
    onQueryChange,
    onReload,
    onNavigate,
  }: KindPageProps & { kind: ComponentKind } = $props();

  const KIND_ROUTE = { agent: "agents", skill: "skills" } as const satisfies Record<
    ComponentKind,
    RouteId
  >;

  const visible = $derived(filterComponents(report?.components ?? [], { kind, query }));
  const PAGE_SIZES = [5, 10, 15] as const;

  let page = $state(1);
  let pageSize = $state<(typeof PAGE_SIZES)[number]>(PAGE_SIZES[0]);
  let previousQuery = $state("");

  const pageCount = $derived(Math.max(1, Math.ceil(visible.length / pageSize)));
  const pageStart = $derived((page - 1) * pageSize);
  const pageComponents = $derived(visible.slice(pageStart, pageStart + pageSize));
  const rangeStart = $derived(visible.length === 0 ? 0 : pageStart + 1);
  const rangeEnd = $derived(Math.min(pageStart + pageComponents.length, visible.length));

  $effect(() => {
    if (query !== previousQuery) {
      page = 1;
      previousQuery = query;
    }
  });

  $effect(() => {
    if (page > pageCount) {
      page = pageCount;
    }
  });

  function onPageSizeChange(event: Event): void {
    pageSize = Number((event.currentTarget as HTMLSelectElement).value) as (typeof PAGE_SIZES)[number];
    page = 1;
  }
</script>

<section class="flex flex-col gap-6">
  <div class="flex flex-wrap items-center justify-between gap-3">
    <h1 class="text-2xl font-semibold tracking-tight text-content">
      {i18n.t(areaLabelKey(KIND_ROUTE[kind]))}
    </h1>
    <IncidentIndicator {incidents} onclick={() => onNavigate("scan")} />
  </div>

  <ComponentToolbar {query} reloading={status === "loading"} {onQueryChange} {onReload} />

  {#if status === "idle" || status === "loading"}
    <div
      role="status"
      class="rounded-panel border border-stroke bg-surface/60 p-12 text-center text-sm text-content-subtle"
    >
      {i18n.t("components.loading")}
    </div>
  {:else if status === "failed"}
    <div
      role="alert"
      class="rounded-panel border border-danger/45 bg-danger/10 p-6 text-sm text-danger"
    >
      <p class="font-medium">{i18n.t("failure.title")}</p>
      {#if failureMessage}
        <p class="mt-1 text-danger/80">{failureMessage}</p>
      {/if}
    </div>
  {:else}
    <ComponentList components={pageComponents} />
    {#if visible.length > 0}
      <nav
        aria-label={i18n.t("components.paginationPage", { current: page, total: pageCount })}
        class="surface-card flex flex-wrap items-center justify-between gap-4 p-4"
      >
        <p class="text-sm text-content-muted" aria-live="polite">
          {i18n.t("components.paginationSummary", {
            from: rangeStart,
            to: rangeEnd,
            total: visible.length,
          })}
        </p>

        <div class="flex flex-wrap items-center gap-2">
          <label class="flex items-center gap-2 text-sm text-content-muted">
            <span>{i18n.t("components.paginationPageSize")}</span>
            <select
              aria-label={i18n.t("components.paginationPageSize")}
              value={pageSize}
              onchange={onPageSizeChange}
              class="rounded-control border border-stroke bg-canvas/45 px-2.5 py-2 text-sm text-content focus:border-interactive-hover focus:outline-none"
            >
              {#each PAGE_SIZES as size (size)}
                <option value={size}>{size}</option>
              {/each}
            </select>
          </label>

          <div
            class="flex items-center gap-1"
            aria-label={i18n.t("components.paginationPage", { current: page, total: pageCount })}
          >
            <button
              type="button"
              aria-label={i18n.t("components.paginationFirst")}
              disabled={page === 1}
              onclick={() => (page = 1)}
              class="rounded-control border border-stroke px-2.5 py-2 text-sm text-content transition-colors hover:bg-surface-raised disabled:cursor-not-allowed disabled:opacity-45 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-interactive-hover"
            >«</button>
            <button
              type="button"
              aria-label={i18n.t("components.paginationPrevious")}
              disabled={page === 1}
              onclick={() => (page -= 1)}
              class="rounded-control border border-stroke px-2.5 py-2 text-sm text-content transition-colors hover:bg-surface-raised disabled:cursor-not-allowed disabled:opacity-45 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-interactive-hover"
            >‹</button>
            <span class="min-w-20 px-2 text-center text-sm tabular-nums text-content-muted">
              {i18n.t("components.paginationPage", { current: page, total: pageCount })}
            </span>
            <button
              type="button"
              aria-label={i18n.t("components.paginationNext")}
              disabled={page === pageCount}
              onclick={() => (page += 1)}
              class="rounded-control border border-stroke px-2.5 py-2 text-sm text-content transition-colors hover:bg-surface-raised disabled:cursor-not-allowed disabled:opacity-45 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-interactive-hover"
            >›</button>
            <button
              type="button"
              aria-label={i18n.t("components.paginationLast")}
              disabled={page === pageCount}
              onclick={() => (page = pageCount)}
              class="rounded-control border border-stroke px-2.5 py-2 text-sm text-content transition-colors hover:bg-surface-raised disabled:cursor-not-allowed disabled:opacity-45 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-interactive-hover"
            >»</button>
          </div>
        </div>
      </nav>
    {/if}
  {/if}
</section>
