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

<article
  class="group flex flex-col gap-3 rounded-panel border border-stroke bg-surface p-5 shadow-panel transition-[border,transform,background] duration-150 hover:-translate-y-px hover:border-interactive-hover/55 hover:bg-surface-raised"
>
  <header class="flex flex-wrap items-center gap-2">
    <h3 class="font-semibold text-content">{component.name}</h3>
    <span class="label-caps rounded-full bg-canvas/65 px-2.5 py-1">
      {component.kind === "skill" ? i18n.t("kind.skill") : i18n.t("kind.agent")}
    </span>
    {#if embedded}
      <span
        data-testid="embedded-status"
        class="rounded-full bg-interactive/18 px-2.5 py-1 text-xs font-semibold text-interactive-hover"
      >
        {i18n.t("components.embedded")}
      </span>
    {/if}
    {#if duplicate}
      <span
        class="rounded-full bg-warning/15 px-2.5 py-1 text-xs font-semibold text-warning"
        title={i18n.t("components.duplicateTitle", { count: component.locations.length })}
      >
        {i18n.t("components.duplicate")}
      </span>
    {/if}
  </header>

  {#if component.description}
    <p class="text-sm leading-6 text-content-muted">{component.description}</p>
  {/if}

  <LocationList locations={component.locations} />
</article>
