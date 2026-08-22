<script lang="ts">
  import { APP_VERSION, PRODUCT_NAME } from "./appTitle";
  import { useI18n, type SupportedLocale } from "./i18n/locale.svelte";
  import NavIcon from "./NavIcon.svelte";
  import { NAV_GROUPS, navGroupLabelKey, navLabelKey, type RouteId } from "./navigation";

  const i18n = useI18n();

  // View-only: emits navigation intent, never touches IPC or global history.
  let {
    current,
    onNavigate,
  }: {
    current: RouteId;
    onNavigate: (route: RouteId) => void;
  } = $props();
</script>

<aside
  class="flex w-72 shrink-0 flex-col gap-6 border-r border-line bg-ink-900/80 px-3 py-5 backdrop-blur"
>
  <div class="flex items-center gap-3 px-2">
    <span
      class="grid size-9 shrink-0 place-items-center rounded-xl bg-gradient-to-br from-accent-400 to-accent-600 text-sm font-bold text-ink-950 shadow-lg shadow-accent-600/25"
    >
      V
    </span>
    <span class="flex min-w-0 flex-col">
      <span class="truncate text-sm font-semibold text-mist-100">{PRODUCT_NAME}</span>
      <span class="text-xs leading-snug text-mist-400">{i18n.t("app.tagline")}</span>
    </span>
  </div>

  <nav class="flex flex-1 flex-col gap-5 overflow-y-auto" aria-label={i18n.t("navGroup.overview")}>
    {#each NAV_GROUPS as group (group.id)}
      <div class="flex flex-col gap-1">
        <h2 class="px-3 pb-1 text-[0.68rem] font-semibold uppercase tracking-widest text-mist-400">
          {i18n.t(navGroupLabelKey(group.id))}
        </h2>
        {#each group.routes as route (route)}
          {@const active = route === current}
          <button
            type="button"
            aria-current={active ? "page" : undefined}
            onclick={() => onNavigate(route)}
            class={[
              "group flex items-center gap-3 rounded-lg px-3 py-2 text-sm transition-colors",
              active
                ? "bg-accent-500/12 font-medium text-mist-100 ring-1 ring-inset ring-accent-500/30"
                : "text-mist-300 hover:bg-ink-800 hover:text-mist-100",
            ]}
          >
            <NavIcon
              {route}
              class={active ? "size-4 text-accent-300" : "size-4 text-mist-400 group-hover:text-mist-200"}
            />
            <span class="truncate">{i18n.t(navLabelKey(route))}</span>
          </button>
        {/each}
      </div>
    {/each}
  </nav>

  <div class="flex flex-col gap-2 border-t border-line px-2 pt-4">
    <label class="flex flex-col gap-1.5 text-xs text-mist-400">
      <span>{i18n.t("app.languageLabel")}</span>
      <select
        aria-label={i18n.t("app.languageLabel")}
        value={i18n.locale}
        onchange={(event) => i18n.setLocale(event.currentTarget.value as SupportedLocale)}
        class="rounded-lg border border-line bg-ink-850 px-2.5 py-1.5 text-sm text-mist-100 transition-colors hover:border-line-strong focus:border-accent-500 focus:outline-none"
      >
        <option value="en">{i18n.t("app.languageEnglish")}</option>
        <option value="es">{i18n.t("app.languageSpanish")}</option>
      </select>
    </label>
    <p class="text-[0.68rem] text-mist-400">v{APP_VERSION}</p>
  </div>
</aside>
