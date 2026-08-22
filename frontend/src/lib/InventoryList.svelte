<script lang="ts">
  import type { Component } from "../bindings/Component";
  import { useI18n } from "./i18n/locale.svelte";
  import InventoryRow from "./InventoryRow.svelte";

  const i18n = useI18n();

  let { components }: { components: Component[] } = $props();
</script>

{#if components.length === 0}
  <div
    role="status"
    class="rounded-xl border border-dashed border-line-strong p-12 text-center text-sm text-mist-400"
  >
    {i18n.t("inventory.empty")}
  </div>
{:else}
  <ul class="flex flex-col gap-3">
    {#each components as component (component.id)}
      <li>
        <InventoryRow {component} />
      </li>
    {/each}
  </ul>
{/if}
