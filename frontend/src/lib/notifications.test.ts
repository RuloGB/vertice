import { describe, expect, it } from "vitest";
import type { ClientInstallSlot } from "../bindings/ClientInstallSlot";
import type { FreshnessCheck } from "../bindings/FreshnessCheck";
import type { FreshnessReport } from "../bindings/FreshnessReport";
import type { ScanReport } from "../bindings/ScanReport";
import {
  collectHomeNotices,
  collectNotices,
  outdatedClientNotices,
  registerNoticeSource,
  subscriptionRenewalNotices,
  type Notice,
  type NoticeTranslator,
} from "./notifications";
import { daysUntil, nextRenewal, type Subscription } from "./subscriptions";

const today = new Date(2026, 4, 28);

function translate(key: string, params?: Record<string, string | number>): string {
  if (params === undefined) {
    return key;
  }
  const rendered = Object.entries(params)
    .map(([name, value]) => `${name}=${String(value)}`)
    .join(",");
  return `${key}(${rendered})`;
}

const t: NoticeTranslator = translate;

function monthly(overrides: Partial<Subscription> = {}): Subscription {
  return {
    id: "monthly",
    provider: "Provider",
    plan: "Pro",
    amount: 20,
    currency: "EUR",
    cycle: "monthly",
    renewalDay: 28,
    renewalMonth: null,
    updatedAt: "2026-01-01T00:00:00.000000001Z",
    ...overrides,
  };
}

function daysFromToday(subscription: Subscription): number {
  return daysUntil(nextRenewal(subscription, today), today);
}

function outdatedCheck(slot: ClientInstallSlot, installed: string, latest: string): FreshnessCheck {
  return {
    subject: { clientInstallation: { slot, path: `C:/clients/${slot}` } },
    installed,
    verdict: { outdated: { latest } },
  };
}

function freshnessReport(checks: FreshnessCheck[], enabled = true): FreshnessReport {
  return { enabled, checks };
}

function emptyReport(): ScanReport {
  return {
    components: [],
    installations: [],
    rootsScanned: [],
    issues: [],
    clientPresence: null,
    durationMs: 0,
  };
}

function notice(id: string, title: string, detail: string, badge: string): Notice {
  return { id, title, detail, badge };
}

describe("outdatedClientNotices", () => {
  it("maps each outdated slot to its own notice and does not collapse two Claude installs", () => {
    const notices = outdatedClientNotices({
      freshness: freshnessReport([
        outdatedCheck("claudeCodeNpm", "1.0.19", "1.0.20"),
        outdatedCheck("claudeCodeBundled", "1.0.18", "1.0.20"),
        outdatedCheck("openCodeNpm", "1.4.2", "1.5.0"),
        outdatedCheck("codexStandalone", "0.98.0", "0.99.0"),
        outdatedCheck("openCodeDesktop", "0.1.0", "0.2.0"),
        {
          subject: { clientInstallation: { slot: "codexStandalone", path: "C:/clients/codex-current" } },
          installed: "0.99.0",
          verdict: "upToDate",
        },
        {
          subject: { clientInstallation: { slot: "openCodeNpm", path: "C:/clients/opencode-unknown" } },
          installed: "1.4.2",
          verdict: { unknown: { reason: "registry timeout" } },
        },
      ]),
      reportPresent: true,
      t,
    });

    expect(notices).toEqual([
      notice(
        "outdated:claudeCodeNpm",
        "Claude Code",
        "1.0.19",
        "home.outdatedUpdateAvailable(latest=1.0.20)",
      ),
      notice(
        "outdated:claudeCodeBundled",
        "Claude Code",
        "1.0.18",
        "home.outdatedUpdateAvailable(latest=1.0.20)",
      ),
      notice("outdated:openCodeNpm", "OpenCode", "1.4.2", "home.outdatedUpdateAvailable(latest=1.5.0)"),
      notice(
        "outdated:codexStandalone",
        "Codex",
        "0.98.0",
        "home.outdatedUpdateAvailable(latest=0.99.0)",
      ),
    ]);
  });

  it("returns no notices when freshness is missing, disabled, or the scan report is absent", () => {
    const freshness = freshnessReport([outdatedCheck("claudeCodeNpm", "1.0.19", "1.0.20")]);

    expect(outdatedClientNotices({ freshness: null, reportPresent: true, t })).toEqual([]);
    expect(
      outdatedClientNotices({
        freshness: { ...freshness, enabled: false },
        reportPresent: true,
        t,
      }),
    ).toEqual([]);
    expect(outdatedClientNotices({ freshness, reportPresent: false, t })).toEqual([]);
  });
});

