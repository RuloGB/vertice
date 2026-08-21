<script lang="ts">
  import type { Location } from "../bindings/Location";

  let { locations }: { locations: Location[] } = $props();
</script>

<!--
  Renders every location verbatim. Nullable paths get a neutral placeholder:
  no filesystem action is invented for embedded components.
-->
<ul class="flex flex-col gap-0.5">
  {#each locations as location, index (index)}
    <li class="font-mono text-xs text-slate-400">
      {#if location.path !== null}
        {location.path}
      {:else}
        <span class="italic text-slate-500">(no path on disk)</span>
      {/if}
    </li>
  {/each}
</ul>
