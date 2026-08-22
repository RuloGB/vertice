<script lang="ts">
  import { APP_VERSION, PRODUCT_NAME } from "./appTitle";
  import BrandMark from "./BrandMark.svelte";
  import { useI18n, type SupportedLocale } from "./i18n/locale.svelte";
  import NavIcon from "./NavIcon.svelte";
  import { NAV_GROUPS, navGroupLabelKey, navLabelKey, type RouteId } from "./navigation";

  const i18n = useI18n();

  let {
    current,
    onNavigate,
  }: {
    current: RouteId;
    onNavigate: (route: RouteId) => void;
  } = $props();
</script>

<aside class="shadow-sidebar flex w-72 shrink-0 flex-col gap-7 border-r border-stroke bg-surface px-3 py-5">
  <div class="flex items-center gap-3 px-2">
    <BrandMark compact variant="sidebar" class="shrink-0" />
    <span class="flex min-w-0 flex-col">
      <span class="truncate text-sm font-semibold text-content">{PRODUCT_NAME}</span>
      <span class="text-xs leading-snug text-content-subtle">{i18n.t("app.tagline")}</span>
    </span>
  </div>

  <nav class="flex flex-1 flex-col gap-6 overflow-y-auto pr-1" aria-label={i18n.t("navGroup.overview")}>
    {#each NAV_GROUPS as group (group.id)}
      <div class="flex flex-col gap-1.5">
        <h2 class="label-caps px-3 pb-1">{i18n.t(navGroupLabelKey(group.id))}</h2>
        {#each group.routes as route (route)}
          {@const active = route === current}
          <button
            type="button"
            aria-current={active ? "page" : undefined}
            onclick={() => onNavigate(route)}
            class={[
              "group flex items-center gap-3 rounded-control border px-3 py-2.5 text-sm transition-[background,color,border,transform] duration-150",
              active
                ? "border-interactive-hover/45 bg-interactive/20 font-semibold text-content nav-active-indicator"
                : "border-transparent text-content-muted hover:border-stroke hover:bg-surface-raised hover:text-content",
            ]}
          >
            <NavIcon {route} class={active ? "size-4 text-action" : "size-4 text-content-subtle group-hover:text-content-muted"} />
            <span class="truncate">{i18n.t(navLabelKey(route))}</span>
          </button>
        {/each}
      </div>
    {/each}
  </nav>

  <div class="flex flex-col gap-2 border-t border-stroke px-2 pt-4">
    <label class="flex flex-col gap-1.5 text-xs text-content-subtle">
      <span>{i18n.t("app.languageLabel")}</span>
      <select
        aria-label={i18n.t("app.languageLabel")}
        value={i18n.locale}
        onchange={(event) => i18n.setLocale(event.currentTarget.value as SupportedLocale)}
        class="rounded-control border border-stroke bg-surface-raised px-2.5 py-2 text-sm text-content transition-colors hover:border-stroke-strong focus:border-interactive-hover focus:outline-none"
      >
        <option value="en">{i18n.t("app.languageEnglish")}</option>
        <option value="es">{i18n.t("app.languageSpanish")}</option>
      </select>
    </label>
    <p class="text-[0.68rem] text-content-subtle">v{APP_VERSION}</p>
  </div>
</aside>