describe("subscriptionRenewalNotices", () => {
  it("includes renewals due today through five days, including a next-month date, and excludes six days", () => {
    const dueToday = monthly({ id: "due-today", provider: "Anthropic", plan: "Pro", renewalDay: 28 });
    const tomorrow = monthly({ id: "tomorrow", provider: "OpenAI", plan: "Plus", renewalDay: 29 });
    const inFiveDays = monthly({ id: "in-five", provider: "Cursor", plan: "Team", renewalDay: 2 });
    const inSixDays = monthly({ id: "in-six", provider: "SST", plan: "Basic", renewalDay: 3 });

    expect(daysFromToday(dueToday)).toBe(0);
    expect(daysFromToday(tomorrow)).toBe(1);
    expect(daysFromToday(inFiveDays)).toBe(5);
    expect(daysFromToday(inSixDays)).toBe(6);

    expect(
      subscriptionRenewalNotices({
        subscriptions: [inSixDays, dueToday, inFiveDays, tomorrow],
        today,
        t,
      }),
    ).toEqual([
      notice("renewal:due-today", "Anthropic", "Pro", "subscriptions.renewsToday"),
      notice("renewal:in-five", "Cursor", "Team", "subscriptions.renewsInDays(days=5)"),
      notice("renewal:tomorrow", "OpenAI", "Plus", "subscriptions.renewsTomorrow"),
    ]);
  });

  it("honors an explicit withinDays window", () => {
    const inSixDays = monthly({ id: "in-six", provider: "SST", plan: "Basic", renewalDay: 3 });

    expect(daysFromToday(inSixDays)).toBe(6);
    expect(
      subscriptionRenewalNotices({
        subscriptions: [inSixDays],
        today,
        t,
        withinDays: 6,
      }).map(({ id }) => id),
    ).toEqual(["renewal:in-six"]);
  });
});

describe("collectNotices", () => {
  it("keeps source order and the first id, and isolates a thrown or rejected load", async () => {
    const notices = await collectNotices([
      {
        id: "throws",
        load: () => {
          throw new Error("sync failure");
        },
      },
      {
        id: "first",
        load: async () => [
          { id: "shared", title: "First", detail: "kept", badge: "b1" },
          { id: "shared", title: "Duplicate in source", detail: "dropped", badge: "b2" },
          { id: "only-first", title: "Only", detail: "a", badge: "b3" },
        ],
      },
      {
        id: "rejects",
        load: () => Promise.reject(new Error("async failure")),
      },
      {
        id: "second",
        load: async () => [
          { id: "shared", title: "Second", detail: "dropped", badge: "b4" },
          { id: "only-second", title: "Second only", detail: "b", badge: "b5" },
        ],
      },
    ]);

    expect(notices).toEqual([
      { id: "shared", title: "First", detail: "kept", badge: "b1" },
      { id: "only-first", title: "Only", detail: "a", badge: "b3" },
      { id: "only-second", title: "Second only", detail: "b", badge: "b5" },
    ]);
  });
});

