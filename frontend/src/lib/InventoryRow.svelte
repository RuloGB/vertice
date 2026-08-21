<script lang="ts">
  import type { Component } from "../bindings/Component";
  import { useI18n } from "./i18n/locale.svelte";
  import { isDuplicate } from "./inventory";
  import LocationList from "./LocationList.svelte";

  const i18n = useI18n();

  let { component }: { component: Component } = $props();

  const duplicate = $derived(isDuplicate(component));
</script>

<article class="flex flex-col gap-2 rounded-lg border border-slate-800 bg-slate-900/60 p-4">
  <header class="flex flex-wrap items-center gap-2">
    <h3 class="font-medium text-slate-100">{component.name}</h3>
    <span class="rounded bg-slate-800 px-1.5 py-0.5 text-xs uppercase tracking-wide text-slate-300"
      >{component.kind === "skill" ? i18n.t("kind.skill") : i18n.t("kind.agent")}</span
    >
    {#if duplicate}
      <span
        class="rounded bg-amber-500/15 px-1.5 py-0.5 text-xs font-medium text-amber-300"
        title={i18n.t("inventory.duplicateTitle", { count: component.locations.length })}
      >
        {i18n.t("inventory.duplicate")}
      </span>
    {/if}
  </header>
  {#if component.description}
    <p class="text-sm text-slate-400">{component.description}</p>
  {/if}
  <LocationList locations={component.locations} />
</article>
