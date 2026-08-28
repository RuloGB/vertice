<script lang="ts">
  import type { Component } from "../bindings/Component";
  import { useI18n } from "./i18n/locale.svelte";

  const i18n = useI18n();

  let { component }: { component: Component } = $props();

</script>

<header class="rounded-3xl border border-white/10 bg-white/5 p-6 shadow-2xl shadow-black/20">
  <p class="text-sm font-semibold uppercase tracking-[0.3em] text-cyan-200">{i18n.t("navGroup.components")}</p>
  <h1 class="mt-2 text-3xl font-semibold text-white">{component.name}</h1>
  {#if component.description}
    <p class="mt-2 max-w-2xl text-sm leading-6 text-mist-300">{component.description}</p>
  {/if}
</header>
