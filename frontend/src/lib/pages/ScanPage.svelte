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

<section class="flex flex-col gap-5">
  <h1 class="text-xl font-semibold tracking-tight text-mist-100">{i18n.t(areaLabelKey("scan"))}</h1>

  {#if status === "idle" || status === "loading"}
    <div
      role="status"
      class="rounded-xl border border-line bg-ink-900/40 p-12 text-center text-sm text-mist-400"
    >
      {i18n.t("home.scanPending")}
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
  {:else if report !== null}
    <div
      class="rounded-xl border border-line bg-ink-800 px-5 py-4 text-sm"
      class:text-mist-100={incidents === 0}
      class:text-amber-300={incidents > 0}
    >
      {incidents === 0
        ? i18n.t("scan.verdictHealthy")
        : i18n.t("scan.verdictIssues", { count: incidents })}
    </div>

    <section class="rounded-xl border border-line bg-ink-800 p-4">
      <h2 class="font-medium text-mist-100">{i18n.t("scan.rootsTitle")}</h2>
      <ul class="mt-2 flex flex-col gap-1 text-sm text-mist-300">
        {#each report.rootsScanned as root (root.id)}
          <li class="flex items-center justify-between gap-3">
            <span>{root.path}</span>
            <span class={root.status === "notFound" ? "text-red-300" : "text-mist-400"}>
              {root.status === "notFound" ? i18n.t("scan.rootNotFound") : i18n.t("scan.rootFound")}
            </span>
          </li>
        {/each}
      </ul>
    </section>

    <section class="rounded-xl border border-line bg-ink-800 p-4">
      <h2 class="font-medium text-mist-100">{i18n.t("scan.installationsTitle")}</h2>
      {#if report.installations.length === 0}
        <p class="mt-2 text-sm text-mist-400">{i18n.t("scan.installationsEmpty")}</p>
      {:else}
        <ul class="mt-2 flex flex-col gap-1 text-sm text-mist-300">
          {#each report.installations as installation (installation.path)}
            <li>{installation.client} {installation.version} — {installation.path}</li>
          {/each}
        </ul>
      {/if}
    </section>

    <div class="text-sm text-mist-400">
      {i18n.t("scan.durationLabel")}: {i18n.t("scan.durationValue", { ms: report.durationMs })}
    </div>

    <ScanIssueList {diagnostics} />
  {/if}
</section>
