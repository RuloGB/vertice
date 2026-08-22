<script lang="ts">
  import type { Diagnostics } from "./scanDiagnostics";
  import { useI18n } from "./i18n/locale.svelte";

  const i18n = useI18n();

  let { diagnostics }: { diagnostics: Diagnostics } = $props();

  const hasDiagnostics = $derived(
    diagnostics.missingClientIssues.length > 0 || diagnostics.remainingRecoverableIssues.length > 0,
  );
</script>

{#if hasDiagnostics}
  <section
    data-testid="scan-diagnostics"
    aria-label={i18n.t("diagnostics.title")}
    class="flex flex-col gap-3 rounded-xl border border-line bg-ink-800 p-4 text-sm text-mist-300"
  >
    {#if diagnostics.missingClientIssues.length > 0}
      <section>
        <h2 class="font-medium text-mist-100">{i18n.t("diagnostics.missingClient")}</h2>
        <ul class="mt-1 flex flex-col gap-1 text-mist-400">
          {#each diagnostics.missingClientIssues as issue (`${issue.reason}-${issue.path}`)}
            <li>{issue.reason} — {issue.path}</li>
          {/each}
        </ul>
      </section>
    {/if}

    {#if diagnostics.remainingRecoverableIssues.length > 0}
      <section>
        <h2 class="font-medium text-mist-100">{i18n.t("diagnostics.recoverableIssues")}</h2>
        <ul class="mt-1 flex flex-col gap-1 text-mist-400">
          {#each diagnostics.remainingRecoverableIssues as issue (`${issue.reason}-${issue.path}`)}
            <li>
              {issue.reason}{#if issue.path !== null} — {issue.path}{/if}
            </li>
          {/each}
        </ul>
      </section>
    {/if}
  </section>
{/if}
