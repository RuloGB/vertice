import type { FreshnessReport } from "../bindings/FreshnessReport";
import type { ScanReport } from "../bindings/ScanReport";
import { daysUntil, nextRenewal, type Subscription } from "./subscriptions";

export type Notice = {
  id: string;
  title: string;
  detail: string;
  badge: string;
};

export type NoticeTranslator = (
  key: string,
  params?: Record<string, string | number>,
) => string;

const CLIENT_DISPLAY_NAMES: Record<string, string> = {
  claudeCodeNpm: "Claude Code",
  claudeCodeBundled: "Claude Code",
  openCodeNpm: "OpenCode",
  codexStandalone: "Codex",
};

export function outdatedClientNotices(input: {
  freshness: FreshnessReport | null;
  reportPresent: boolean;
  t: NoticeTranslator;
}): Notice[] {
  const { freshness, reportPresent, t } = input;
  if (!freshness || !freshness.enabled || !reportPresent) {
    return [];
  }

  const notices: Notice[] = [];
  for (const check of freshness.checks) {
    if (!("clientInstallation" in check.subject)) continue;

    const { verdict } = check;
    if (verdict === "upToDate" || !("outdated" in verdict)) continue;

    const slot = check.subject.clientInstallation.slot;
    const displayName = CLIENT_DISPLAY_NAMES[slot];
    if (!displayName) continue;

    notices.push({
      id: `outdated:${slot}`,
      title: displayName,
      detail: check.installed,
      badge: t("home.outdatedUpdateAvailable", {
        latest: verdict.outdated.latest,
      }),
    });
  }

  return notices;
}

export function subscriptionRenewalNotices(input: {
  subscriptions: Subscription[];
  today: Date;
  t: NoticeTranslator;
  withinDays?: number;
}): Notice[] {
  const { subscriptions, today, t, withinDays = 5 } = input;

  const candidates: { subscription: Subscription; days: number }[] = [];
  for (const subscription of subscriptions) {
    const days = daysUntil(nextRenewal(subscription, today), today);
    if (days >= 0 && days <= withinDays) {
      candidates.push({ subscription, days });
    }
  }

  return candidates.map(({ subscription, days }) => {
    let badge: string;
    if (days === 0) {
      badge = t("subscriptions.renewsToday");
    } else if (days === 1) {
      badge = t("subscriptions.renewsTomorrow");
    } else {
      badge = t("subscriptions.renewsInDays", { days });
    }

    return {
      id: `renewal:${subscription.id}`,
      title: subscription.provider,
      detail: subscription.plan,
      badge,
    };
  });
}

type NoticeSource = {
  id: string;
  load: () => Promise<Notice[]> | Notice[];
};

const extraSources: NoticeSource[] = [];

export function registerNoticeSource(source: NoticeSource): () => void {
  extraSources.push(source);
  let removed = false;
  return () => {
    if (removed) return;
    removed = true;
    const index = extraSources.indexOf(source);
    if (index >= 0) {
      extraSources.splice(index, 1);
    }
  };
}

export async function collectNotices(sources: NoticeSource[]): Promise<Notice[]> {
  const results = await Promise.all(
    sources.map(async (source) => {
      try {
        return await source.load();
      } catch {
        return [];
      }
    }),
  );

  const seen = new Set<string>();
  const deduped: Notice[] = [];
  for (const batch of results) {
    for (const notice of batch) {
      if (!seen.has(notice.id)) {
        seen.add(notice.id);
        deduped.push(notice);
      }
    }
  }

  return deduped;
}

async function loadFreshnessSafely(
  load: () => Promise<FreshnessReport | null> | FreshnessReport | null,
): Promise<FreshnessReport | null> {
  try {
    return await load();
  } catch {
    return null;
  }
}

async function loadSubscriptionsSafely(
  load: () => Promise<Subscription[]> | Subscription[],
): Promise<Subscription[]> {
  try {
    return await load();
  } catch {
    return [];
  }
}

export async function collectHomeNotices(input: {
  report: ScanReport | null;
  today: Date;
  t: NoticeTranslator;
  loadFreshness: () =>
    | Promise<FreshnessReport | null>
    | FreshnessReport
    | null;
  loadSubscriptions: () => Promise<Subscription[]> | Subscription[];
}): Promise<Notice[]> {
  const { report, today, t, loadFreshness, loadSubscriptions } = input;

  const freshness = await loadFreshnessSafely(loadFreshness);
  const subscriptions = await loadSubscriptionsSafely(loadSubscriptions);

  const outdated = outdatedClientNotices({
    freshness,
    reportPresent: report !== null,
    t,
  });

  const renewals = subscriptionRenewalNotices({
    subscriptions,
    today,
    t,
  });

  return collectNotices([
    { id: "__outdated__", load: () => outdated },
    { id: "__renewals__", load: () => renewals },
    ...extraSources,
  ]);
}
