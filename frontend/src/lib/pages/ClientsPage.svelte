<script lang="ts">
  import type { ClientPresence } from "../../bindings/ClientPresence";
  import type { ScanReport } from "../../bindings/ScanReport";
  import { useI18n } from "../i18n/locale.svelte";

  const i18n = useI18n();

  let {
    report,
    status,
    failureMessage,
  }: {
    report: ScanReport | null;
    status: "idle" | "loading" | "ready" | "failed";
    failureMessage: string | null;
  } = $props();

  const clients = [
    { id: "claudeCode", name: "Claude Code", owner: "Anthropic", mark: "C", tone: "#d6a77a" },
    { id: "openCode", name: "OpenCode", owner: "SST", mark: "O", tone: "#a78bfa" },
    { id: "codex", name: "Codex", owner: "OpenAI", mark: "*", tone: "#54d68a" },
  ] as const;

  const presenceFor = (clientId: string): ClientPresence | undefined => {
    const records = report?.clientPresence ?? [];
    return records.find((record) => {
      const label = record.label.toLowerCase();
      return clientId === "claudeCode"
        ? label.includes("claude")
        : clientId === "openCode"
          ? label.includes("opencode")
          : label.includes("codex");
    });
  };

  const loading = $derived(status === "idle" || status === "loading");
</script>

<section class="flex flex-col gap-6">
  <header class="flex flex-col gap-2 border-b border-stroke pb-5">
    <h1 class="text-2xl font-semibold tracking-tight text-content">{i18n.t("area.clients")}</h1>
    <p class="text-sm text-content-muted">{i18n.t("clients.intro")}</p>
  </header>

  {#if loading}
    <div role="status" class="surface-card p-12 text-center text-sm text-content-subtle">
      {i18n.t("home.scanPending")}
    </div>
  {:else if status === "failed"}
    <div role="alert" class="rounded-panel border border-danger/45 bg-danger/10 p-6 text-sm text-danger">
      <p class="font-semibold">{i18n.t("failure.title")}</p>
      {#if failureMessage}<p class="mt-1 text-danger/80">{failureMessage}</p>{/if}
    </div>
  {:else if report?.clientPresence === null}
    <div role="status" class="surface-card p-8 text-center text-sm text-content-subtle">
      {i18n.t("scan.clientsUnsupportedPlatform")}
    </div>
  {:else}
    <div class="grid gap-5 lg:grid-cols-2 2xl:grid-cols-3">
      {#each clients as client (client.id)}
        {@const presence = presenceFor(client.id)}
        {@const detected = presence?.status === "detected"}
        {@const versions = presence?.installations.map((installation) => installation.version).join(", ")}
        <article class="surface-card flex min-h-80 flex-col gap-6 p-5 transition-transform duration-200 hover:-translate-y-0.5">
          <div class="flex items-start justify-between gap-4">
            <div class="flex items-center gap-3">
              <div
                class="flex size-12 items-center justify-center rounded-2xl border text-xl font-bold text-canvas"
                style={`background: ${client.tone}; border-color: ${client.tone}`}
                aria-hidden="true"
              >{client.mark}</div>
              <div>
                <h2 class="text-lg font-semibold text-content">{client.name}</h2>
                <p class="text-sm text-content-muted">{i18n.t("clients.owner", { owner: client.owner })}</p>
              </div>
            </div>
            <span class={["rounded-full border px-2.5 py-1 text-xs font-bold", detected ? "border-success/40 bg-success/10 text-success" : "border-stroke bg-canvas/40 text-content-muted"]}>
              {detected ? i18n.t("scan.clientDetected") : i18n.t("scan.clientNotDetected")}
            </span>
          </div>

          <div class="flex flex-col gap-4">
            <div class="flex items-center justify-between text-sm">
              <span class="text-content-muted">{i18n.t("clients.version")}</span>
              <span class="font-semibold text-content">{detected && versions ? versions : i18n.t("scan.clientVersionUnavailable")}</span>
            </div>
            <div class="flex flex-col gap-2">
              <div class="flex justify-between text-xs font-semibold text-content-muted"><span>{i18n.t("clients.weeklyUsage")}</span><span>0%</span></div>
              <div class="h-2 overflow-hidden rounded-full bg-canvas"><div class="h-full w-0 rounded-full bg-action"></div></div>
            </div>
            <div class="flex flex-col gap-2">
              <div class="flex justify-between text-xs font-semibold text-content-muted"><span>{i18n.t("clients.monthlyUsage")}</span><span>0%</span></div>
              <div class="h-2 overflow-hidden rounded-full bg-canvas"><div class="h-full w-0 rounded-full bg-interactive-hover"></div></div>
            </div>
          </div>

          <p class="mt-auto border-t border-stroke pt-4 text-xs leading-relaxed text-content-subtle">{i18n.t("clients.usageUnavailable")}</p>
        </article>
      {/each}
    </div>
  {/if}
</section>
