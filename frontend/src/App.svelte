<script lang="ts">
  import { onMount } from "svelte";
  import type { ScanReport } from "./bindings/ScanReport";
  import { appTitle } from "./lib/appTitle";
  import { filterComponents, type ComponentFilter } from "./lib/filterComponents";
  import InventoryList from "./lib/InventoryList.svelte";
  import InventoryToolbar from "./lib/InventoryToolbar.svelte";
  import { isScanError, rescan, scan } from "./lib/scan";

  type Status = "idle" | "loading" | "ready" | "failed";

  const title = appTitle("Vertice", "0.1.0");

  let status: Status = $state("idle");
  let report = $state<ScanReport | null>(null);
  let failureMessage = $state<string | null>(null);
  let kind: ComponentFilter["kind"] = $state("all");
  let query = $state("");

  const visibleComponents = $derived(filterComponents(report?.components ?? [], { kind, query }));

  function scanErrorMessage(error: unknown): string {
    if (isScanError(error)) {
      return error.kind === "noRootsConfigured"
        ? "No search roots are configured."
        : `Internal scan failure: ${error.detail.reason}`;
    }
    return "The scan failed unexpectedly.";
  }

  async function loadInventory(source: "startup" | "reload"): Promise<void> {
    status = "loading";
    failureMessage = null;
    try {
      // Startup uses `scan`; reload uses `rescan`. No watchers or timers.
      const next = source === "startup" ? await scan() : await rescan();
      report = next;
      status = "ready";
    } catch (error) {
      failureMessage = scanErrorMessage(error);
      status = "failed";
    }
  }

  onMount(() => {
    void loadInventory("startup");
  });
</script>

<main class="min-h-screen bg-slate-950 text-slate-100">
  <div class="mx-auto flex max-w-4xl flex-col gap-6 px-6 py-8">
    <h1 class="text-2xl font-semibold">{title}</h1>

    <InventoryToolbar
      {query}
      {kind}
      reloading={status === "loading"}
      onQueryChange={(value) => (query = value)}
      onKindChange={(value) => (kind = value)}
      onReload={() => void loadInventory("reload")}
    />

    {#if status === "idle" || status === "loading"}
      <div role="status" class="rounded-lg border border-slate-800 p-10 text-center text-sm text-slate-400">
        Scanning for installed components...
      </div>
    {:else if status === "failed"}
      <div role="alert" class="rounded-lg border border-red-900 bg-red-950/40 p-6 text-sm text-red-200">
        <p class="font-medium">Inventory scan failed.</p>
        {#if failureMessage}
          <p class="mt-1 text-red-300/80">{failureMessage}</p>
        {/if}
      </div>
    {:else}
      <InventoryList components={visibleComponents} />
    {/if}
  </div>
</main>
