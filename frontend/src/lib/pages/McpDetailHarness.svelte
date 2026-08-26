<script lang="ts">
  // Test-only harness — see ClientsPageHarness.svelte for the pattern.
  import type { Component } from "../../bindings/Component";
  import { createI18n, provideI18n, type SupportedLocale } from "../i18n/locale.svelte";
  import McpDetail from "./McpDetail.svelte";

  let {
    locale = "en",
    component,
  }: {
    locale?: SupportedLocale;
    component: Component;
  } = $props();

  // svelte-ignore state_referenced_locally
  provideI18n(createI18n(locale));
</script>

<McpDetail {component} onBack={() => {}} />
