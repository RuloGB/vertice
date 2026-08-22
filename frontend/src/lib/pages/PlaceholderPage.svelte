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
  class="flex flex-col items-center justify-center gap-4 rounded-2xl border border-dashed border-line-strong bg-ink-900/40 px-8 py-20 text-center"
>
  <span class="grid size-12 place-items-center rounded-2xl bg-ink-700 text-accent-300">
    <NavIcon {route} class="size-6" />
  </span>
  <span
    class="rounded-full border border-line px-3 py-1 text-xs uppercase tracking-wide text-mist-400"
  >
    {i18n.t("placeholder.badge")}
  </span>
  <h1 class="text-lg font-medium text-mist-100">
    {i18n.t(navLabelKey(route))} — {i18n.t("placeholder.title")}
  </h1>
  <p class="max-w-md text-sm leading-relaxed text-mist-400">{i18n.t("placeholder.body")}</p>
</section>
