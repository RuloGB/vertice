import { resolveLocale } from "./locale.svelte";
import type { SupportedLocale } from "./catalogs";

/**
 * The bounded wait before the first paint falls back to
 * `resolveLocale(navigator.languages)`. The read is one local file on the
 * blocking pool — sub-millisecond in practice — so this bound only ever
 * matters on the failure paths (design "Decision 1").
 */
export const SETTINGS_TIMEOUT_MS = 500;

/**
 * Whether `value` is one of the frontend's supported locale codes. An
 * unrecognised persisted value is never treated as an explicit choice.
 */
export function isSupportedLocale(value: unknown): value is SupportedLocale {
  return value === "en" || value === "es";
}

/**
 * Resolve the initial locale before mount: a persisted, durable choice
 * takes precedence when it exists and is supported; otherwise (missing,
 * unsupported, a rejected load, or a load that never settles within
 * `timeoutMs`) falls through to `resolveLocale(languages)` exactly as if no
 * persisted choice existed. Never throws.
 */
export async function resolveInitialLocale(
  load: () => Promise<{ locale: string | null }>,
  languages: readonly string[] | null,
  timeoutMs: number = SETTINGS_TIMEOUT_MS,
): Promise<SupportedLocale> {
  const fallback = (): SupportedLocale => resolveLocale(languages);

  const timeout = new Promise<null>((resolve) => {
    setTimeout(() => resolve(null), timeoutMs);
  });

  let settled: { locale: string | null } | null;
  try {
    settled = await Promise.race([load(), timeout]);
  } catch {
    return fallback();
  }

  if (settled === null || !isSupportedLocale(settled.locale)) {
    return fallback();
  }

  return settled.locale;
}
