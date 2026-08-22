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

  let { subscription, today }: { subscription: Subscription; today: Date } = $props();

  const intlLocale = $derived(i18n.locale === "es" ? "es-ES" : "en-US");
  const renewal = $derived(nextRenewal(subscription, today));
  const remainingDays = $derived(daysUntil(renewal, today));
  const imminent = $derived(remainingDays <= 7);
  const cycleLabel = $derived(
    subscription.cycle === "yearly"
      ? i18n.t("subscriptions.cycleYearly")
      : i18n.t("subscriptions.cycleMonthly"),
  );
  const cycleSuffix = $derived(
    subscription.cycle === "yearly"
      ? i18n.t("subscriptions.perYear")
      : i18n.t("subscriptions.perMonth"),
  );
  const countdown = $derived.by(() => {
    if (remainingDays === 0) return i18n.t("subscriptions.renewsToday");
    if (remainingDays === 1) return i18n.t("subscriptions.renewsTomorrow");
    return i18n.t("subscriptions.renewsInDays", { days: remainingDays });
  });
</script>

<article
  data-testid="subscription-card"
  class="group flex h-full flex-col gap-5 rounded-panel border border-stroke bg-surface p-5 shadow-panel transition-[border,background,transform] duration-150 hover:-translate-y-px hover:border-interactive-hover/55 hover:bg-surface-raised"
>
  <header class="flex items-start justify-between gap-3">
    <div class="flex min-w-0 flex-col gap-1">
      <h2 class="truncate text-base font-semibold text-content">{subscription.provider}</h2>
      <span class="text-xs text-content-subtle">
        {i18n.t("subscriptions.planLabel")}: {subscription.plan}
      </span>
    </div>
    <span class="shrink-0 rounded-full bg-interactive/18 px-2.5 py-1 text-xs font-bold text-interactive-hover">
      {cycleLabel}
    </span>
  </header>

  <div class="flex items-baseline gap-1">
    <span class="text-3xl font-semibold tabular-nums tracking-tight text-content">
      {formatAmount(subscription.amount, subscription.currency, intlLocale)}
    </span>
    <span class="text-sm text-content-subtle">{cycleSuffix}</span>
  </div>

  <footer class="flex flex-col gap-2 border-t border-stroke pt-4">
    <span class="label-caps">{i18n.t("subscriptions.renewalLabel")}</span>
    <div class="flex flex-wrap items-center gap-2">
      <span class="text-sm text-content-muted">{formatRenewalDate(renewal, intlLocale)}</span>
      <span class={["rounded-full px-2.5 py-1 text-xs font-bold", imminent ? "bg-warning/15 text-warning" : "bg-canvas/55 text-content-muted"]}>
        {countdown}
      </span>
    </div>
  </footer>
</article>
