<script lang="ts">
  import { useI18n } from "./i18n/locale.svelte";

  const i18n = useI18n();

  // Discreet, not alarming: a small pill that links to the full scan report.
  let {
    incidents,
    onclick,
  }: {
    incidents: number;
    onclick: () => void;
  } = $props();
</script>

{#if incidents > 0}
  <button
    type="button"
    data-testid="incident-indicator"
    aria-label={i18n.t("incident.label")}
    title={i18n.t("incident.action")}
    {onclick}
    class="inline-flex items-center gap-1.5 rounded-full border border-amber-500/40 bg-amber-500/10 px-3 py-1 text-xs font-medium text-amber-300 transition-colors hover:bg-amber-500/20"
  >
    {i18n.t("incident.count", { count: incidents })}
  </button>
{/if}
