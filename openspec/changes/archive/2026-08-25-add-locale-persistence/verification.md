# Verification Report: add-locale-persistence

Date: 2026-08-25
Verifier: sdd-verify (adversarial pass; independent of apply-agent's own gate run)

## Gates (re-run independently)

| Gate | Result |
|---|---|
| cargo fmt --all --check | PASS |
| cargo clippy --workspace --all-targets -- -D warnings | PASS |
| cargo test --workspace --locked | PASS (all suites green, including settings/store.rs, commands.rs, freshness/mod.rs, read_only_audit.rs) |
| cargo deny check bans licenses | PASS (bans ok, licenses ok; two informational warnings, pre-existing and non-blocking) |
| frontend: npm run lint | PASS |
| frontend: npm run check | PASS (0 errors, 0 warnings, 224 files) |
| frontend: npm run test | PASS (124 tests, 14 files) |
| frontend: npm run build | PASS |

Per instructions, green gates are treated as necessary, not sufficient - the findings below are from
reading production code directly against each spec requirement.

## Per-requirement verification

### 1. Asymmetric fallback (settings/store.rs) - SATISFIED

crates/vertice-app/src/settings/store.rs:38-52 (load) classifies in exactly the spec's order:
Err(NotFound) -> Missing (line 49); any other Err -> Unreadable (line 50); Ok(contents) where
contents.trim().is_empty() -> Unreadable (lines 41-43, checked before parsing, so an empty file never
reaches the parser); a parse failure -> Unreadable (line 46); otherwise Loaded (line 45).

resolve (lines 60-74) is a pure, total match: Missing -> enabled: true, Unreadable -> enabled: false,
both with locale: None, disclosure_seen: false. Confirmed by dedicated tests:
never_created_file_loads_as_missing_and_resolves_enabled_true (line 139),
corrupt_file_resolves_enabled_false (line 158), empty_file_resolves_enabled_false (line 173, writes
b""), whitespace_only_file_resolves_enabled_false (line 184, writes b"   \n"), and
resolve_is_conservative_for_every_unreadable_producer (line 195, a pure call against
LoadOutcome::Unreadable directly). All pass under cargo test.

End-to-end: freshness/mod.rs build_report (lines 91-92) reads enabled exclusively through
settings::store::{load, resolve}, never through the freshness cache - confirmed by
unreadable_settings_document_disables_the_check_and_issues_no_request (freshness/mod.rs:303-313),
which corrupts settings.json directly and asserts !report.enabled and an empty checks list. The
freshness cache (freshness/cache.rs) no longer has enabled/disclosure_seen fields at all (its struct
is { cache: HashMap<String, CacheEntry> }, cache.rs:40-43).

### 2. Partial patch / two independent writers - SATISFIED

commands.rs write_user_settings (lines 233-264) does read-modify-write: it loads the current
document, then applies each field only if let Some(...) (lines 242-249) - an omitted field is never
touched. frontend/src/lib/settings.ts setUserSettings (lines 22-32) sends omitted fields as explicit
null, and the Rust side's Option<String>/Option<bool> deserializes JSON null to None.

Concurrent-writer scenario proven directly: a_locale_patch_does_not_clobber_enabled
(commands.rs:539-561) persists enabled: Some(false) via one call, then patches locale: Some("es")
with the other two fields None via a second, independent call, and asserts enabled is still false
afterward. ClientsPage.svelte's two write sites are each single-field: dismissDisclosure sends
{ disclosureSeen: true } only (line 150), toggleEnabled sends { enabled } only (line 155). App.svelte
persistLocale sends { locale } only (line 33). App.test.ts's new case asserts
mockedSetUserSettings.toHaveBeenCalledWith({ locale: "es" }) - a single-key object, not a full-state
object. No caller anywhere sends all three fields on every call.

### 3. Command surface - SATISFIED, exactly six

lib.rs:57-64 generate_handler! lists exactly: scan, rescan, freshness, user_settings,
set_user_settings, log_file_path - six entries, no seventh. read_only_audit.rs's
desktop_shell_exposes_only_scan_commands_and_core_default_capability test independently re-derives
the command list by scanning commands.rs for #[tauri::command]-annotated pub async fn declarations
(exported_tauri_commands, lines 345-375) and asserts it equals the same six-item list (lines 51-61).
Grepped the whole crate: no freshness_settings or set_freshness_settings identifier remains anywhere
except historical doc comments describing the rename.

### 4. CA-16 / read-only audit - SATISFIED

read_only_audit.rs:24-45 names exactly three SanctionedWriter entries: freshness/cache.rs,
logging.rs, settings/store.rs. Each is proved individually by
assert_write_path_is_derived_from_app_data_dir (lines 170-180, looped over all three), checking for
an app_data_dir reference, absence of std::env::/ env::, and absence of literal absolute-path
markers, run against production source with #[cfg(test)] blocks stripped.

settings/store.rs's allow-list is pinned at ["fs::write", "create_dir", "fs::rename"], and
settings_store_allow_list_does_not_extend_beyond_its_own_three_entries (lines 444-466) asserts every
other forbidden pattern (remove_file, remove_dir, OpenOptions, File::create, .write_all(, .set_len(,
set_permissions, hard_link, symlink_file, symlink_dir) is denied specifically inside that module.
Direct read of settings/store.rs confirms it contains exactly fs::write (line 83),
fs::create_dir_all (line 81), fs::rename (line 89) - no remove_file anywhere. The audit scans every
.rs file under src/, not just the sanctioned ones, so a remove_file anywhere in the crate would fail
the audit; the audit passed.

Every sanctioned write path derives its directory from the caller-supplied app_data_dir: &Path
parameter - never a literal path, never std::env::. capabilities/default.json (read in full) grants
"permissions": ["core:default"] only - no fs:, shell:, dialog:, no "scope" key.

### 5. Core purity - SATISFIED

crates/vertice-core/src/model/settings.rs imports only serde::{Deserialize, Serialize} and ts_rs::TS
- no std::fs, std::io, std::env, SystemTime, or Instant anywhere in the file. cargo tree -p
vertice-core --locked | grep -i tauri returned no match - vertice-core has zero transitive dependency
on tauri or any tauri-* crate.

### 6. Write durability - SATISFIED

stage() (store.rs:79-85) does create_dir_all + fs::write of settings.json.tmp, and returns without
committing. commit() (store.rs:88-90) is the sole fs::rename. save() (store.rs:97-102) calls stage
then commit - two distinct, independently testable steps. The direct proof:
an_interrupted_write_leaves_the_previous_document_intact (store.rs:256-278) saves an original
document, then calls stage() only (never commit()) with a different, "interrupted" payload, and
asserts resolve(load(&path)) still returns the original values. Passed under cargo test.

### 7. First paint / bounded timeout - SATISFIED

frontend/src/main.ts:17-19: resolveInitialLocale(fetchUserSettings, navigator.languages).then(...) -
no top-level await. initialLocale.ts:34-50 (resolveInitialLocale) races load() against
Promise.race([load(), timeout]) where timeout resolves to null after timeoutMs (default
SETTINGS_TIMEOUT_MS = 500). If the race settles to null (timeout won) or load() rejects or the
resolved locale is unsupported, it falls through to resolveLocale(languages) - never throws, never
hangs past the bound. initialLocale.test.ts's timeout test uses fake timers with a load that never
settles and asserts the overall call still resolves after advancing exactly SETTINGS_TIMEOUT_MS -
direct proof a hung IPC call cannot produce a permanently blank window, since mount() in main.ts is
gated on this same promise resolving.

### 8. Locale precedence - SATISFIED

resolveLocale (locale.svelte.ts:17-31) maps any candidate starting with es- to es (line 22) and
anything not matching es/es-*/en/en-* falls through to the final return "en" (line 30) - so es-MX ->
es and pt-BR -> en both hold, confirmed by resolveLocale's own logic and exercised at the
resolveInitialLocale layer by initialLocale.test.ts. Precedence chain end to end: main.ts calls
resolveInitialLocale(fetchUserSettings, navigator.languages) before mount(), so the value tested in
initialLocale.test.ts is exactly the value that seeds App.svelte's initialLocale prop at real
startup - not merely unit tested in isolation from wiring. The explicit precedence test "prefers a
persisted supported locale over the browser languages" (load resolves { locale: "es" }, languages
["en-US"] -> result "es") proves persisted beats browser at the same call site used at startup.

### 9. Stale binding - SATISFIED

frontend/src/bindings/FreshnessSettings.ts is deleted (git status shows D
frontend/src/bindings/FreshnessSettings.ts). Grepped the entire frontend/src tree for
FreshnessSettings: zero hits. frontend/src/bindings/UserSettings.ts (freshly generated, read in
full) is { locale: string | null, enabled: boolean, disclosureSeen: boolean } - matches
crates/vertice-core/src/model/settings.rs's UserSettings { locale: Option<String>, enabled: bool,
disclosure_seen: bool } under #[serde(rename_all = "camelCase")] field-for-field. npm run check
(which does typecheck, unlike Vitest) passed with 0 errors.

### 10. Sidebar untouched - SATISFIED

git diff HEAD -- frontend/src/lib/Sidebar.svelte and git status --porcelain on that path both
returned empty - byte-identical to HEAD, confirming Sidebar needed no change because it already
called i18n.setLocale, which now writes through via createI18n's onLocaleChange callback.

### App.test.ts rewrite - reviewed independently, judgment CONFIRMED

git diff HEAD -- frontend/src/App.test.ts was read in full. The deleted test asserted the old, buggy
contract: browser locale es-ES, no persisted choice, yet document.documentElement.lang was hardcoded
to "en" at mount. That is precisely the frontend-i18n spec's requirement being violated by the old
code ("Supported browser locale, no persisted choice" -> active locale is es), and is the bug this
change exists to fix - App.svelte's new default is resolveLocale(navigator.languages), not a
hardcoded "en". Keeping the old assertion would make the new, spec-correct code fail a test that
encoded the old, spec-violating behavior. The replacement test is strictly stronger, not weaker: it
asserts the mount resolves to Spanish chrome from es-ES (the corrected behavior), and additionally
exercises switching back to English via the selector and asserts English chrome renders - a behavior
the deleted test never touched at all. Two further new tests are additive assertions with no
counterpart being removed. No assertion strength was reduced anywhere in the diff - every changed
expect(...) either targets updated identifiers (a pure rename with no semantic change) or is a new,
additional assertion. Independent conclusion: this rewrite is legitimate, not a weakening.

## Environment note (non-blocking)

git status --porcelain shows roughly two dozen files under frontend/src/bindings/*.ts as modified,
unrelated to this change's own bindings. git diff and git diff --stat on these paths show zero
content difference - only a CRLF/LF line-ending warning is emitted. This is not caused by this
change's logic and produced no test or gate failure.

## Task list cross-check

45/46 tasks checked. The one remaining unchecked task, I2 (the desktop-shell spec Purpose-paragraph
edit), is explicitly scoped by its own text to happen during archive, not during apply - its being
unchecked at this point is correct, not a gap, provided sdd-archive performs it.

## Overall verdict

PASS - no CRITICAL issues found. Every one of the ten specifically-flagged risk areas was traced
through production code (not just test names) and confirmed to match its governing spec requirement.
All re-run gates are green. The one prior human judgment call (the App.test.ts rewrite) was
independently re-examined and confirmed legitimate - no test was weakened or deleted to paper over
incorrect code; the deleted test encoded the bug being fixed.

No WARNING or SUGGESTION-level findings beyond the non-blocking environment note above.

Nothing must be fixed before archive. sdd-archive should still perform task I2 (the desktop-shell
Purpose-paragraph edit) as part of the archive merge, per its own explicit instructions in tasks.md.
