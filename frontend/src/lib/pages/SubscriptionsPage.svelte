<script lang="ts">
  import { useI18n } from "../i18n/locale.svelte";
  import SubscriptionCard from "../SubscriptionCard.svelte";
  import {
    formatAmount,
    monthlyTotalsByCurrency,
    sortByRenewal,
    type Subscription,
  } from "../subscriptions";

  const i18n = useI18n();

  let {
    subscriptions,
    today,
  }: {
    subscriptions: readonly Subscription[];
    today: Date;
  } = $props();

  const intlLocale = $derived(i18n.locale === "es" ? "es-ES" : "en-US");
  const ordered = $derived(sortByRenewal(subscriptions, today));
  const monthlySpend = $derived(
    [...monthlyTotalsByCurrency(subscriptions)]
      .map(([currency, total]) => formatAmount(total, currency, intlLocale))
      .join(" + "),
  );
</script>

<section class="flex flex-col gap-6">
  <header class="flex flex-wrap items-end justify-between gap-4 border-b border-stroke pb-5">
    <div class="flex flex-col gap-2">
      <div class="flex items-center gap-3">
        <h1 class="text-2xl font-semibold tracking-tight text-content">
          {i18n.t("area.subscriptions")}
        </h1>
        <span
          class="rounded-full border border-interactive-hover/35 bg-interactive/10 px-2.5 py-1 text-xs font-bold uppercase tracking-wide text-interactive-hover"
        >
          {i18n.t("subscriptions.sampleBadge")}
        </span>
      </div>
      <p class="text-sm text-content-muted">{i18n.t("subscriptions.intro")}</p>
    </div>
  </header>

  {#if ordered.length === 0}
    <div
      role="status"
      class="rounded-panel border border-dashed border-stroke-strong bg-surface/60 p-12 text-center text-sm text-content-subtle"
    >
      {i18n.t("subscriptions.empty")}
    </div>
  {:else}
    <div class="grid gap-4 md:grid-cols-2">
      <div class="surface-card relative overflow-hidden px-5 py-4">
        <span class="label-caps">{i18n.t("subscriptions.summaryActive")}</span>
        <span class="mt-2 block text-3xl font-semibold tabular-nums text-content">
          {ordered.length}
        </span>
        <span class="absolute bottom-0 left-0 h-1 w-16 bg-action"></span>
      </div>
      <div class="surface-card relative overflow-hidden px-5 py-4">
        <span class="label-caps">{i18n.t("subscriptions.summaryMonthly")}</span>
        <span class="mt-2 block text-3xl font-semibold tabular-nums text-content">
          {monthlySpend}
        </span>
        <span class="absolute bottom-0 left-0 h-1 w-16 bg-interactive-hover"></span>
      </div>
    </div>

    <ul class="grid gap-4 lg:grid-cols-2 2xl:grid-cols-3">
      {#each ordered as subscription (subscription.id)}
        <li>
          <SubscriptionCard {subscription} {today} />
        </li>
      {/each}
    </ul>
  {/if}
</section>
