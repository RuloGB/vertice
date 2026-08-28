import { invoke } from "@tauri-apps/api/core";
import type { Subscription } from "../bindings/Subscription";
import type { SubscriptionDraft } from "../bindings/SubscriptionDraft";
import type { SubscriptionError } from "../bindings/SubscriptionError";
import type { SubscriptionUpdate } from "../bindings/SubscriptionUpdate";

export type { Subscription } from "../bindings/Subscription";
export type { SubscriptionDraft } from "../bindings/SubscriptionDraft";

const MILLISECONDS_PER_DAY = 86_400_000;
export const MONTHS_PER_YEAR = 12;
export const MAX_RENEWAL_DAY = 28;

export function fetchSubscriptions(): Promise<Subscription[]> {
  return invoke("list_subscriptions");
}

export function createSubscription(draft: SubscriptionDraft): Promise<Subscription> {
  return invoke("create_subscription", { draft });
}

export function updateSubscription(update: SubscriptionUpdate): Promise<Subscription> {
  return invoke("update_subscription", { update });
}

export function deleteSubscription(id: string): Promise<void> {
  return invoke("delete_subscription", { id });
}

export function isSubscriptionError(error: unknown): error is SubscriptionError {
  return typeof error === "object" && error !== null && (
    "invalidInput" in error ||
    "notFound" in error ||
    "storeCorrupt" in error ||
    "storeUnavailable" in error ||
    "committedWithDurabilityWarning" in error
  );
}

function startOfDayUtc(today: Date): number {
  return Date.UTC(today.getFullYear(), today.getMonth(), today.getDate());
}

export function nextRenewal(subscription: Subscription, today: Date): Date {
  const start = startOfDayUtc(today);
  const month = subscription.cycle === "yearly"
    ? (subscription.renewalMonth ?? 1) - 1
    : today.getMonth();
  const currentYearRenewal = Date.UTC(today.getFullYear(), month, subscription.renewalDay);
  const nextYear = subscription.cycle === "yearly" ? today.getFullYear() + 1 : today.getFullYear();
  const nextMonth = subscription.cycle === "yearly" ? month : today.getMonth() + 1;
  return new Date(currentYearRenewal >= start
    ? currentYearRenewal
    : Date.UTC(nextYear, nextMonth, subscription.renewalDay));
}

export function daysUntil(renewal: Date, today: Date): number {
  return Math.round((renewal.getTime() - startOfDayUtc(today)) / MILLISECONDS_PER_DAY);
}

export function formatAmount(amount: number, currency: Subscription["currency"], locale: string): string {
  return new Intl.NumberFormat(locale, { style: "currency", currency }).format(amount);
}

export function formatRenewalDate(value: Date, locale: string): string {
  return new Intl.DateTimeFormat(locale, {
    day: "numeric",
    month: "long",
    year: "numeric",
    timeZone: "UTC",
  }).format(value);
}

export function monthlyEquivalent(subscription: Subscription): number {
  return subscription.cycle === "yearly" ? subscription.amount / MONTHS_PER_YEAR : subscription.amount;
}

export function monthlyTotalsByCurrency(subscriptions: readonly Subscription[]): Map<Subscription["currency"], number> {
  const totals = new Map<Subscription["currency"], number>();
  for (const subscription of subscriptions) {
    totals.set(subscription.currency, (totals.get(subscription.currency) ?? 0) + monthlyEquivalent(subscription));
  }
  return totals;
}

export function sortByRenewal(subscriptions: readonly Subscription[], today: Date): Subscription[] {
  return [...subscriptions].sort((left, right) => nextRenewal(left, today).getTime() - nextRenewal(right, today).getTime());
}
