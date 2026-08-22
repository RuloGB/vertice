<script lang="ts">
  import { useI18n } from "./i18n/locale.svelte";

  const i18n = useI18n();

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
    class="inline-flex items-center gap-1.5 rounded-full border border-warning/45 bg-warning/10 px-3 py-1.5 text-xs font-bold text-warning transition-colors hover:bg-warning/18"
  >
    {i18n.t("incident.count", { count: incidents })}
  </button>
{/if}
