<script lang="ts">
  import { useI18n } from "./i18n/locale.svelte";
  import {
    daysUntil,
    formatAmount,
    formatRenewalDate,
    nextRenewal,
    type Subscription,
  } from "./subscriptions";

  const i18n = useI18n();

  // `today` is passed in by the page so the card stays a pure projection.
  let { subscription, today }: { subscription: Subscription; today: Date } = $props();

  const intlLocale = $derived(i18n.locale === "es" ? "es-ES" : "en-US");
  const renewal = $derived(nextRenewal(subscription, today));
  const remainingDays = $derived(daysUntil(renewal, today));
  const imminent = $derived(remainingDays <= 7);

  const cycleLabel = $derived(
    subscription.cycle === "yearly" ? i18n.t("subscriptions.cycleYearly") : i18n.t("subscriptions.cycleMonthly"),
  );
  const cycleSuffix = $derived(
    subscription.cycle === "yearly" ? i18n.t("subscriptions.perYear") : i18n.t("subscriptions.perMonth"),
  );
  const countdown = $derived.by(() => {
    if (remainingDays === 0) {
      return i18n.t("subscriptions.renewsToday");
    }
    if (remainingDays === 1) {
      return i18n.t("subscriptions.renewsTomorrow");
    }
    return i18n.t("subscriptions.renewsInDays", { days: remainingDays });
  });
</script>

<article
  data-testid="subscription-card"
  class="flex h-full flex-col gap-4 rounded-xl border border-line bg-ink-800 p-5 transition-colors hover:border-line-strong"
>
  <header class="flex items-start justify-between gap-3">
    <div class="flex min-w-0 flex-col gap-1">
      <h2 class="truncate text-base font-medium text-mist-100">{subscription.provider}</h2>
      <span class="text-xs text-mist-400">{i18n.t("subscriptions.planLabel")}: {subscription.plan}</span>
    </div>
    <span
      class="shrink-0 rounded-full bg-accent-500/15 px-2.5 py-1 text-xs font-medium text-accent-300"
    >
      {cycleLabel}
    </span>
  </header>

  <div class="flex items-baseline gap-1">
    <span class="text-2xl font-semibold tabular-nums text-mist-100">
      {formatAmount(subscription.amount, subscription.currency, intlLocale)}
    </span>
    <span class="text-sm text-mist-400">{cycleSuffix}</span>
  </div>

  <footer class="flex flex-col gap-1 border-t border-line pt-3">
    <span class="text-xs uppercase tracking-wide text-mist-400">
      {i18n.t("subscriptions.renewalLabel")}
    </span>
    <div class="flex flex-wrap items-center gap-2">
      <span class="text-sm text-mist-200">{formatRenewalDate(renewal, intlLocale)}</span>
      <span
        class={[
          "rounded-full px-2 py-0.5 text-xs font-medium",
          imminent ? "bg-amber-500/15 text-amber-300" : "bg-ink-700 text-mist-300",
        ]}
      >
        {countdown}
      </span>
    </div>
  </footer>
</article>
