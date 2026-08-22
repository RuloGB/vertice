<script lang="ts">
  import { onMount } from "svelte";
  import type { ScanReport } from "./bindings/ScanReport";
  import { appTitle, APP_VERSION, PRODUCT_NAME } from "./lib/appTitle";
  import type { ComponentFilter } from "./lib/filterComponents";
  import { createI18n, provideI18n, resolveLocale } from "./lib/i18n/locale.svelte";
  import { areaLabelKey, DEFAULT_ROUTE, hasContent, type RouteId } from "./lib/navigation";
  import HomePage from "./lib/pages/HomePage.svelte";
  import InventoryPage from "./lib/pages/InventoryPage.svelte";
  import PlaceholderPage from "./lib/pages/PlaceholderPage.svelte";
  import { isScanError, rescan, scan } from "./lib/scan";
  import Sidebar from "./lib/Sidebar.svelte";

  type Status = "idle" | "loading" | "ready" | "failed";
  type ScanFailure =
    | { kind: "noRootsConfigured" }
    | { kind: "internal"; reason: string }
    | { kind: "unexpected" };

  const i18n = provideI18n(createI18n(resolveLocale(browserLanguages())));

  let route: RouteId = $state(DEFAULT_ROUTE);
  let status: Status = $state("idle");
  let report = $state<ScanReport | null>(null);
  let failure = $state<ScanFailure | null>(null);
  let kind: ComponentFilter["kind"] = $state("all");
  let query = $state("");

  const title = $derived(appTitle(PRODUCT_NAME, APP_VERSION, i18n.t(areaLabelKey(route))));
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

<div class="flex h-screen overflow-hidden bg-ink-950 text-mist-200">
  <Sidebar current={route} onNavigate={(next) => (route = next)} />

  <main class="flex-1 overflow-y-auto">
    <div class="mx-auto w-full max-w-5xl px-8 py-8">
      {#if route === "home"}
        <HomePage {report} onNavigate={(next) => (route = next)} />
      {:else if route === "inventory"}
        <InventoryPage
          {status}
          {report}
          {failureMessage}
          {query}
          {kind}
          onQueryChange={(value) => (query = value)}
          onKindChange={(value) => (kind = value)}
          onReload={() => void loadInventory("reload")}
        />
      {:else if !hasContent(route)}
        <PlaceholderPage {route} />
      {/if}
    </div>
  </main>
</div>
