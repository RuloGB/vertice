<script lang="ts">
  import { useI18n } from "./i18n/locale.svelte";

  const i18n = useI18n();

  let {
    query,
    onQueryChange,
  }: {
    query: string;
    onQueryChange: (query: string) => void;
  } = $props();
</script>

<div class="surface-card flex flex-wrap items-center gap-3 p-3">
  <input
    type="search"
    placeholder={i18n.t("toolbar.searchPlaceholder")}
    aria-label={i18n.t("toolbar.searchAriaLabel")}
    value={query}
    oninput={(event) => onQueryChange(event.currentTarget.value)}
    class="min-w-56 flex-1 rounded-control border border-stroke bg-canvas/45 px-3.5 py-2.5 text-sm text-content placeholder:text-content-subtle transition-colors hover:border-stroke-strong focus:border-interactive-hover focus:outline-none"
  />
</div>
