<script lang="ts">
  import type { ScanReport } from "../../bindings/ScanReport";
  import BrandMark from "../BrandMark.svelte";
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
    { key: "components", route: "scan" as RouteId, label: i18n.t("home.statComponents"), value: stats?.components },
    { key: "skills", route: "skills" as RouteId, label: i18n.t("home.statSkills"), value: stats?.skills },
    { key: "agents", route: "agents" as RouteId, label: i18n.t("home.statAgents"), value: stats?.agents },
    { key: "roots", route: "scan" as RouteId, label: i18n.t("home.statRoots"), value: stats?.roots },
  ]);
</script>

<section class="flex min-h-full flex-col gap-6 rounded-[1.75rem] bg-canvas p-2 text-content xl:gap-7">
  <div
    class="relative overflow-hidden rounded-3xl border border-stroke hero-gradient px-8 py-9 shadow-hero xl:px-12 xl:py-10"
  >
    <div
      class="pointer-events-none absolute -right-24 -top-24 size-72 rounded-full bg-interactive/35 blur-3xl"
      aria-hidden="true"
    ></div>
    <div
      class="pointer-events-none absolute -bottom-20 left-1/4 size-52 rounded-full bg-action/10 blur-3xl"
      aria-hidden="true"
    ></div>
    <div class="relative flex items-center justify-between gap-10">
      <div class="flex max-w-3xl flex-col gap-4">
        <span class="h-px w-12 bg-action" aria-hidden="true"></span>
        <h1 class="max-w-[18ch] text-balance text-4xl font-semibold leading-[1.05] tracking-[-0.045em] text-content xl:text-5xl">
          {i18n.t("home.greeting")}
        </h1>
        <p class="max-w-2xl text-pretty text-sm leading-7 text-content-muted xl:text-base">
          {i18n.t("home.subtitle")}
        </p>
      </div>
      <div class="hidden size-20 shrink-0 place-items-center xl:grid" aria-hidden="true">
        <BrandMark compact variant="sidebar" class="scale-[2]" />
      </div>
    </div>
  </div>

  <div class="grid gap-6 xl:grid-cols-[minmax(19rem,0.82fr)_minmax(0,2.18fr)] xl:items-stretch">
    <div
      class="relative flex min-h-36 flex-wrap items-center justify-between gap-5 overflow-hidden rounded-2xl border border-stroke bg-surface px-6 py-6 shadow-panel"
    >
      <div class="absolute inset-y-0 left-0 w-1 bg-interactive" aria-hidden="true"></div>
      <div class="flex flex-col gap-1">
        <h2 class="text-lg font-semibold tracking-tight text-content">{i18n.t("home.scanTitle")}</h2>
        {#if status === "failed"}
          <p class="text-sm text-danger">{i18n.t("home.scanFailed")}</p>
          {#if failureMessage}
            <p class="text-sm text-danger/80">{failureMessage}</p>
          {/if}
        {:else if status === "ready" && incidents === 0}
          <p class="text-sm text-success">{i18n.t("home.scanHealthy")}</p>
        {:else if status === "ready"}
          <p class="text-sm text-warning">
            {i18n.t("home.scanIssues", { count: incidents, ms: report?.durationMs ?? 0 })}
          </p>
        {:else}
          <p class="text-sm text-content-muted">{i18n.t("home.scanPending")}</p>
        {/if}
      </div>
      {#if status === "failed"}
        <button
          type="button"
          onclick={onRetry}
          class="inline-flex cursor-pointer items-center gap-2 rounded-lg bg-action px-5 py-2.5 text-sm font-semibold text-canvas transition-colors hover:bg-action/90 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-interactive-hover"
        >
          {i18n.t("home.scanRetry")}
        </button>
      {:else if status === "ready" && incidents > 0}
        <button
          type="button"
          onclick={() => onNavigate("scan")}
          class="inline-flex cursor-pointer items-center gap-2 rounded-lg border border-interactive-hover/60 px-5 py-2.5 text-sm font-medium text-content transition-colors hover:bg-surface-raised focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-interactive-hover"
        >
          {i18n.t("home.scanOpen")}
        </button>
      {/if}
    </div>

    {#if status === "ready"}
      <div class="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
      {#each tiles as tile (tile.key)}
        <button
          type="button"
          data-testid={`home-stat-${tile.key}`}
          onclick={() => onNavigate(tile.route)}
          class="group relative flex min-h-36 cursor-pointer flex-col justify-between overflow-hidden rounded-2xl border border-stroke bg-surface px-5 py-4 text-left shadow-stat transition-colors hover:border-interactive-hover/60 hover:bg-surface-raised focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-interactive-hover active:bg-surface-raised"
        >
          <span class="text-xs font-medium uppercase tracking-[0.16em] text-content-muted">{tile.label}</span>
          <span class="text-3xl font-semibold tabular-nums tracking-tight text-content">
            {tile.value ?? i18n.t("home.statsPending")}
          </span>
          <span class="absolute bottom-0 left-0 h-1 w-0 bg-action transition-all duration-200 group-hover:w-full group-focus-visible:w-full" aria-hidden="true"></span>
        </button>
      {/each}
      </div>
    {/if}
  </div>

  <div
    class="flex flex-wrap items-center justify-between gap-5 rounded-2xl border border-interactive/45 bg-surface-raised px-6 py-6 shadow-callout"
  >
    <div class="flex flex-col gap-1">
      <h2 class="text-lg font-semibold tracking-tight text-content">{i18n.t("home.ctaTitle")}</h2>
      <p class="max-w-xl text-sm leading-6 text-content-muted">{i18n.t("home.ctaBody")}</p>
    </div>
    <button
      type="button"
      onclick={() => onNavigate("agents")}
      class="inline-flex cursor-pointer items-center gap-2 rounded-lg bg-interactive px-5 py-2.5 text-sm font-semibold text-content transition-colors hover:bg-interactive-hover hover:text-canvas focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-action"
    >
      <NavIcon route="agents" />
      {i18n.t("home.ctaAction")}
    </button>
  </div>
</section>
