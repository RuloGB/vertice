import { createContext } from "svelte";
import { catalogs } from "./catalogs";
import type { CatalogKey, SupportedLocale } from "./catalogs";

export type { CatalogKey, SupportedLocale } from "./catalogs";

export type MessageParams = Record<string, string | number>;

export interface I18nContext {
  readonly locale: SupportedLocale;
  setLocale(locale: SupportedLocale): void;
  t(key: CatalogKey, params?: MessageParams): string;
}

const [getI18nContext, setI18nContext] = createContext<I18nContext>();

export function resolveLocale(languages?: readonly string[] | string | null): SupportedLocale {
  const candidates = Array.isArray(languages) ? languages : languages ? [languages] : [];

  for (const language of candidates) {
    const normalized = language.trim().toLowerCase();
    if (normalized === "es" || normalized.startsWith("es-")) {
      return "es";
    }
    if (normalized === "en" || normalized.startsWith("en-")) {
      return "en";
    }
  }

  return "en";
}

export function formatMessage(message: string, params: MessageParams = {}): string {
  return message.replace(/\{(\w+)\}/g, (placeholder, name: string) => {
    const value = params[name];
    return value === undefined ? placeholder : String(value);
  });
}

export function createI18n(initialLocale: SupportedLocale): I18nContext {
  let locale = $state<SupportedLocale>(initialLocale);

  return {
    get locale() {
      return locale;
    },
    setLocale(nextLocale) {
      locale = nextLocale;
    },
    t(key, params) {
      return formatMessage(messageFor(locale, key), params);
    },
  };
}

export function provideI18n(i18n: I18nContext): I18nContext {
  setI18nContext(i18n);
  return i18n;
}

export function useI18n(): I18nContext {
  return getI18nContext();
}

function messageFor(locale: SupportedLocale, key: CatalogKey): string {
  return key.split(".").reduce<unknown>((node, part) => {
    if (typeof node !== "object" || node === null || !(part in node)) {
      throw new Error(`Missing i18n message: ${locale}.${key}`);
    }
    return (node as Record<string, unknown>)[part];
  }, catalogs[locale]) as string;
}