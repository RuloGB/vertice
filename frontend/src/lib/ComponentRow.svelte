<script lang="ts">
  import type { Component } from "../bindings/Component";
  import type { ComponentKind } from "../bindings/ComponentKind";
  import { useI18n } from "./i18n/locale.svelte";
  import { isDuplicate } from "./inventory";
  import LocationList from "./LocationList.svelte";

  const i18n = useI18n();

  let { component, onSelect }: { component: Component; onSelect?: (component: Component) => void } =
    $props();

  const KIND_LABEL_KEY = {
    skill: "kind.skill",
    agent: "kind.agent",
    mcp: "kind.mcp",
  } as const satisfies Record<ComponentKind, `kind.${ComponentKind}`>;

  const duplicate = $derived(isDuplicate(component));
  const embedded = $derived(component.locations.some(({ origin }) => origin === "embedded"));
  const compact = $derived(onSelect !== undefined);
</script>

{#if compact}
  <button
    type="button"
    class="group flex w-full flex-col gap-2 rounded-panel border border-stroke bg-surface p-4 text-left shadow-panel transition-[border,transform,background] duration-150 hover:-translate-y-px hover:border-interactive-hover/55 hover:bg-surface-raised focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-interactive-hover"
    onclick={() => onSelect?.(component)}
  >
    <header class="flex flex-wrap items-center gap-2">
      <h3 class="font-semibold text-content">{component.name}</h3>
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
      <p class="truncate text-sm leading-6 text-content-muted">{component.description}</p>
    {/if}
  </button>
{:else}
  <article
    class="group flex flex-col gap-3 rounded-panel border border-stroke bg-surface p-5 shadow-panel transition-[border,transform,background] duration-150 hover:-translate-y-px hover:border-interactive-hover/55 hover:bg-surface-raised"
  >
    <header class="flex flex-wrap items-center gap-2">
      <h3 class="font-semibold text-content">{component.name}</h3>
      <span class="label-caps rounded-full bg-canvas/65 px-2.5 py-1">
        {i18n.t(KIND_LABEL_KEY[component.kind])}
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
{/if}
