/**
 * AI subscription model for the Subscriptions page.
 *
 * Pure data and pure functions only: no I/O, no clock reads. Every function
 * that needs "now" receives it as an argument, mirroring the core convention
 * where measured values are passed in by the caller instead of sampled inside
 * the model. The sample entries below are illustrative placeholders, not a
 * reading of the user's real billing accounts.
 */

export type BillingCycle = "monthly" | "yearly";

export type Currency = "EUR" | "USD";

export interface Subscription {
  readonly id: string;
  /** Product or vendor name shown as the card heading. */
  readonly provider: string;
  /** Plan tier, e.g. "Pro" or "Team". */
  readonly plan: string;
  readonly amount: number;
  readonly currency: Currency;
  readonly cycle: BillingCycle;
  /** Day of the month the plan renews. Capped at 28 so every month has it. */
  readonly renewalDay: number;
  /** Renewal month (1-12). Required for yearly plans, ignored for monthly ones. */
  readonly renewalMonth?: number;
}

/**
 * Illustrative subscriptions used to populate the page while no billing source
 * exists. Renewal dates are derived from the current date, so the sample never
 * drifts into the past.
 */
export const SAMPLE_SUBSCRIPTIONS: readonly Subscription[] = [
  {
    id: "claude-pro",
    provider: "Claude Pro",
    plan: "Pro",
    amount: 18.99,
    currency: "EUR",
    cycle: "monthly",
    renewalDay: 4,
  },
  {
    id: "chatgpt-plus",
    provider: "ChatGPT Plus",
    plan: "Plus",
    amount: 20,
    currency: "EUR",
    cycle: "monthly",
    renewalDay: 12,
  },
  {
    id: "github-copilot",
    provider: "GitHub Copilot",
    plan: "Pro",
    amount: 100,
    currency: "EUR",
    cycle: "yearly",
    renewalDay: 21,
    renewalMonth: 3,
  },
  {
    id: "cursor",
    provider: "Cursor",
    plan: "Pro",
    amount: 20,
    currency: "EUR",
    cycle: "monthly",
    renewalDay: 27,
  },
  {
    id: "gemini-advanced",
    provider: "Google Gemini",
    plan: "AI Pro",
    amount: 21.99,
    currency: "EUR",
    cycle: "monthly",
    renewalDay: 8,
  },
  {
    id: "midjourney",
    provider: "Midjourney",
    plan: "Standard",
    amount: 288,
    currency: "EUR",
    cycle: "yearly",
    renewalDay: 15,
    renewalMonth: 9,
  },
];

const MILLISECONDS_PER_DAY = 86_400_000;

/** Midnight UTC of the calendar day `date` falls on, in local terms. */
function startOfDayUtc(date: Date): number {
  return Date.UTC(date.getFullYear(), date.getMonth(), date.getDate());
}

/**
 * Next renewal date on or after `today`. Monthly plans roll to the following
 * month once the day has passed; yearly plans roll to the following year.
 */
export function nextRenewal(subscription: Subscription, today: Date): Date {
  const cursor = startOfDayUtc(today);
  const year = today.getFullYear();

  if (subscription.cycle === "yearly") {
    const month = (subscription.renewalMonth ?? 1) - 1;
    const thisYear = Date.UTC(year, month, subscription.renewalDay);
    return new Date(thisYear >= cursor ? thisYear : Date.UTC(year + 1, month, subscription.renewalDay));
  }

  const thisMonth = Date.UTC(year, today.getMonth(), subscription.renewalDay);
  return new Date(
    thisMonth >= cursor ? thisMonth : Date.UTC(year, today.getMonth() + 1, subscription.renewalDay),
  );
}

/** Whole days between `today` and `renewal`. Zero means it renews today. */
export function daysUntil(renewal: Date, today: Date): number {
  return Math.round((renewal.getTime() - startOfDayUtc(today)) / MILLISECONDS_PER_DAY);
}

/** Sorts a copy of the list by soonest renewal first. Never mutates the input. */
export function sortByRenewal(
  subscriptions: readonly Subscription[],
  today: Date,
): Subscription[] {
  return [...subscriptions].sort(
    (left, right) => nextRenewal(left, today).getTime() - nextRenewal(right, today).getTime(),
  );
}

/** Monthly-equivalent cost: yearly plans are spread across twelve months. */
export function monthlyEquivalent(subscription: Subscription): number {
  return subscription.cycle === "yearly" ? subscription.amount / 12 : subscription.amount;
}

/**
 * Monthly spend grouped by currency. Amounts in different currencies are never
 * summed together, so a mixed list yields one total per currency.
 */
export function monthlyTotalsByCurrency(
  subscriptions: readonly Subscription[],
): Map<Currency, number> {
  const totals = new Map<Currency, number>();
  for (const subscription of subscriptions) {
    const current = totals.get(subscription.currency) ?? 0;
    totals.set(subscription.currency, current + monthlyEquivalent(subscription));
  }
  return totals;
}

export function formatAmount(amount: number, currency: Currency, locale: string): string {
  return new Intl.NumberFormat(locale, {
    style: "currency",
    currency,
    maximumFractionDigits: 2,
  }).format(amount);
}

export function formatRenewalDate(renewal: Date, locale: string): string {
  return new Intl.DateTimeFormat(locale, {
    day: "numeric",
    month: "long",
    year: "numeric",
    timeZone: "UTC",
  }).format(renewal);
}
