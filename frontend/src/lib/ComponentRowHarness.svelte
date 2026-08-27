<script lang="ts">
  import type { Component } from "../bindings/Component";
  import { createI18n, provideI18n, type SupportedLocale } from "./i18n/locale.svelte";
  import ComponentRow from "./ComponentRow.svelte";

  let {
    locale = "en",
    component,
    compact = false,
  }: {
    locale?: SupportedLocale;
    component: Component;
    compact?: boolean;
  } = $props();

  // svelte-ignore state_referenced_locally
  provideI18n(createI18n(locale));
</script>

{#if compact}
  <ComponentRow {component} onSelect={() => {}} />
{:else}
  <ComponentRow {component} />
{/if}