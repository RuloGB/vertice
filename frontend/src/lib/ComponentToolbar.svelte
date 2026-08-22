<script lang="ts">
  import { useI18n } from "./i18n/locale.svelte";

  const i18n = useI18n();

  let {
    query,
    reloading = false,
    onQueryChange,
    onReload,
  }: {
    query: string;
    reloading?: boolean;
    onQueryChange: (query: string) => void;
    onReload: () => void;
  } = $props();
</script>

<!-- View-only controls: emit intent callbacks, never invoke IPC themselves. -->
<div class="flex flex-wrap items-center gap-3">
  <input
    type="search"
    placeholder={i18n.t("toolbar.searchPlaceholder")}
    aria-label={i18n.t("toolbar.searchAriaLabel")}
    value={query}
    oninput={(event) => onQueryChange(event.currentTarget.value)}
    class="min-w-48 flex-1 rounded-lg border border-line bg-ink-850 px-3 py-1.5 text-sm text-mist-100 placeholder:text-mist-400 focus:border-accent-500 focus:outline-none"
  />
  <button
    type="button"
    disabled={reloading}
    onclick={onReload}
    class="rounded-lg bg-accent-500 px-4 py-1.5 text-sm font-medium text-ink-950 transition-colors hover:bg-accent-400 disabled:cursor-not-allowed disabled:opacity-50"
  >
    {reloading ? i18n.t("toolbar.reloading") : i18n.t("toolbar.reload")}
  </button>
</div>
