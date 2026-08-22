<script lang="ts">
  import type { Location } from "../bindings/Location";
  import { useI18n } from "./i18n/locale.svelte";

  const i18n = useI18n();

  let { locations }: { locations: Location[] } = $props();
</script>

<!--
  Renders every location verbatim. Nullable paths get a neutral placeholder:
  no filesystem action is invented for embedded components.
-->
<ul class="flex flex-col gap-0.5">
  {#each locations as location, index (index)}
    <li class="font-mono text-xs text-mist-400">
      {#if location.path !== null}
        {location.path}
      {:else}
        <span class="italic text-mist-400">{i18n.t("location.noPath")}</span>
      {/if}
    </li>
  {/each}
</ul>
