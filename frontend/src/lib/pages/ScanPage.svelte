<script lang="ts">
  import type { ScanReport } from "../../bindings/ScanReport";
  import { useI18n } from "../i18n/locale.svelte";
  import { areaLabelKey } from "../navigation";
  import ScanIssueList from "../ScanIssueList.svelte";
  import type { Diagnostics } from "../scanDiagnostics";

  const i18n = useI18n();

  let {
    status,
    report,
    failureMessage,
    diagnostics,
    incidents,
  }: {
    status: "idle" | "loading" | "ready" | "failed";
    report: ScanReport | null;
    failureMessage: string | null;
    diagnostics: Diagnostics;
    incidents: number;
  } = $props();
</script>

<section class="flex flex-col gap-6">
  <header class="flex items-end justify-between gap-4 border-b border-stroke pb-5">
    <div>
      <h1 class="text-2xl font-semibold tracking-tight text-content">
        {i18n.t(areaLabelKey("scan"))}
      </h1>
    </div>
    {#if report !== null}
      <span
        data-testid="scan-duration"
        class="rounded-full border border-stroke bg-surface px-3 py-1.5 text-xs font-semibold text-content-muted tabular-nums"
      >
        {i18n.t("scan.durationLabel")}: {i18n.t("scan.durationValue", { ms: report.durationMs })}
      </span>
    {/if}
  </header>

  {#if status === "idle" || status === "loading"}
    <div role="status" class="surface-card p-12 text-center text-sm text-content-subtle">
      {i18n.t("home.scanPending")}
    </div>
  {:else if status === "failed"}
    <div role="alert" class="rounded-panel border border-danger/45 bg-danger/10 p-6 text-sm text-danger">
      <p class="font-semibold">{i18n.t("failure.title")}</p>
      {#if failureMessage}
        <p class="mt-1 text-danger/80">{failureMessage}</p>
      {/if}
    </div>
  {:else if report !== null}
    <div
      class={[
        "rounded-panel border p-5 text-sm font-semibold shadow-panel",
        incidents === 0
          ? "border-success/40 bg-success/10 text-success"
          : "border-warning/45 bg-warning/10 text-warning",
      ]}
    >
      {incidents === 0
        ? i18n.t("scan.verdictHealthy")
        : i18n.t("scan.verdictIssues", { count: incidents })}
    </div>

    <div class="grid gap-5 xl:grid-cols-2">
      <section class="surface-card p-5">
        <h2 class="panel-heading">{i18n.t("scan.rootsTitle")}</h2>
        <ul class="mt-4 flex flex-col gap-2 text-sm">
          {#each report.rootsScanned as root (root.id)}
            <li
              class="flex min-w-0 items-center justify-between gap-3 rounded-control bg-canvas/35 px-3 py-2.5"
            >
              <span class="min-w-0 truncate text-content-muted" title={root.path}>
                {root.path}
              </span>
              <span
                class={[
                  "shrink-0 text-xs font-bold",
                  root.status === "notFound" ? "text-danger" : "text-success",
                ]}
              >
                {root.status === "notFound"
                  ? i18n.t("scan.rootNotFound")
                  : i18n.t("scan.rootFound")}
              </span>
            </li>
          {/each}
        </ul>
      </section>

      <section class="surface-card p-5">
        <h2 class="panel-heading">{i18n.t("scan.installationsTitle")}</h2>
        {#if report.installations.length === 0}
          <p class="mt-4 text-sm text-content-subtle">{i18n.t("scan.installationsEmpty")}</p>
        {:else}
          <ul class="mt-4 flex flex-col gap-2 text-sm text-content-muted">
            {#each report.installations as installation (installation.path)}
              <li class="rounded-control bg-canvas/35 px-3 py-2.5 break-all">
                {installation.client} {installation.version} — {installation.path}
              </li>
            {/each}
          </ul>
        {/if}
      </section>
    </div>

    <ScanIssueList {diagnostics} />
  {/if}
</section>
