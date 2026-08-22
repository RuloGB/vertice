<script lang="ts">
  import type { Component } from "../bindings/Component";
  import { useI18n } from "./i18n/locale.svelte";
  import { isDuplicate } from "./inventory";
  import LocationList from "./LocationList.svelte";

  const i18n = useI18n();

  let { component }: { component: Component } = $props();

  const duplicate = $derived(isDuplicate(component));
  const embedded = $derived(component.locations.some(({ origin }) => origin === "embedded"));
</script>

<article class="flex flex-col gap-2 rounded-xl border border-line bg-ink-800 p-4">
  <header class="flex flex-wrap items-center gap-2">
    <h3 class="font-medium text-mist-100">{component.name}</h3>
    <span class="rounded bg-ink-700 px-1.5 py-0.5 text-xs uppercase tracking-wide text-mist-300"
      >{component.kind === "skill" ? i18n.t("kind.skill") : i18n.t("kind.agent")}</span
    >
    {#if embedded}
      <span
        data-testid="embedded-status"
        class="rounded bg-accent-500/15 px-1.5 py-0.5 text-xs font-medium text-accent-300"
      >
        {i18n.t("inventory.embedded")}
      </span>
    {/if}
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
    <p class="text-sm text-mist-400">{component.description}</p>
  {/if}
  <LocationList locations={component.locations} />
</article>
