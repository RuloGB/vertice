<script lang="ts">
  import type { ScanReport } from "../../bindings/ScanReport";
  import { filterComponents, type ComponentFilter } from "../filterComponents";
  import { useI18n } from "../i18n/locale.svelte";
  import InventoryList from "../InventoryList.svelte";
  import InventoryToolbar from "../InventoryToolbar.svelte";
  import ScanDiagnostics from "../ScanDiagnostics.svelte";
  import { partitionDiagnostics } from "../scanDiagnostics";

  const i18n = useI18n();

  // Filter state lives in the shell so it survives navigating away and back.
  let {
    status,
    report,
    failureMessage,
    query,
    kind,
    onQueryChange,
    onKindChange,
    onReload,
  }: {
    status: "idle" | "loading" | "ready" | "failed";
    report: ScanReport | null;
    failureMessage: string | null;
    query: string;
    kind: ComponentFilter["kind"];
    onQueryChange: (query: string) => void;
    onKindChange: (kind: ComponentFilter["kind"]) => void;
    onReload: () => void;
  } = $props();

  const visibleComponents = $derived(filterComponents(report?.components ?? [], { kind, query }));
  const diagnostics = $derived(
    partitionDiagnostics(report?.rootsScanned ?? [], report?.issues ?? []),
  );
</script>

<section class="flex flex-col gap-5">
  <h1 class="text-xl font-semibold tracking-tight text-mist-100">{i18n.t("area.inventory")}</h1>

  <InventoryToolbar
    {query}
    {kind}
    reloading={status === "loading"}
    {onQueryChange}
    {onKindChange}
    {onReload}
  />

  {#if status === "idle" || status === "loading"}
    <div
      role="status"
      class="rounded-xl border border-line bg-ink-900/40 p-12 text-center text-sm text-mist-400"
    >
      {i18n.t("inventory.loading")}
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
    <ScanDiagnostics {diagnostics} />
    <InventoryList components={visibleComponents} />
  {/if}
</section>
