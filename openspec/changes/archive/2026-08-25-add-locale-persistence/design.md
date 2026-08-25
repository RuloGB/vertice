# Design: Durable User Settings and Locale Persistence

## Technical Approach

Split persistence by durability semantics. `freshness-cache.json` stays a disposable TTL'd cache
(`FreshnessStore` → `FreshnessCache`, narrowed to `HashMap<String, CacheEntry>`, keeping its cheap
whole-file `fs::write`). A new durable document `settings.json` holds `locale`, `enabled`,
`disclosure_seen`, written through temp-file-plus-rename by a new sanctioned writer module
`crates/vertice-app/src/settings/store.rs`, modelled on `freshness/cache.rs` and reached through
`spawn_blocking` + `resolve_app_data_dir` exactly as `commands.rs:172-258` already does.

`vertice-core/src/model/` gains the plain `UserSettings` type only — no I/O, per its own import
allow-list. `vertice-app` owns every byte written. Tauri capabilities stay `core:default`; no new
dependencies.

## Architecture Decisions

### Decision 1 — First paint: block the mount, bounded

**Choice**: resolve the initial locale **before** `mount()`, racing the IPC read against a 500 ms
timeout that falls back to `resolveLocale(navigator.languages)`.

| Option | Tradeoff |
|---|---|
| Sync from `navigator.languages`, reconcile on arrival | One-frame flash — visible **only** to a returning user whose explicit choice differs from their system language, i.e. exactly the population this feature exists to serve |
| Block the mount unbounded | No flash, but an unresolvable `app_data_dir` or a wedged IPC call yields a permanently blank window |
| **Block the mount, bounded (chosen)** | No flash in the normal case; worst case is a bounded blank shell, then correct system-language resolution |

**Rationale**: the flash lands precisely on the user the change is for, so "the flash is cheap" is
false here. The read is one local file on the blocking pool — sub-millisecond in practice — and the
timeout converts the only failure mode of blocking (an unbounded wait) into the behaviour we would
have had anyway.

`createI18n` gains **no async initializer**. It keeps its synchronous `initialLocale` parameter and
gains an optional write-through callback; the async resolution lives in a pure, injectable function
outside Svelte so it is unit-testable without a component or a real `invoke`.

```ts
// frontend/src/lib/i18n/initialLocale.ts (new, plain .ts — no runes)
export const SETTINGS_TIMEOUT_MS = 500;
export function isSupportedLocale(value: unknown): value is SupportedLocale;
export async function resolveInitialLocale(
  load: () => Promise<{ locale: string | null }>,
  languages: readonly string[] | null,
  timeoutMs = SETTINGS_TIMEOUT_MS,
): Promise<SupportedLocale>; // never throws; timeout/rejection/unsupported → resolveLocale(languages)

// frontend/src/lib/i18n/locale.svelte.ts (modified)
export function createI18n(
  initialLocale: SupportedLocale,
  onLocaleChange?: (locale: SupportedLocale) => void, // called after state update, never awaited
): I18nContext;

// frontend/src/main.ts (modified — no top-level await, so no build-target dependency)
export default resolveInitialLocale(fetchUserSettings, navigator.languages).then((initialLocale) =>
  mount(App, { target, props: { initialLocale } }),
);
```

`App.svelte:25` becomes:

```svelte
let { initialLocale = resolveLocale(navigator.languages) }: { initialLocale?: SupportedLocale } = $props();
const i18n = provideI18n(createI18n(initialLocale, persistLocale));
// persistLocale: void setUserSettings({ locale }).catch(() => {});  — a failed write never surfaces
```

The prop default keeps every existing `App.test.ts` mount compiling. `Sidebar.svelte` is **not
modified**: it already calls `i18n.setLocale`, which now writes through.

### Decision 2 — Write durability: temp-file-plus-rename, with an explicit three-way load outcome

**Choice**: `settings.json` is staged to `settings.json.tmp` in the same directory and committed with
`fs::rename`. The cache's "a torn write is indistinguishable from a corrupt cache" justification does
not transfer, because here the two are distinguishable *and* the distinction drives `enabled`.

The absent/unreadable distinction is made pure and total, so it is table-testable rather than
inferred at the call site:

```rust
// crates/vertice-app/src/settings/store.rs
pub enum LoadOutcome { Missing, Loaded(UserSettingsDocument), Unreadable }

pub fn load(path: &Path) -> LoadOutcome;      // classifies, never panics, never errors out
pub fn resolve(outcome: LoadOutcome) -> UserSettings; // pure; the asymmetric fallback lives here
fn stage(path: &Path, contents: &str) -> io::Result<PathBuf>; // create_dir_all + fs::write of .tmp
fn commit(tmp: &Path, path: &Path) -> io::Result<()>;         // fs::rename
pub fn save(path: &Path, settings: &UserSettings) -> io::Result<()>; // stage then commit
```

