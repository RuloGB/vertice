<script lang="ts">
 import { createI18n, provideI18n, type SupportedLocale } from "../i18n/locale.svelte";
 import SubscriptionsPage from "./SubscriptionsPage.svelte";
 let { locale = "en" }: { locale?: SupportedLocale } = $props();
 // The harness intentionally seeds i18n once per mount.
 // svelte-ignore state_referenced_locally
 provideI18n(createI18n(locale));
</script>
<SubscriptionsPage />
