<script lang="ts">
  import type { ScanReport } from "../../bindings/ScanReport";
  import { useI18n } from "../i18n/locale.svelte";
  import NavIcon from "../NavIcon.svelte";
  import type { RouteId } from "../navigation";

  const i18n = useI18n();

  let {
    report,
    status,
    failureMessage,
    incidents,
    onNavigate,
    onRetry,
  }: {
    report: ScanReport | null;
    status: "idle" | "loading" | "ready" | "failed";
    failureMessage: string | null;
    incidents: number;
    onNavigate: (route: RouteId) => void;
    onRetry: () => void;
  } = $props();

  // Counts mirror the report already in memory; the landing page never scans.
  const stats = $derived.by(() => {
    if (report === null) {
      return null;
    }
    const components = report.components;
    return {
      components: components.length,
      skills: components.filter(({ kind }) => kind === "skill").length,
      agents: components.filter(({ kind }) => kind === "agent").length,
      roots: report.rootsScanned.length,
    };
  });

  const tiles = $derived([
    { key: "components", label: i18n.t("home.statComponents"), value: stats?.components },
    { key: "skills", label: i18n.t("home.statSkills"), value: stats?.skills },
    { key: "agents", label: i18n.t("home.statAgents"), value: stats?.agents },
    { key: "roots", label: i18n.t("home.statRoots"), value: stats?.roots },
  ]);
</script>

<section class="flex flex-col gap-8">
  <div
    class="relative overflow-hidden rounded-2xl border border-line bg-gradient-to-br from-ink-850 via-ink-900 to-ink-950 px-8 py-10"
  >
    <div
      class="pointer-events-none absolute -right-24 -top-24 size-72 rounded-full bg-accent-500/20 blur-3xl"
      aria-hidden="true"
    ></div>
    <div class="relative flex max-w-2xl flex-col gap-3">
      <h1 class="text-3xl font-semibold tracking-tight text-mist-100">
        {i18n.t("home.greeting")}
      </h1>
      <p class="text-sm leading-relaxed text-mist-300">{i18n.t("home.subtitle")}</p>
    </div>
  </div>

  <div
    class="flex flex-wrap items-center justify-between gap-4 rounded-xl border border-line bg-ink-800 px-6 py-5"
  >
    <div class="flex flex-col gap-1">
      <h2 class="text-sm font-medium text-mist-100">{i18n.t("home.scanTitle")}</h2>
      {#if status === "failed"}
        <p class="text-sm text-red-300">{i18n.t("home.scanFailed")}</p>
        {#if failureMessage}
          <p class="text-sm text-red-300/80">{failureMessage}</p>
        {/if}
      {:else if status === "ready" && incidents === 0}
        <p class="text-sm text-mist-400">{i18n.t("home.scanHealthy")}</p>
      {:else if status === "ready"}
        <p class="text-sm text-amber-300">
          {i18n.t("home.scanIssues", { count: incidents, ms: report?.durationMs ?? 0 })}
        </p>
      {:else}
        <p class="text-sm text-mist-400">{i18n.t("home.scanPending")}</p>
      {/if}
    </div>
    {#if status === "failed"}
      <button
        type="button"
        onclick={onRetry}
        class="inline-flex items-center gap-2 rounded-lg bg-accent-500 px-4 py-2 text-sm font-medium text-ink-950 transition-colors hover:bg-accent-400"
      >
        {i18n.t("home.scanRetry")}
      </button>
    {:else if status === "ready" && incidents > 0}
      <button
        type="button"
        onclick={() => onNavigate("scan")}
        class="inline-flex items-center gap-2 rounded-lg border border-line px-4 py-2 text-sm font-medium text-mist-100 transition-colors hover:bg-ink-700"
      >
        {i18n.t("home.scanOpen")}
      </button>
    {/if}
  </div>

  {#if status === "ready"}
    <div class="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
      {#each tiles as tile (tile.key)}
        <div class="flex flex-col gap-1 rounded-xl border border-line bg-ink-800 px-5 py-4">
          <span class="text-xs uppercase tracking-wide text-mist-400">{tile.label}</span>
          <span class="text-2xl font-semibold tabular-nums text-mist-100">
            {tile.value ?? i18n.t("home.statsPending")}
          </span>
        </div>
      {/each}
    </div>
  {/if}

  <div
    class="flex flex-wrap items-center justify-between gap-4 rounded-xl border border-line bg-ink-800 px-6 py-5"
  >
    <div class="flex flex-col gap-1">
      <h2 class="text-sm font-medium text-mist-100">{i18n.t("home.ctaTitle")}</h2>
      <p class="text-sm text-mist-400">{i18n.t("home.ctaBody")}</p>
    </div>
    <button
      type="button"
      onclick={() => onNavigate("agents")}
      class="inline-flex items-center gap-2 rounded-lg bg-accent-500 px-4 py-2 text-sm font-medium text-ink-950 transition-colors hover:bg-accent-400"
    >
      <NavIcon route="agents" />
      {i18n.t("home.ctaAction")}
    </button>
  </div>
</section>
