<script lang="ts">
  import type { Diagnostics } from "./scanDiagnostics";
  import { useI18n } from "./i18n/locale.svelte";

  const i18n = useI18n();

  let { diagnostics }: { diagnostics: Diagnostics } = $props();

  const hasDiagnostics = $derived(diagnostics.recoverableIssues.length > 0);
</script>

{#if hasDiagnostics}
  <section
    data-testid="scan-diagnostics"
    aria-label={i18n.t("diagnostics.title")}
    class="surface-card flex flex-col gap-5 border-warning/35 p-5 text-sm text-content-muted"
  >
    <section>
      <h2 class="font-semibold text-content">{i18n.t("diagnostics.recoverableIssues")}</h2>
      <ul class="mt-2 flex flex-col gap-1.5 text-content-muted">
        {#each diagnostics.recoverableIssues as issue (`${issue.reason}-${issue.path}`)}
          <li class="rounded-control bg-canvas/35 px-3 py-2">
            {issue.reason}{#if issue.path !== null} — {issue.path}{/if}
          </li>
        {/each}
      </ul>
    </section>
  </section>
{/if}