Classification, in order:

| `fs::read_to_string` result | Outcome | Why |
|---|---|---|
| `Err(NotFound)` | `Missing` | Only a never-created document produces this |
| `Err(_)` (permission, IO) | `Unreadable` | The file may exist; assume it does |
| `Ok(s)` with `s.trim().is_empty()` | `Unreadable` | **The torn/empty case.** An empty file cannot be created by `save` (rename commits a fully written temp), so its existence is evidence of an anomaly, not of a first run |
| `Ok(s)` that fails to parse | `Unreadable` | Corrupt |
| `Ok(s)` that parses | `Loaded` | Per-field `serde` defaults apply |

`resolve` (the settled asymmetric fallback):

| Outcome | `enabled` | `locale` | `disclosure_seen` |
|---|---|---|---|
| `Missing` | `true` (first run — "The Check Is Enabled By Default" stands) | `None` | `false` |
| `Loaded` | as stored | as stored | as stored |
| `Unreadable` | **`false`** — never silently resume outbound requests | `None` | `false` |

No `remove_file` anywhere (the audit denies it in every module, sanctioned or not). A failed `commit`
leaves the previous `settings.json` intact and an orphan `.tmp` that the next `stage` overwrites.

### Decision 3 — `set_user_settings` is a partial patch, not a full-state write

The existing pair deliberately sends full state ("no ambiguity about which field changed"). That
contract is unsafe once **two independent frontend owners** write the same document: `App` (locale)
and `ClientsPage` (enabled / disclosure). A full-state write from `App` carrying a stale `enabled`
would silently re-enable outbound traffic — the exact failure the asymmetric fallback exists to
prevent. Each field therefore travels as `Option`, `None` meaning "leave unchanged". There is no
clear-locale operation, so `None` is unambiguous.

### Decision 4 — The settings pair is REPLACED, not added (binding: `desktop-shell` spec)

`openspec/specs/desktop-shell/spec.md:11` requires **exactly six** commands and `:17` requires the
settings-write command to be **the only** write-capable command. Adding a user-settings pair
alongside `freshness_settings` / `set_freshness_settings` would make eight commands and two
write-capable commands, breaking both. The pair is therefore **renamed and repurposed**, its payload
widened with `locale`, which follows directly from the settled scope: `enabled` and
`disclosure_seen` move into `settings.json`, so the freshness-named pair has nothing left to own.

**Final surface — six commands, one write-capable:**

| Command | Signature | Write? |
|---|---|---|
| `scan` | `() -> Result<ScanReport, ScanError>` | no |
| `rescan` | `() -> Result<ScanReport, ScanError>` | no |
| `freshness` | `(app) -> Result<FreshnessReport, ScanError>` | cache only (pre-existing, see delta note) |
| `user_settings` *(was `freshness_settings`)* | `(app) -> Result<UserSettings, ScanError>` | no |
| `set_user_settings` *(was `set_freshness_settings`)* | `(app, locale: Option<String>, enabled: Option<bool>, disclosure_seen: Option<bool>) -> Result<UserSettings, ScanError>` | **yes — the only one** |
| `log_file_path` | `(app) -> Result<String, ScanError>` | no |

**Every call site of the renamed pair:**

| Site | Change |
|---|---|
| `crates/vertice-app/src/commands.rs:177-197` | `#[tauri::command]` fns renamed; payload widened |
| `crates/vertice-app/src/commands.rs:217-258` | Seams `read_freshness_settings` / `write_freshness_settings` → `read_user_settings` / `write_user_settings`, retargeted at `settings::store` |
| `crates/vertice-app/src/lib.rs:60-61` | `generate_handler!` entries renamed |
| `crates/vertice-app/tests/read_only_audit.rs:50-56,115-125,359-362` | Expected command list, handler literal, `exported_tauri_commands` matcher arms |
| `frontend/src/lib/freshness.ts:18-35` | Both wrappers **removed** (file keeps `fetchFreshness` only) |
| `frontend/src/lib/settings.ts` | New home of the wrappers |
| `frontend/src/lib/pages/ClientsPage.svelte:7,58,89-101,145-166` | The **only** component calling the pair — it owns both the opt-out toggle and the first-run disclosure |
| `frontend/src/lib/pages/ClientsPage.test.ts:10-21` | `vi.mock("../freshness")` split: freshness report vs `vi.mock("../settings")` |
| `frontend/src/App.test.ts:10,23-34` | Same mock split (it mocks the pair only because it renders `ClientsPage`) |
| `frontend/src/lib/Sidebar.svelte` | **Verified: no call site.** It only calls `i18n.setLocale`, which now writes through via `createI18n`'s callback — Sidebar stays byte-identical |

