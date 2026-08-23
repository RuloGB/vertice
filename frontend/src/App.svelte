<script lang="ts">
  import { onMount } from "svelte";
  import type { ScanReport } from "./bindings/ScanReport";
  import { appTitle, APP_VERSION, PRODUCT_NAME } from "./lib/appTitle";
  import { createI18n, provideI18n } from "./lib/i18n/locale.svelte";
  import { areaLabelKey, DEFAULT_ROUTE, hasContent, type RouteId } from "./lib/navigation";
  import AgentsPage from "./lib/pages/AgentsPage.svelte";
  import HomePage from "./lib/pages/HomePage.svelte";
  import PlaceholderPage from "./lib/pages/PlaceholderPage.svelte";
  import ScanPage from "./lib/pages/ScanPage.svelte";
  import SkillsPage from "./lib/pages/SkillsPage.svelte";
  import SubscriptionsPage from "./lib/pages/SubscriptionsPage.svelte";
  import { isScanError, rescan, scan } from "./lib/scan";
  import { incidentCount, partitionDiagnostics } from "./lib/scanDiagnostics";
  import Sidebar from "./lib/Sidebar.svelte";
  import { SAMPLE_SUBSCRIPTIONS } from "./lib/subscriptions";

  type Status = "idle" | "loading" | "ready" | "failed";
  type ScanFailure =
    | { kind: "noRootsConfigured" }
    | { kind: "internal"; reason: string }
    | { kind: "unexpected" };

  const i18n = provideI18n(createI18n("en"));

  let route: RouteId = $state(DEFAULT_ROUTE);
  let status: Status = $state("idle");
  let report = $state<ScanReport | null>(null);
  let failure = $state<ScanFailure | null>(null);
  // Never shared, never reset on navigation — a shared query would silently
  // pre-filter the other page.
  let agentsQuery = $state("");
  let skillsQuery = $state("");
  // Read once at startup: renewal countdowns must not shift mid-session.
  const today = new Date();

  const title = $derived(appTitle(PRODUCT_NAME, APP_VERSION, i18n.t(areaLabelKey(route))));
  const failureMessage = $derived(failure === null ? null : scanFailureMessage(failure));
  const diagnostics = $derived(
    partitionDiagnostics(report?.rootsScanned ?? [], report?.issues ?? []),
  );
  const incidents = $derived(report === null ? 0 : incidentCount(diagnostics));

  $effect(() => {
    if (typeof document !== "undefined") {
      document.documentElement.lang = i18n.locale;
      document.title = title;
    }
  });

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

  async function runScan(source: "startup" | "reload"): Promise<void> {
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
    void runScan("startup");
  });
</script>

<div class="flex h-screen overflow-hidden bg-canvas text-mist-200">
  <Sidebar current={route} onNavigate={(next) => (route = next)} />

  <main class="flex-1 overflow-y-auto app-canvas-gradient">
    <div class={route === "home" ? "w-full px-8 py-8 2xl:px-10" : "mx-auto w-full max-w-6xl px-8 py-8"}>
      {#if route === "home"}
        <HomePage
          {report}
          {status}
          {failureMessage}
          {incidents}
          onNavigate={(next) => (route = next)}
          onRetry={() => void runScan("reload")}
        />
      {:else if route === "agents"}
        <AgentsPage
          {status}
          {report}
          {failureMessage}
          query={agentsQuery}
          {incidents}
          onQueryChange={(value) => (agentsQuery = value)}
          onReload={() => void runScan("reload")}
          onNavigate={(next) => (route = next)}
        />
      {:else if route === "skills"}
        <SkillsPage
          {status}
          {report}
          {failureMessage}
          query={skillsQuery}
          {incidents}
          onQueryChange={(value) => (skillsQuery = value)}
          onReload={() => void runScan("reload")}
          onNavigate={(next) => (route = next)}
        />
      {:else if route === "scan"}
        <ScanPage
          {status}
          {report}
          {failureMessage}
          {diagnostics}
          {incidents}
          onReload={() => void runScan("reload")}
        />
      {:else if route === "subscriptions"}
        <SubscriptionsPage subscriptions={SAMPLE_SUBSCRIPTIONS} {today} />
      {:else if !hasContent(route)}
        <PlaceholderPage {route} />
      {/if}
    </div>
  </main>
</div>
