<script lang="ts">
  import type { ClientInstallSlot } from "../../bindings/ClientInstallSlot";
  import type { ClientPresence } from "../../bindings/ClientPresence";
  import type { FreshnessReport } from "../../bindings/FreshnessReport";
  import type { FreshnessSettings } from "../../bindings/FreshnessSettings";
  import type { ScanReport } from "../../bindings/ScanReport";
  import { fetchFreshness, fetchFreshnessSettings, setFreshnessSettings } from "../freshness";
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

  // Slots are the stable identity; `label` is display copy and MUST NOT be
  // matched on (`client-installation-detector` spec).
  const clients = [
    {
      id: "claudeCode",
      name: "Claude Code",
      owner: "Anthropic",
      mark: "C",
      tone: "#d6a77a",
      slots: ["claudeCodeNpm", "claudeCodeBundled"] as ClientInstallSlot[],
    },
    {
      id: "openCode",
      name: "OpenCode",
      owner: "SST",
      mark: "O",
      tone: "#a78bfa",
      slots: ["openCodeNpm"] as ClientInstallSlot[],
    },
    {
      id: "codex",
      name: "Codex",
      owner: "OpenAI",
      mark: "*",
      tone: "#54d68a",
      slots: ["codexStandalone"] as ClientInstallSlot[],
    },
  ] as const;

  const presenceFor = (slots: readonly ClientInstallSlot[]): ClientPresence | undefined =>
    (report?.clientPresence ?? []).find((record) => slots.includes(record.slot));

  const loading = $derived(status === "idle" || status === "loading");

  // Freshness resolves independently of the scan and never blocks first
  // render (design §1, §9): the page paints, then the badge settles.
  let settings = $state<FreshnessSettings | null>(null);
  let freshness = $state<FreshnessReport | null>(null);
  let lookupFailed = $state(false);

  // Every lookup carries a token. A response whose token is stale — the
  // user toggled the check off, or started a newer lookup, while it was in
  // flight — is discarded instead of painting a verdict nobody asked for.
  let lookupToken = 0;

  async function loadFreshness(): Promise<void> {
    const token = ++lookupToken;
    freshness = null;
    lookupFailed = false;
    try {
      const result = await fetchFreshness();
      if (token !== lookupToken) return;
      freshness = result;
    } catch {
      if (token !== lookupToken) return;
      // "Gave up" IS Unknown (design §9) — never an error surface.
      lookupFailed = true;
    }
  }

  function cancelLookup(): void {
    lookupToken += 1;
    freshness = null;
    lookupFailed = false;
  }

  void (async () => {
    let resolved: FreshnessSettings;
    try {
      resolved = await fetchFreshnessSettings();
    } catch {
      // Settings unreadable: stay silent rather than guessing that the
      // check is on. No request is issued.
      return;
    }
    settings = resolved;
    if (resolved.enabled) {
      await loadFreshness();
    }
  })();

  type Badge = { kind: "pending" | "upToDate" | "outdated" | "unknown"; text: string };

  const badgeFor = (presence: ClientPresence | undefined): Badge | null => {
    if (!presence || presence.status !== "detected" || !settings?.enabled) {
      return null;
    }
    if (lookupFailed) {
      return { kind: "unknown", text: i18n.t("freshness.unknown") };
    }
    if (!freshness) {
      return { kind: "pending", text: i18n.t("freshness.pending") };
    }

    const check = freshness.checks.find(
      (candidate) => candidate.subject.clientInstallation.slot === presence.slot,
    );
    if (!check) {
      return { kind: "unknown", text: i18n.t("freshness.unknown") };
    }

    const verdict = check.verdict;
    if (verdict === "upToDate") {
      return { kind: "upToDate", text: i18n.t("freshness.upToDate") };
    }
    if ("outdated" in verdict) {
      return {
        kind: "outdated",
        text: i18n.t("freshness.outdated", { latest: verdict.outdated.latest }),
      };
    }
    return { kind: "unknown", text: i18n.t("freshness.unknown") };
  };

  // Unknown is a first-class state, never danger styling: "we could not
  // tell" is not a failure the user caused or can fix.
  const badgeTone: Record<Badge["kind"], string> = {
    pending: "border-stroke bg-canvas/40 text-content-muted",
    upToDate: "border-success/40 bg-success/10 text-success",
    outdated: "border-action/45 bg-action/10 text-action",
    unknown: "border-stroke bg-canvas/40 text-content-subtle",
  };

  const showDisclosure = $derived(settings !== null && !settings.disclosureSeen);

  async function dismissDisclosure(): Promise<void> {
    if (!settings) return;
    settings = await setFreshnessSettings(settings.enabled, true);
  }

  async function toggleEnabled(event: Event): Promise<void> {
    const enabled = (event.currentTarget as HTMLInputElement).checked;
    const updated = await setFreshnessSettings(enabled, settings?.disclosureSeen ?? true);
    settings = updated;
    if (updated.enabled) {
      // Re-enabling must actually re-run the check. Clearing the state
      // without re-fetching left every card stuck on the pending copy
      // until the page was remounted.
      await loadFreshness();
    } else {
      // Disabled means no further outbound request of any kind, no stale
      // verdicts left on screen, and any in-flight response discarded.
      cancelLookup();
    }
  }
</script>

<section class="flex flex-col gap-6">
  <header class="flex flex-col gap-2 border-b border-stroke pb-5">
    <h1 class="text-2xl font-semibold tracking-tight text-content">{i18n.t("area.clients")}</h1>
    <p class="text-sm text-content-muted">{i18n.t("clients.intro")}</p>

    <div class="mt-2 flex flex-wrap items-center justify-between gap-3">
      <label class="flex items-center gap-3 text-sm">
        <input
          type="checkbox"
          class="size-4 accent-action"
          aria-label={i18n.t("freshness.settingToggleAria")}
          checked={settings?.enabled ?? false}
          onchange={toggleEnabled}
        />
        <span class="flex flex-col">
          <span class="font-medium text-content">{i18n.t("freshness.settingLabel")}</span>
          <span class="text-xs text-content-subtle">{i18n.t("freshness.settingDescription")}</span>
        </span>
      </label>
    </div>

    {#if showDisclosure}
      <div class="rounded-panel border border-stroke bg-canvas/40 p-4 text-sm">
        <p class="font-semibold text-content">{i18n.t("freshness.disclosureTitle")}</p>
        <p class="mt-1 text-content-muted">{i18n.t("freshness.disclosureBody")}</p>
        <button
          type="button"
          class="mt-3 rounded-full border border-stroke px-3 py-1 text-xs font-semibold text-content transition-colors hover:bg-interactive-hover"
          onclick={dismissDisclosure}
        >{i18n.t("freshness.disclosureDismiss")}</button>
      </div>
    {/if}
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
        {@const presence = presenceFor(client.slots)}
        {@const detected = presence?.status === "detected"}
        {@const badge = badgeFor(presence)}
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
            {#if badge}
              <div class="flex justify-end">
                <span
                  data-testid="freshness-badge"
                  class={["rounded-full border px-2.5 py-1 text-xs font-semibold", badgeTone[badge.kind]]}
                >{badge.text}</span>
              </div>
            {/if}
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