**Read-only audit: a third sanctioned exception, not a re-point.** `freshness/cache.rs` still writes
`freshness-cache.json` (`build_report` persists refreshed TTL entries at `mod.rs:197`), so its
exception cannot be re-pointed at `settings.json`; the two documents have deliberately different
write semantics (cheap whole-file vs stage-and-rename), which is the whole point of the split. The
audit therefore names **three** modules and must assert, after the change:

- `SANCTIONED_WRITERS.len() == 3` (the reviewed-event assert bumped from 2);
- the new entry `SanctionedWriter { module: "settings/store.rs", allowed: &["fs::write", "create_dir", "fs::rename"] }`;
- `settings/store.rs` proved on its own merits by the existing
  `assert_write_path_is_derived_from_app_data_dir` (references `app_data_dir`, no `std::env::`, no literal absolute path);
- a new `settings_store_allow_list_does_not_extend_beyond_its_own_three_entries`, pinning that
  `remove_file`, `remove_dir`, `OpenOptions`, `File::create`, `.write_all(`, `.set_len(`,
  `set_permissions`, `hard_link`, `symlink_*` stay denied **inside** the new exception;
- the unchanged `an_unsanctioned_module_is_permitted_no_forbidden_pattern` and the cache/logging
  allow-list tests, which must keep passing untouched.

**`desktop-shell` wording that still needs a MODIFIED delta** (for the spec phase):

1. `:11-14` "Minimal Scan Command Surface" — the two settings commands read/write **the persisted
   freshness settings**; must become the durable user-settings document (`locale`, `enabled`,
   `disclosure_seen`). Count stays six; only the names and the document change.
2. `:167-172` "The Read-Only Audit Recognizes A Second Write Exception" — title and body say
   **exactly two** sanctioned write-exception modules; becomes three, with the settings store proved
   on its own merits.
3. Line 5 (Purpose) — the provenance sentence needs one clause for this change.
4. *Pre-existing drift the spec phase should fix while it is in this file*: `:17` claims the
   settings-write command is the only command that can cause a write, yet `freshness` already
   persists refreshed cache entries via `build_report`; and `:65` still says "All **five** commands"
   after the surface grew to six. Neither is caused by this change.

## Data Flow

```
main.ts ──resolveInitialLocale──→ invoke("user_settings") ──→ settings::store::load → resolve
   │                                                                     │
   └──(persisted | timeout/reject → resolveLocale(navigator.languages))──┘
                          ↓
              mount(App, { initialLocale })
                          ↓
   Sidebar select → i18n.setLocale → onLocaleChange → setUserSettings({ locale })
                                                          ↓
                            invoke("set_user_settings") → load → patch → stage(.tmp) → rename

   ClientsPage → setUserSettings({ enabled }) / ({ disclosureSeen })  ─┘ (same path, disjoint fields)
   freshness::build_report → settings::store::load/resolve → enabled? → cache.rs (TTL entries only)
```

## File Changes

