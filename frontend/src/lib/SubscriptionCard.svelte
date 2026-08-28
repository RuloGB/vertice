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
  let { subscription, today, onEdit, onDelete }: {
    subscription: Subscription;
    today: Date;
    onEdit?: (subscription: Subscription) => void;
    onDelete?: (subscription: Subscription) => void;
  } = $props();

  const locale = $derived(i18n.locale === "es" ? "es-ES" : "en-US");
  const renewal = $derived(nextRenewal(subscription, today));
  const daysRemaining = $derived(daysUntil(renewal, today));
  const imminent = $derived(daysRemaining <= 7);
  const billingCycle = $derived(
    subscription.cycle === "yearly"
      ? i18n.t("subscriptions.cycleYearly")
      : i18n.t("subscriptions.cycleMonthly"),
  );
  const renewalCountdown = $derived(
    daysRemaining === 0
      ? i18n.t("subscriptions.renewsToday")
      : daysRemaining === 1
        ? i18n.t("subscriptions.renewsTomorrow")
        : i18n.t("subscriptions.renewsInDays", { days: daysRemaining }),
  );
</script>

<article data-testid="subscription-card" class="group flex h-full flex-col gap-5 rounded-panel border border-stroke bg-surface p-5 shadow-panel transition-[border,background,transform] duration-150 hover:-translate-y-px hover:border-interactive-hover/55 hover:bg-surface-raised">
  <header class="flex items-start justify-between gap-3">
    <div class="min-w-0">
      <h2 class="truncate text-base font-semibold text-content">{subscription.provider}</h2>
      <p class="mt-1 truncate text-xs text-content-subtle">{i18n.t("subscriptions.planLabel")}: {subscription.plan}</p>
      <span class="sr-only">{i18n.t("subscriptions.cycleLabel")}: {billingCycle}. {renewalCountdown}</span>
    </div>
    <span class="shrink-0 rounded-full bg-interactive/18 px-2.5 py-1 text-xs font-bold text-interactive-hover">{billingCycle}</span>
  </header>

  <div class="flex items-baseline gap-1">
    <span class="text-3xl font-semibold tabular-nums tracking-tight text-content">{formatAmount(subscription.amount, subscription.currency, locale)}</span>
    <span class="text-sm text-content-subtle">{subscription.cycle === "yearly" ? i18n.t("subscriptions.perYear") : i18n.t("subscriptions.perMonth")}</span>
  </div>

  <footer class="mt-auto border-t border-stroke pt-4">
    <span class="label-caps">{i18n.t("subscriptions.renewalLabel")}</span>
    <div class="mt-2 flex flex-wrap items-center gap-2">
      <span class="text-sm text-content-muted">{formatRenewalDate(renewal, locale)}</span>
      <span class={["rounded-full px-2.5 py-1 text-xs font-bold", imminent ? "bg-warning/15 text-warning" : "bg-canvas/55 text-content-muted"]}>{renewalCountdown}</span>
    </div>
    <div class="mt-5 flex gap-2 border-t border-stroke pt-4">
      <button type="button" class="flex-1 rounded-control border border-stroke px-3 py-2 text-sm font-bold text-content-muted transition-colors hover:border-interactive-hover/55 hover:bg-canvas/45 hover:text-content focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-action" aria-label={`${i18n.t("subscriptions.editAction")} ${subscription.provider}`} onclick={() => onEdit?.(subscription)}>{i18n.t("subscriptions.editAction")}</button>
      <button type="button" class="rounded-control border border-danger/35 px-3 py-2 text-sm font-bold text-danger transition-colors hover:bg-danger/10 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-danger" aria-label={`${i18n.t("subscriptions.deleteAction")} ${subscription.provider}`} onclick={() => onDelete?.(subscription)}>{i18n.t("subscriptions.deleteAction")}</button>
    </div>
  </footer>
</article>
