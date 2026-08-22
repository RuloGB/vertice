import { describe, expect, it } from "vitest";
import {
  daysUntil,
  formatAmount,
  formatRenewalDate,
  monthlyEquivalent,
  monthlyTotalsByCurrency,
  nextRenewal,
  SAMPLE_SUBSCRIPTIONS,
  sortByRenewal,
  type Subscription,
} from "./subscriptions";

const nonBreakingSpace = String.fromCharCode(160);

function monthly(overrides: Partial<Subscription> = {}): Subscription {
  return {
    id: "monthly",
    provider: "Provider",
    plan: "Pro",
    amount: 20,
    currency: "EUR",
    cycle: "monthly",
    renewalDay: 10,
    ...overrides,
  };
}

function yearly(overrides: Partial<Subscription> = {}): Subscription {
  return monthly({
    id: "yearly",
    cycle: "yearly",
    amount: 120,
    renewalMonth: 3,
    ...overrides,
  });
}

describe("nextRenewal", () => {
  it("keeps a monthly renewal in the current month when the day is still ahead", () => {
    expect(nextRenewal(monthly(), new Date(2026, 4, 3))).toEqual(new Date(Date.UTC(2026, 4, 10)));
  });

  it("treats the renewal day itself as still due today", () => {
    expect(nextRenewal(monthly(), new Date(2026, 4, 10))).toEqual(new Date(Date.UTC(2026, 4, 10)));
  });

  it("rolls a monthly renewal into the next month once the day has passed", () => {
    expect(nextRenewal(monthly(), new Date(2026, 4, 11))).toEqual(new Date(Date.UTC(2026, 5, 10)));
  });

  it("rolls a December monthly renewal into January of the next year", () => {
    expect(nextRenewal(monthly(), new Date(2026, 11, 20))).toEqual(new Date(Date.UTC(2027, 0, 10)));
  });

  it("rolls a yearly renewal into the next year once its month and day have passed", () => {
    expect(nextRenewal(yearly(), new Date(2026, 0, 5))).toEqual(new Date(Date.UTC(2026, 2, 10)));
    expect(nextRenewal(yearly(), new Date(2026, 6, 5))).toEqual(new Date(Date.UTC(2027, 2, 10)));
  });
});

describe("daysUntil", () => {
  it("returns zero on the renewal day and whole days otherwise", () => {
    const today = new Date(2026, 4, 3);

    expect(daysUntil(new Date(Date.UTC(2026, 4, 3)), today)).toBe(0);
    expect(daysUntil(new Date(Date.UTC(2026, 4, 4)), today)).toBe(1);
    expect(daysUntil(new Date(Date.UTC(2026, 4, 10)), today)).toBe(7);
  });
});

describe("sortByRenewal", () => {
  it("orders by soonest renewal without mutating the source list", () => {
    const today = new Date(2026, 4, 3);
    const source = [monthly({ id: "late", renewalDay: 20 }), monthly({ id: "soon", renewalDay: 5 })];

    expect(sortByRenewal(source, today).map(({ id }) => id)).toEqual(["soon", "late"]);
    expect(source.map(({ id }) => id)).toEqual(["late", "soon"]);
  });
});

describe("monthly cost", () => {
  it("spreads a yearly plan across twelve months", () => {
    expect(monthlyEquivalent(monthly({ amount: 20 }))).toBe(20);
    expect(monthlyEquivalent(yearly({ amount: 120 }))).toBe(10);
  });

  it("never sums amounts across different currencies", () => {
    const totals = monthlyTotalsByCurrency([
      monthly({ amount: 20, currency: "EUR" }),
      yearly({ amount: 120, currency: "EUR" }),
      monthly({ amount: 15, currency: "USD" }),
    ]);

    expect(totals.get("EUR")).toBe(30);
    expect(totals.get("USD")).toBe(15);
  });

  it("returns an empty map for an empty list", () => {
    expect(monthlyTotalsByCurrency([]).size).toBe(0);
  });
});

describe("formatting", () => {
  it("formats amounts with the locale currency conventions", () => {
    expect(formatAmount(18.99, "EUR", "en-US")).toBe("€18.99");
    expect(formatAmount(18.99, "EUR", "es-ES").split(nonBreakingSpace).join(" ")).toBe(
      "18,99 €",
    );
  });

  it("formats renewal dates in UTC so the day never shifts by timezone", () => {
    const renewal = new Date(Date.UTC(2026, 2, 21));

    expect(formatRenewalDate(renewal, "en-US")).toBe("March 21, 2026");
    expect(formatRenewalDate(renewal, "es-ES")).toBe("21 de marzo de 2026");
  });
});

describe("SAMPLE_SUBSCRIPTIONS", () => {
  it("uses unique ids and renewal days every month actually has", () => {
    const ids = SAMPLE_SUBSCRIPTIONS.map(({ id }) => id);

    expect(new Set(ids).size).toBe(ids.length);
    for (const subscription of SAMPLE_SUBSCRIPTIONS) {
      expect(subscription.renewalDay, subscription.id).toBeGreaterThanOrEqual(1);
      expect(subscription.renewalDay, subscription.id).toBeLessThanOrEqual(28);
      expect(subscription.amount, subscription.id).toBeGreaterThan(0);
    }
  });

  it("gives every yearly plan a renewal month", () => {
    for (const subscription of SAMPLE_SUBSCRIPTIONS.filter(({ cycle }) => cycle === "yearly")) {
      expect(subscription.renewalMonth, subscription.id).toBeGreaterThanOrEqual(1);
      expect(subscription.renewalMonth, subscription.id).toBeLessThanOrEqual(12);
    }
  });

  it("never renews in the past for any reference date", () => {
    const today = new Date(2026, 7, 22);

    for (const subscription of SAMPLE_SUBSCRIPTIONS) {
      expect(daysUntil(nextRenewal(subscription, today), today), subscription.id)
        .toBeGreaterThanOrEqual(0);
    }
  });
});
