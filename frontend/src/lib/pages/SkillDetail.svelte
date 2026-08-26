<script lang="ts">
  import type { Component } from "../../bindings/Component";
  import { useI18n } from "../i18n/locale.svelte";
  import { isDuplicate } from "../inventory";
  import LocationList from "../LocationList.svelte";
  import { CLIENT_LABEL, groupLocationsByClient } from "../clientGroups";

  const i18n = useI18n();

  let { component, onBack }: { component: Component; onBack: () => void } = $props();

  const duplicate = $derived(isDuplicate(component));
  const embedded = $derived(component.locations.some(({ origin }) => origin === "embedded"));
  const clientGroups = $derived(groupLocationsByClient(component.locations));
</script>

<section class="flex flex-col gap-6">
  <button
    type="button"
    onclick={onBack}
    class="flex w-fit items-center gap-2 rounded-control border border-stroke bg-surface px-4 py-2 text-sm font-medium text-content transition-colors hover:bg-surface-raised hover:border-interactive-hover/55 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-interactive-hover"
  >
    <span aria-hidden="true">&#8592;</span>
    {i18n.t("skillDetail.back")}
  </button>

  <article class="flex flex-col gap-6 rounded-panel border border-stroke bg-surface p-6 shadow-panel">
    <header class="flex flex-wrap items-center gap-3">
      <h1 class="text-2xl font-semibold tracking-tight text-content">{component.name}</h1>
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

    <div class="flex flex-col gap-2">
      <h2 class="text-sm font-semibold uppercase tracking-wider text-content-subtle">
        {i18n.t("skillDetail.description")}
      </h2>
      {#if component.description}
        <p class="text-sm leading-6 text-content-muted">{component.description}</p>
      {:else}
        <p class="text-sm italic text-content-subtle">{i18n.t("skillDetail.noDescription")}</p>
      {/if}
    </div>

    <div class="flex flex-col gap-2">
      <h2 class="text-sm font-semibold uppercase tracking-wider text-content-subtle">
        {i18n.t("skillDetail.locations")}
      </h2>
      <LocationList locations={component.locations} />
    </div>

    <div class="flex flex-col gap-2">
      <h2 class="text-sm font-semibold uppercase tracking-wider text-content-subtle">
        {i18n.t("skillDetail.aiClients")}
      </h2>
      {#if component.locations.length === 0}
        <div
          class="rounded-control border border-dashed border-stroke-strong bg-canvas/35 p-4 text-center text-sm text-content-subtle"
        >
          {i18n.t("skillDetail.aiClientsEmpty")}
        </div>
      {:else}
        <ul class="flex flex-col gap-2">
          {#each clientGroups as group (group.client)}
            <li
              class="flex items-center justify-between rounded-control bg-canvas/35 px-3 py-2.5"
            >
              <span class="text-sm text-content">
                {group.client !== null ? CLIENT_LABEL[group.client] : i18n.t("aiClients.shared")}
              </span>
              <span class="text-xs text-content-subtle">{group.count}</span>
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  </article>
</section>
