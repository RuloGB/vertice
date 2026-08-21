<script lang="ts">
  import { onMount } from "svelte";
  import type { ScanReport } from "./bindings/ScanReport";
  import { filterComponents, type ComponentFilter } from "./lib/filterComponents";
  import {
    createI18n,
    provideI18n,
    resolveLocale,
    type SupportedLocale,
  } from "./lib/i18n/locale.svelte";
  import InventoryList from "./lib/InventoryList.svelte";
  import InventoryToolbar from "./lib/InventoryToolbar.svelte";
  import { isScanError, rescan, scan } from "./lib/scan";

  type Status = "idle" | "loading" | "ready" | "failed";
  type ScanFailure =
    | { kind: "noRootsConfigured" }
    | { kind: "internal"; reason: string }
    | { kind: "unexpected" };

  const i18n = provideI18n(createI18n(resolveLocale(browserLanguages())));

  let status: Status = $state("idle");
  let report = $state<ScanReport | null>(null);
  let failure = $state<ScanFailure | null>(null);
  let kind: ComponentFilter["kind"] = $state("all");
  let query = $state("");

  const visibleComponents = $derived(filterComponents(report?.components ?? [], { kind, query }));
  const title = $derived(i18n.t("app.title"));
  const failureMessage = $derived(failure === null ? null : scanFailureMessage(failure));

  $effect(() => {
    if (typeof document !== "undefined") {
      document.documentElement.lang = i18n.locale;
      document.title = title;
    }
  });

  function browserLanguages(): readonly string[] | string | null {
    if (typeof navigator === "undefined") {
      return null;
    }

    return navigator.languages.length > 0 ? navigator.languages : navigator.language;
  }

  function toScanFailure(error: unknown): ScanFailure {
    if (isScanError(error)) {
      return error.kind === "noRootsConfigured"
        ? { kind: "noRootsConfigured" }
        : { kind: "internal", reason: error.detail.reason };
    }
    return { kind: "unexpected" };
  }

  function scanFailureMessage(error: ScanFailure): string {
    if (error.kind === "noRootsConfigured") {
      return i18n.t("failure.noRootsConfigured");
    }
    if (error.kind === "internal") {
      return i18n.t("failure.internalReason", { reason: error.reason });
    }
    return i18n.t("failure.unexpected");
  }

  async function loadInventory(source: "startup" | "reload"): Promise<void> {
    status = "loading";
    failure = null;
    try {
      // Startup uses `scan`; reload uses `rescan`. No watchers or timers.
      const next = source === "startup" ? await scan() : await rescan();
      report = next;
      status = "ready";
    } catch (error) {
      failure = toScanFailure(error);
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

    <label class="flex items-center gap-2 self-start text-sm text-slate-300">
      <span>{i18n.t("app.languageLabel")}</span>
      <select
        aria-label={i18n.t("app.languageLabel")}
        value={i18n.locale}
        onchange={(event) => i18n.setLocale(event.currentTarget.value as SupportedLocale)}
        class="rounded-md border border-slate-700 bg-slate-900 px-3 py-1.5 text-sm text-slate-100 focus:border-slate-500 focus:outline-none"
      >
        <option value="en">{i18n.t("app.languageEnglish")}</option>
        <option value="es">{i18n.t("app.languageSpanish")}</option>
      </select>
    </label>

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
        {i18n.t("inventory.loading")}
      </div>
    {:else if status === "failed"}
      <div role="alert" class="rounded-lg border border-red-900 bg-red-950/40 p-6 text-sm text-red-200">
        <p class="font-medium">{i18n.t("failure.title")}</p>
        {#if failureMessage}
          <p class="mt-1 text-red-300/80">{failureMessage}</p>
        {/if}
      </div>
    {:else}
      <InventoryList components={visibleComponents} />
    {/if}
  </div>
</main>
