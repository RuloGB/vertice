<script lang="ts">
  import type { ScanReport } from "../../bindings/ScanReport";
  import { fetchLogFilePath } from "../appLog";
  import { useI18n } from "../i18n/locale.svelte";
  import { areaLabelKey } from "../navigation";
  import ScanIssueList from "../ScanIssueList.svelte";
  import type { Diagnostics } from "../scanDiagnostics";

  const i18n = useI18n();

  // Resolves independently of the scan report — a failure to resolve the
  // log path simply leaves the element unrendered rather than failing the
  // route (desktop-shell "The log-path command returns the path without
  // touching the file").
  let logPath = $state<string | null>(null);

  void (async () => {
    try {
      logPath = await fetchLogFilePath();
    } catch {
      logPath = null;
    }
  })();

  let {
    status,
    report,
    failureMessage,
    diagnostics,
    incidents,
    onReload,
  }: {
    status: "idle" | "loading" | "ready" | "failed";
    report: ScanReport | null;
    failureMessage: string | null;
    diagnostics: Diagnostics;
    incidents: number;
    onReload: () => void;
  } = $props();

  const reloading = $derived(status === "loading");
</script>

<section class="flex flex-col gap-6">
  <header class="flex items-end justify-between gap-4 border-b border-stroke pb-5">
    <div>
      <h1 class="text-2xl font-semibold tracking-tight text-content">
        {i18n.t(areaLabelKey("scan"))}
      </h1>
    </div>
    <div class="flex items-center gap-3">
      {#if report !== null}
        <span
          data-testid="scan-duration"
          class="rounded-full border border-stroke bg-surface px-3 py-1.5 text-xs font-semibold text-content-muted tabular-nums"
        >
          {i18n.t("scan.durationLabel")}: {i18n.t("scan.durationValue", { ms: report.durationMs })}
        </span>
      {/if}
      <button
        type="button"
        disabled={reloading}
        onclick={onReload}
        class="shadow-action rounded-control bg-action px-4 py-2.5 text-sm font-bold text-canvas transition-[background,transform] hover:bg-action/85 active:scale-[.98] disabled:cursor-not-allowed disabled:opacity-50"
      >
        {reloading ? i18n.t("toolbar.reloading") : i18n.t("toolbar.reload")}
      </button>
    </div>
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
              <span class="shrink-0 text-xs font-bold text-content-muted">
                {root.status === "notFound"
                  ? i18n.t("scan.rootNotFound")
                  : i18n.t("scan.rootFound")}
              </span>
            </li>
          {/each}
        </ul>
      </section>

      <section class="surface-card p-5">
        <h2 class="panel-heading">{i18n.t("scan.clientsTitle")}</h2>
        {#if report.clientPresence === null}
          <p class="mt-4 text-sm text-content-subtle">
            {i18n.t("scan.clientsUnsupportedPlatform")}
          </p>
        {:else}
          <ul class="mt-4 flex flex-col gap-2 text-sm text-content-muted">
            {#each report.clientPresence as record (record.label)}
              <li
                class="flex min-w-0 flex-wrap items-center justify-between gap-3 rounded-control bg-canvas/35 px-3 py-2.5"
              >
                <span class="min-w-0 truncate text-content">{record.label}</span>
                <span class="flex shrink-0 flex-wrap items-center justify-end gap-2 text-xs">
                  <span
                    class={[
                      "font-bold",
                      record.status === "detected" ? "text-success" : "text-content-muted",
                    ]}
                  >
                    {record.status === "detected"
                      ? i18n.t("scan.clientDetected")
                      : i18n.t("scan.clientNotDetected")}
                  </span>
                  {#if record.status === "detected"}
                    {#if record.installations.length > 0}
                      {#each record.installations as installation (installation.path)}
                        <span class="font-semibold text-content-muted" title={installation.path}>
                          {installation.version}
                        </span>
                      {/each}
                    {:else}
                      <span class="text-content-subtle">
                        {i18n.t("scan.clientVersionUnavailable")}
                      </span>
                    {/if}
                  {/if}
                </span>
              </li>
            {/each}
          </ul>
        {/if}
      </section>
    </div>

    <ScanIssueList {diagnostics} />

    {#if logPath !== null}
      <section class="surface-card p-5">
        <h2 class="panel-heading">{i18n.t("scan.logPathLabel")}</h2>
        <p class="mt-1 text-xs text-content-subtle">{i18n.t("scan.logPathHint")}</p>
        <code
          data-testid="log-path"
          class="mt-3 block break-all rounded-control bg-canvas/35 px-3 py-2.5 text-xs text-content-muted select-all"
        >{logPath}</code>
      </section>
    {/if}
  {/if}
</section>