| File | Action | Description |
|---|---|---|
| `crates/vertice-core/src/model/settings.rs` | Create | `UserSettings` plain data, `TS`-derived |
| `crates/vertice-core/src/model/mod.rs` | Modify | `mod settings;` + `pub use settings::UserSettings`; drop `FreshnessSettings` from the `freshness` re-export |
| `crates/vertice-core/src/model/freshness.rs` | Modify | Delete `FreshnessSettings` (superseded) |
| `crates/vertice-core/tests/model_contract.rs` | Modify | Replace the `FreshnessSettings` round-trip with `UserSettings` |
| `crates/vertice-app/src/settings/mod.rs` | Create | `pub mod store;` only |
| `crates/vertice-app/src/settings/store.rs` | Create | Document, `LoadOutcome`, `load`/`resolve`/`stage`/`commit`/`save`, `store_path` |
| `crates/vertice-app/src/freshness/cache.rs` | Modify | `FreshnessStore` → `FreshnessCache`; drop `enabled`, `disclosure_seen`, `default_enabled`; `#[derive(Default)]`; module doc no longer claims to be the only writer |
| `crates/vertice-app/src/freshness/mod.rs` | Modify | Rename at `:18,:53,:265,:267`; `build_report` reads `enabled` from the settings store |
| `crates/vertice-app/src/commands.rs` | Modify | `freshness_settings`/`set_freshness_settings` → `user_settings`/`set_user_settings`; seams `read_user_settings`/`write_user_settings` |
| `crates/vertice-app/src/lib.rs` | Modify | `mod settings;` + the two renamed commands in `generate_handler!` |
| `crates/vertice-app/tests/read_only_audit.rs` | Modify | Command list, handler check, `exported_tauri_commands` matcher, third `SanctionedWriter`, `len()` assert 2 → 3, new allow-list test |
| `frontend/src/bindings/UserSettings.ts` | Regenerate | Via `cargo test -p vertice-core` |
| `frontend/src/bindings/FreshnessSettings.ts` | Delete | Rust type gone; `ts_rs` does not clean the export dir, so this must be deleted by hand in the commit |
| `frontend/src/lib/settings.ts` | Create | `fetchUserSettings` / `setUserSettings`, mirroring `lib/freshness.ts` |
| `frontend/src/lib/freshness.ts` | Modify | Keeps `fetchFreshness` only |
| `frontend/src/lib/i18n/initialLocale.ts` | Create | `resolveInitialLocale`, `isSupportedLocale` |
| `frontend/src/lib/i18n/locale.svelte.ts` | Modify | `createI18n` gains `onLocaleChange` |
| `frontend/src/main.ts` | Modify | Resolve, then mount |
| `frontend/src/App.svelte` | Modify | `initialLocale` prop + `persistLocale` |
| `frontend/src/lib/pages/ClientsPage.svelte` | Modify | Retarget to `fetchUserSettings` / `setUserSettings` patches |
| `frontend/src/lib/pages/ClientsPage.test.ts` | Modify | Split `vi.mock` between `../freshness` and `../settings` |
| `frontend/src/App.test.ts` | Modify | Same mock split; add the `initialLocale` prop cases |
| `frontend/src/lib/Sidebar.svelte` | Unchanged | Already calls `setLocale` |

## Interfaces / Contracts

```rust
// crates/vertice-core/src/model/settings.rs — binding: frontend/src/bindings/UserSettings.ts
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../frontend/src/bindings/")]
pub struct UserSettings {
    /// Free-form on purpose: core has no business knowing which catalogs the
    /// frontend ships. An unrecognised value is treated as "no explicit
    /// choice" by the frontend, which then falls through to navigator.languages.
    pub locale: Option<String>,
    pub enabled: bool,
    pub disclosure_seen: bool,
}

// crates/vertice-app/src/commands.rs
#[tauri::command] pub async fn user_settings(app: tauri::AppHandle) -> Result<UserSettings, ScanError>;
#[tauri::command] pub async fn set_user_settings(
    app: tauri::AppHandle,
    locale: Option<String>,
    enabled: Option<bool>,
    disclosure_seen: Option<bool>,
) -> Result<UserSettings, ScanError>; // read-modify-write; returns what was persisted
```

