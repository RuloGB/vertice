import { invoke } from "@tauri-apps/api/core";
import type { UserSettings } from "../bindings/UserSettings";
import type { SupportedLocale } from "./i18n/locale.svelte";

/**
 * Invoke the `user_settings` command: a read-only view of the durable
 * settings document (`locale`, `enabled`, `disclosureSeen`), resolvable
 * before any check has ever run and creating no file as a side effect.
 */
export function fetchUserSettings(): Promise<UserSettings> {
  return invoke<UserSettings>("user_settings");
}

/**
 * Invoke the `set_user_settings` command. A true partial patch: an omitted
 * field is sent as `null`, which the backend treats as "leave this field's
 * persisted value unchanged" — never as a request to reset it. Two
 * independent frontend owners write this document (the application shell
 * for `locale`, the clients page for `enabled`/`disclosureSeen`), so this
 * shape is load-bearing — never widen it into a full-state write.
 */
export function setUserSettings(patch: {
  locale?: SupportedLocale;
  enabled?: boolean;
  disclosureSeen?: boolean;
}): Promise<UserSettings> {
  return invoke<UserSettings>("set_user_settings", {
    locale: patch.locale ?? null,
    enabled: patch.enabled ?? null,
    disclosureSeen: patch.disclosureSeen ?? null,
  });
}
