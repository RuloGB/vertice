<script lang="ts">
  // Test-only harness. `provideI18n` wraps `setContext`, which Svelte 5 only
  // permits during component initialisation, and the key `createContext`
  // mints is internal — so a test cannot seed the context through `mount`'s
  // `context` option. Mounting this instead mirrors what `App.svelte` does
  // in production: provide the context, then render the page.
  import type { ScanReport } from "../../bindings/ScanReport";
  import { createI18n, provideI18n, type SupportedLocale } from "../i18n/locale.svelte";
  import ClientsPage from "./ClientsPage.svelte";

  let {
    locale = "en",
    report,
    status,
    failureMessage,
  }: {
    locale?: SupportedLocale;
    report: ScanReport | null;
    status: "idle" | "loading" | "ready" | "failed";
    failureMessage: string | null;
  } = $props();

  // Deliberate one-time read: the context is created once at init and the
  // harness never re-provides it.
  // svelte-ignore state_referenced_locally
  provideI18n(createI18n(locale));
</script>

<ClientsPage {report} {status} {failureMessage} />
