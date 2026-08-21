<script lang="ts">
  import type { ComponentFilter } from "./filterComponents";
  import { useI18n } from "./i18n/locale.svelte";

  const i18n = useI18n();

  let {
    query,
    kind,
    reloading = false,
    onQueryChange,
    onKindChange,
    onReload,
  }: {
    query: string;
    kind: ComponentFilter["kind"];
    reloading?: boolean;
    onQueryChange: (query: string) => void;
    onKindChange: (kind: ComponentFilter["kind"]) => void;
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
    class="min-w-48 flex-1 rounded-md border border-slate-700 bg-slate-900 px-3 py-1.5 text-sm text-slate-100 placeholder:text-slate-500 focus:border-slate-500 focus:outline-none"
  />
  <select
    aria-label={i18n.t("toolbar.kindAriaLabel")}
    value={kind}
    onchange={(event) => onKindChange(event.currentTarget.value as ComponentFilter["kind"])}
    class="rounded-md border border-slate-700 bg-slate-900 px-3 py-1.5 text-sm text-slate-100 focus:border-slate-500 focus:outline-none"
  >
    <option value="all">{i18n.t("toolbar.allKinds")}</option>
    <option value="skill">{i18n.t("kind.skill")}</option>
    <option value="agent">{i18n.t("kind.agent")}</option>
  </select>
  <button
    type="button"
    disabled={reloading}
    onclick={onReload}
    class="rounded-md border border-slate-600 bg-slate-800 px-3 py-1.5 text-sm font-medium text-slate-100 hover:bg-slate-700 disabled:cursor-not-allowed disabled:opacity-50"
  >
    {reloading ? i18n.t("toolbar.reloading") : i18n.t("toolbar.reload")}
  </button>
</div>
