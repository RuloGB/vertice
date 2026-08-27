<script lang="ts">
  import { createI18n, provideI18n, type SupportedLocale } from "../i18n/locale.svelte";
  import PromptsPage from "./PromptsPage.svelte";

  let { locale = "en" }: { locale?: SupportedLocale } = $props();

  // svelte-ignore state_referenced_locally
  provideI18n(createI18n(locale));
</script>

<PromptsPage />