describe("collectHomeNotices", () => {
  it("returns the due renewal when the freshness loader throws", async () => {
    const dueToday = monthly({ id: "due", provider: "Anthropic", plan: "Pro", renewalDay: 28 });

    await expect(
      collectHomeNotices({
        report: emptyReport(),
        today,
        t,
        loadFreshness: () => {
          throw new Error("freshness down");
        },
        loadSubscriptions: async () => [dueToday],
      }),
    ).resolves.toEqual([notice("renewal:due", "Anthropic", "Pro", "subscriptions.renewsToday")]);
  });

  it("returns the outdated notice when the subscription loader throws", async () => {
    await expect(
      collectHomeNotices({
        report: emptyReport(),
        today,
        t,
        loadFreshness: async () => freshnessReport([outdatedCheck("codexStandalone", "0.9.0", "1.0.0")]),
        loadSubscriptions: () => {
          throw new Error("subscriptions down");
        },
      }),
    ).resolves.toEqual([
      notice("outdated:codexStandalone", "Codex", "0.9.0", "home.outdatedUpdateAvailable(latest=1.0.0)"),
    ]);
  });

  it("resolves with no notices when both loaders throw", async () => {
    await expect(
      collectHomeNotices({
        report: emptyReport(),
        today,
        t,
        loadFreshness: () => {
          throw new Error("freshness down");
        },
        loadSubscriptions: () => {
          throw new Error("subscriptions down");
        },
      }),
    ).resolves.toEqual([]);
  });

  it("omits outdated notices when the scan report is missing and still returns renewals", async () => {
    const dueToday = monthly({ id: "due", provider: "OpenAI", plan: "Plus", renewalDay: 28 });

    await expect(
      collectHomeNotices({
        report: null,
        today,
        t,
        loadFreshness: async () => freshnessReport([outdatedCheck("openCodeNpm", "1.4.2", "1.5.0")]),
        loadSubscriptions: async () => [dueToday],
      }),
    ).resolves.toEqual([notice("renewal:due", "OpenAI", "Plus", "subscriptions.renewsToday")]);
  });

  it("appends registered sources after built-in notices and removes only the unregistered one", async () => {
    const unregisterExtra = registerNoticeSource({
      id: "extra",
      load: async () => [notice("extra:1", "Extra", "plugin", "new")],
    });
    const unregisterStays = registerNoticeSource({
      id: "stays",
      load: async () => [notice("stays:1", "Stays", "still registered", "kept")],
    });

    try {
      const withBoth = await collectHomeNotices({
        report: emptyReport(),
        today,
        t,
        loadFreshness: async () => freshnessReport([outdatedCheck("openCodeNpm", "1.4.2", "1.5.0")]),
        loadSubscriptions: async () => [
          monthly({ id: "due", provider: "OpenAI", plan: "Plus", renewalDay: 2 }),
        ],
      });

      expect(withBoth.map(({ id }) => id)).toEqual([
        "outdated:openCodeNpm",
        "renewal:due",
        "extra:1",
        "stays:1",
      ]);

      unregisterExtra();

      const afterExtra = await collectHomeNotices({
        report: emptyReport(),
        today,
        t,
        loadFreshness: async () => freshnessReport([outdatedCheck("openCodeNpm", "1.4.2", "1.5.0")]),
        loadSubscriptions: async () => [
          monthly({ id: "due", provider: "OpenAI", plan: "Plus", renewalDay: 2 }),
        ],
      });

      expect(afterExtra.map(({ id }) => id)).toEqual(["outdated:openCodeNpm", "renewal:due", "stays:1"]);
    } finally {
      unregisterExtra();
      unregisterStays();
    }
  });

  it("isolates a registered source that throws and still resolves the built-in notices", async () => {
    const unregister = registerNoticeSource({
      id: "broken",
      load: () => {
        throw new Error("plugin down");
      },
    });

    try {
      await expect(
        collectHomeNotices({
          report: emptyReport(),
          today,
          t,
          loadFreshness: async () => freshnessReport([outdatedCheck("codexStandalone", "0.9.0", "1.0.0")]),
          loadSubscriptions: async () => [
            monthly({ id: "due", provider: "Anthropic", plan: "Pro", renewalDay: 28 }),
          ],
        }),
      ).resolves.toEqual([
        notice(
          "outdated:codexStandalone",
          "Codex",
          "0.9.0",
          "home.outdatedUpdateAvailable(latest=1.0.0)",
        ),
        notice("renewal:due", "Anthropic", "Pro", "subscriptions.renewsToday"),
      ]);
    } finally {
      unregister();
    }
  });
});
