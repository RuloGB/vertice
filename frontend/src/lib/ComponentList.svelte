<script lang="ts">
  import type { Component } from "../bindings/Component";
  import { useI18n } from "./i18n/locale.svelte";
  import ComponentRow from "./ComponentRow.svelte";

  const i18n = useI18n();

  let { components }: { components: Component[] } = $props();
</script>

{#if components.length === 0}
  <div
    role="status"
    class="rounded-panel border border-dashed border-stroke-strong bg-surface/60 p-12 text-center text-sm text-content-subtle"
  >
    {i18n.t("components.empty")}
  </div>
{:else}
  <ul class="flex flex-col gap-4">
    {#each components as component (component.id)}
      <li>
        <ComponentRow {component} />
      </li>
    {/each}
  </ul>
{/if}
