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

  // `today` is injected so the page renders deterministically under test.
  let { subscriptions, today }: { subscriptions: readonly Subscription[]; today: Date } = $props();

  const intlLocale = $derived(i18n.locale === "es" ? "es-ES" : "en-US");
  const ordered = $derived(sortByRenewal(subscriptions, today));
  const monthlySpend = $derived(
    [...monthlyTotalsByCurrency(subscriptions)]
      .map(([currency, total]) => formatAmount(total, currency, intlLocale))
      .join(" + "),
  );
</script>

<section class="flex flex-col gap-6">
  <header class="flex flex-col gap-2">
    <div class="flex flex-wrap items-center gap-3">
      <h1 class="text-xl font-semibold tracking-tight text-mist-100">
        {i18n.t("area.subscriptions")}
      </h1>
      <span
        class="rounded-full border border-line px-2.5 py-0.5 text-xs uppercase tracking-wide text-mist-400"
      >
        {i18n.t("subscriptions.sampleBadge")}
      </span>
    </div>
    <p class="text-sm text-mist-400">{i18n.t("subscriptions.intro")}</p>
  </header>

  {#if ordered.length === 0}
    <div
      role="status"
      class="rounded-xl border border-dashed border-line-strong p-12 text-center text-sm text-mist-400"
    >
      {i18n.t("subscriptions.empty")}
    </div>
  {:else}
    <div class="grid gap-3 sm:grid-cols-2">
      <div class="flex flex-col gap-1 rounded-xl border border-line bg-ink-800 px-5 py-4">
        <span class="text-xs uppercase tracking-wide text-mist-400">
          {i18n.t("subscriptions.summaryActive")}
        </span>
        <span class="text-2xl font-semibold tabular-nums text-mist-100">{ordered.length}</span>
      </div>
      <div class="flex flex-col gap-1 rounded-xl border border-line bg-ink-800 px-5 py-4">
        <span class="text-xs uppercase tracking-wide text-mist-400">
          {i18n.t("subscriptions.summaryMonthly")}
        </span>
        <span class="text-2xl font-semibold tabular-nums text-mist-100">{monthlySpend}</span>
      </div>
    </div>

    <ul class="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
      {#each ordered as subscription (subscription.id)}
        <li>
          <SubscriptionCard {subscription} {today} />
        </li>
      {/each}
    </ul>
  {/if}
</section>