Registered in `lib.rs`'s `generate_handler!` as `commands::user_settings,
commands::set_user_settings` in place of the two freshness-settings entries. Final surface:
`scan, rescan, freshness, user_settings, set_user_settings, log_file_path` — six commands, matching
the audit's updated expectation.

```ts
// frontend/src/lib/settings.ts
export function fetchUserSettings(): Promise<UserSettings>;
export function setUserSettings(patch: {
  locale?: SupportedLocale; enabled?: boolean; disclosureSeen?: boolean;
}): Promise<UserSettings>; // omitted fields sent as null → None → unchanged
```

## Testing Strategy (strict TDD — RED first, in this order)

| # | Layer | Test | How the branch is reached |
|---|---|---|---|
| 1 | core | `user_settings_round_trips_camel_cased` in `model_contract.rs` | Fails to compile until the type exists |
| 2 | app unit | `store_path_is_a_child_of_the_stubbed_app_data_dir` | `store_path(&Path)` — no Tauri, no real app data dir |
| 3 | app unit | `never_created_file_loads_as_missing_and_resolves_enabled_true` | Temp dir per test (`env::temp_dir()` + pid + `AtomicU64`, exactly `cache.rs:119-127`); the file is simply never written |
| 4 | app unit | `corrupt_file_resolves_enabled_false` | Write `b"{ not json"` into the temp dir |
| 5 | app unit | `empty_file_resolves_enabled_false` and `whitespace_only_file_resolves_enabled_false` | Write `b""` / `b"   \n"` — the plausible torn-write shape |
| 6 | app unit | `resolve_is_conservative_for_every_unreadable_producer` | Pure call `resolve(LoadOutcome::Unreadable)` — covers the IO-error producer (EACCES) that is not portably creatable on Windows and Linux alike |
| 7 | app unit | `valid_document_round_trips_all_three_fields` | `save` then `load`/`resolve` |
| 8 | app unit | `save_creates_the_app_data_directory_when_it_does_not_yet_exist` | Path built but deliberately not created (ported from `cache.rs:189-208`) |
| 9 | app unit | `save_leaves_no_temp_file_behind` | `read_dir` count == 1, name `settings.json` |
| 10 | app unit | `an_interrupted_write_leaves_the_previous_document_intact` | Call `stage` only, never `commit`; assert `load`/`resolve` still returns the *old* values — the direct proof of Decision 2 |
| 11 | app unit | `writing_settings_survives_a_never_created_app_data_directory` (`commands.rs`) | Ported; exercises `write_user_settings(PathBuf, …)` without an `AppHandle` |
| 12 | app unit | `a_locale_patch_does_not_clobber_enabled` | Write `enabled: Some(false)`, then `locale: Some("es")` with the other fields `None`; assert `enabled` still `false` — proof of Decision 3 |
| 13 | app unit | `unreadable_settings_document_disables_the_check_and_issues_no_request` (`freshness/mod.rs`) | Corrupt `settings.json` in a temp dir; assert `!report.enabled` and empty checks |
| 14 | app integ | `read_only_audit.rs` updated expectations + `settings_store_allow_list_does_not_extend_beyond_its_own_three_entries` | Asserts `remove_file`, `OpenOptions`, `File::create`, `.write_all(`, `set_permissions` stay denied inside the new sanctioned module |
| 15 | frontend | `initialLocale.test.ts` | Persisted `es` beats `["en-US"]`; `locale: null` → system; unsupported `"pt-BR"` → system → `en`; loader rejects → system; loader never settles → system after the timeout (fake timers). `languages` is a **parameter**, never a stubbed jsdom global |
| 16 | frontend | `locale.test.ts` | `setLocale` invokes `onLocaleChange` exactly once with the new locale, still switches translations, and survives a throwing callback |
| 17 | frontend | `settings.test.ts` | `setUserSettings({ locale: "es" })` invokes with `{ locale: "es", enabled: null, disclosureSeen: null }` — pins the wire shape |
| 18 | frontend | `App.test.ts` | Mount with `props: { initialLocale: "es" }` → Spanish chrome + `documentElement.lang === "es"`; Sidebar change calls `setUserSettings({ locale })` only |
| 19 | frontend | `ClientsPage.test.ts` | Mocks retargeted; dismissal sends `{ disclosureSeen: true }`, toggle sends `{ enabled: false }` — neither carries the other field |

No test touches the machine's real app data directory: every Rust test passes an explicit temp
`app_data_dir` into `store_path`, and every frontend test injects the loader.

Gates: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace --locked`, `cargo deny check bans licenses`, and from `frontend/`
`npm run lint && npm run check && npm run test && npm run build`. `npm run check` is mandatory here —
Vitest does not typecheck the bindings, and this change deletes one.

## Migration / Rollout

No migration. Existing `freshness-cache.json` files deserialize fine (unknown `enabled` /
`disclosure_seen` keys are ignored by serde), and their values are not carried over: `settings.json`
is `Missing` on first run after the upgrade, so `enabled` is `true` and the privacy disclosure
reappears once. Accepted by the proposal.

**Rename mechanics.** `FreshnessStore` → `FreshnessCache` is compiler-enforced and confined to
`freshness/cache.rs` (definition, `Default`, `load`, `save`, the `expect` message, 6 test sites) and
`freshness/mod.rs:18,53,265,267` plus `commands.rs:266,564,608`. The read-only audit is unaffected by
the rename itself — `SANCTIONED_WRITERS` keys on the **path** `"freshness/cache.rs"`, which does not
move. What the audit *does* require is a reviewed edit: the third writer entry
(`"settings/store.rs"`, allowed `["fs::write", "create_dir", "fs::rename"]`), the `len()` assert bump
to 3, the six-command list, the `generate_handler!` literal, and the `exported_tauri_commands`
matcher arms. `store.rs` satisfies `assert_write_path_is_derived_from_app_data_dir` the same way
`cache.rs` does: `store_path(app_data_dir: &Path)`, no `std::env::`, no literal absolute path.
Note the audit lists `fs::rename` and `std::fs::rename` as separate patterns — `store.rs` must
`use std::fs;` and call `fs::rename(...)`, mirroring `cache.rs`, so the single allow-list entry
suffices.

## Open Questions

None blocking. Both design-phase decisions are settled above.
