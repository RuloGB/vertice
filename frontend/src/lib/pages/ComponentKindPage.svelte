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
</script>

<section class="flex flex-col gap-5">
  <div class="flex flex-wrap items-center justify-between gap-3">
    <h1 class="text-xl font-semibold tracking-tight text-mist-100">
      {i18n.t(areaLabelKey(KIND_ROUTE[kind]))}
    </h1>
    <IncidentIndicator {incidents} onclick={() => onNavigate("scan")} />
  </div>

  <ComponentToolbar {query} reloading={status === "loading"} {onQueryChange} {onReload} />

  {#if status === "idle" || status === "loading"}
    <div
      role="status"
      class="rounded-xl border border-line bg-ink-900/40 p-12 text-center text-sm text-mist-400"
    >
      {i18n.t("components.loading")}
    </div>
  {:else if status === "failed"}
    <div
      role="alert"
      class="rounded-xl border border-red-900/60 bg-red-950/30 p-6 text-sm text-red-200"
    >
      <p class="font-medium">{i18n.t("failure.title")}</p>
      {#if failureMessage}
        <p class="mt-1 text-red-300/80">{failureMessage}</p>
      {/if}
    </div>
  {:else}
    <ComponentList components={visible} />
  {/if}
</section>
