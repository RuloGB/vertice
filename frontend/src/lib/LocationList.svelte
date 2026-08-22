<script lang="ts">
  import type { Location } from "../bindings/Location";
  import { useI18n } from "./i18n/locale.svelte";

  const i18n = useI18n();

  let { locations }: { locations: Location[] } = $props();
</script>

<ul class="flex flex-col gap-1 border-t border-stroke pt-3">
  {#each locations as location, index (index)}
    <li class="min-w-0 rounded-control bg-canvas/35 px-2.5 py-1.5 font-mono text-xs text-content-subtle break-all">
      {#if location.path !== null}
        {location.path}
      {:else}
        <span class="italic">{i18n.t("location.noPath")}</span>
      {/if}
    </li>
  {/each}
</ul>
