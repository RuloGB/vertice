<script lang="ts">
  import { useI18n } from "../i18n/locale.svelte";
  import NavIcon from "../NavIcon.svelte";
  import { navLabelKey, type RouteId } from "../navigation";

  const i18n = useI18n();

  // Sections with no backend source yet: an explicit empty state, never fake rows.
  let { route }: { route: RouteId } = $props();
</script>

<section
  data-testid="placeholder-page"
  class="flex flex-col items-center justify-center gap-4 rounded-panel border border-dashed border-stroke-strong bg-surface/60 shadow-panel px-8 py-20 text-center"
>
  <span class="grid size-12 place-items-center rounded-2xl bg-interactive/20 text-action">
    <NavIcon {route} class="size-6" />
  </span>
  <span
    class="rounded-full border border-stroke bg-canvas/35 px-3 py-1 text-xs uppercase tracking-wide text-content-subtle"
  >
    {i18n.t("placeholder.badge")}
  </span>
  <h1 class="text-xl font-semibold text-content">
    {i18n.t(navLabelKey(route))} — {i18n.t("placeholder.title")}
  </h1>
  <p class="max-w-md text-sm leading-relaxed text-content-muted">{i18n.t("placeholder.body")}</p>
</section>
