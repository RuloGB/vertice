# Tasks: Durable User Settings and Locale Persistence

Ordering rule (strict TDD, both layers): every behavioral slice below is a
**RED → GREEN** pair — the failing test is written and confirmed failing
(or non-compiling) before the implementation that satisfies it. Do not batch
all tests ahead of all code. Task IDs carry the spec requirement(s) they
prove in `[]`.

Parallelism legend: **(P)** = can run concurrently with sibling tasks in the
same phase once its phase's prerequisite is met. **(S)** = must run in the
stated sequence; do not reorder.

---

## Phase A — Core model: `UserSettings` (sequential, blocks everything else)

`[user-settings: A Single Durable Settings Document Holds Locale And Opt-Out State — "settings type is defined in core without I/O"]`

- [x] A1 (S) RED — Replace the `FreshnessSettings` round-trip test in
      `crates/vertice-core/tests/model_contract.rs` with
      `user_settings_round_trips_camel_cased`, asserting the three
      camelCase fields (`locale`, `enabled`, `disclosureSeen`) on the wire.
      Confirm it fails to compile (`UserSettings` does not exist yet).
- [x] A2 (S) GREEN — Create `crates/vertice-core/src/model/settings.rs`
      (`UserSettings { locale: Option<String>, enabled: bool,
      disclosure_seen: bool }`, `#[serde(rename_all = "camelCase")]`,
      `#[derive(TS)]` exporting to `../../../frontend/src/bindings/`). Wire
      `mod settings;` + `pub use settings::UserSettings;` into
      `crates/vertice-core/src/model/mod.rs`. Delete the `FreshnessSettings`
      type from `crates/vertice-core/src/model/freshness.rs` and drop it
      from the `freshness` re-export. Run `cargo test -p vertice-core
      --locked` to regenerate bindings and confirm A1 goes green.
- [x] A3 (S) Gotcha — delete
      `frontend/src/bindings/FreshnessSettings.ts` by hand in this same
      commit (`ts_rs` does not clean its export directory; regeneration
      alone leaves a dead file the CI bindings-drift gate cannot see,
      since it only diffs generated files, not extra ones). Grep the whole
      `frontend/` tree for any import of `FreshnessSettings` or
      `bindings/FreshnessSettings` and remove/retarget every hit found.

---

## Phase B — `settings/store.rs` in `vertice-app` (sequential chain; depends on A)

`[user-settings: A Single Durable Settings Document..., An Explicit User Choice Survives A Full Application Restart, The Load Outcome Is A Three-Way Classification...]`

- [x] B1 (S) RED — `store_path_is_a_child_of_the_stubbed_app_data_dir` —
      calls `store_path(&Path)` with no Tauri and no real app data dir.
- [x] B2 (S) GREEN — Create `crates/vertice-app/src/settings/mod.rs`
      (`pub mod store;`) and `crates/vertice-app/src/settings/store.rs`
      with just `store_path` to make B1 pass.
- [x] B3 (S) RED — `never_created_file_loads_as_missing_and_resolves_enabled_true`
      (temp dir per test, mirroring `freshness/cache.rs:119-127`'s
      pid+`AtomicU64` pattern; the file is simply never written).
- [x] B4 (S) GREEN — Implement `LoadOutcome { Missing, Loaded(..),
      Unreadable }`, `load()`'s `Err(NotFound) -> Missing` branch, and
      `resolve()`'s `Missing -> (enabled: true, locale: None,
      disclosure_seen: false)` branch.
- [x] B5 (S) RED — `corrupt_file_resolves_enabled_false` (write
      `b"{ not json"`).
- [x] B6 (S) RED — `empty_file_resolves_enabled_false` and
      `whitespace_only_file_resolves_enabled_false` (write `b""` and
      `b"   \n"` — the torn-write shape).
- [x] B7 (S) GREEN — Implement the rest of `load()`'s classification order
      (non-NotFound `Err` → `Unreadable`; `Ok(s)` with
      `s.trim().is_empty()` → `Unreadable`; parse failure → `Unreadable`)
      and `resolve()`'s asymmetric `Unreadable` branch (`enabled: false`,
      `locale: None`, `disclosure_seen: false`). Confirm B5/B6 green.
- [x] B8 (S) RED — `resolve_is_conservative_for_every_unreadable_producer`
      — pure call `resolve(LoadOutcome::Unreadable)` (covers the
      not-portably-creatable EACCES producer directly, without touching a
      real permission-denied file).
- [x] B9 (S) GREEN — Confirm B8 passes against the existing `resolve()`
      from B7 (should require no new production code if B7 was done
      correctly — if it fails, fix `resolve()`, not the test).
- [x] B10 (S) RED — `valid_document_round_trips_all_three_fields` (calls
      `save()`, which does not exist yet, then `load()`/`resolve()`).
- [x] B11 (S) GREEN — Implement `stage()` (`create_dir_all` +
      `fs::write` of `.tmp`), `commit()` (`fs::rename`), and `save()`
      (`stage` then `commit`), plus the `Loaded` branch of `resolve()`
      (per-field values as stored, serde defaults for genuinely absent
      fields). Confirm B10 green.
- [x] B12 (S) RED — `save_creates_the_app_data_directory_when_it_does_not_yet_exist`
      (path built but deliberately not pre-created, ported from
      `freshness/cache.rs:189-208`).
- [x] B13 (S) GREEN — Confirm `create_dir_all` in `stage()` (added in B11)
      already satisfies B12; add it now if it was deferred.
- [x] B14 (S) RED — `save_leaves_no_temp_file_behind` (`read_dir` count
      == 1, name `settings.json`).
- [x] B15 (S) GREEN — Fix `commit()`/`stage()` naming if B14 exposes a
      leftover `.tmp` file.
- [x] B16 (S) RED — `an_interrupted_write_leaves_the_previous_document_intact`
      — call `stage()` only, never `commit()`; assert `load()`/`resolve()`
      still returns the *old* persisted values. This is the direct proof
      of the atomic-rename design decision.
- [x] B17 (S) GREEN — Confirm B16 passes with no further production code
      (an untouched `settings.json` plus an orphan `.tmp` is already the
      expected shape of `stage`/`commit` as split functions).

---

## Phase C — Commands rename/repurpose (depends on B; can run alongside Phase D)

`[user-settings: A Read Command And A Partial-Patch Write Command..., desktop-shell: Minimal Scan Command Surface, Non-Blocking Command Execution]`

- [x] C1 (P) RED — `a_locale_patch_does_not_clobber_enabled` in
      `commands.rs` tests: persist `enabled: Some(false)`, then patch
      `locale: Some("es")` with other fields `None`; assert `enabled` is
      still `false` afterward.
- [x] C2 (P) RED —
      `writing_settings_survives_a_never_created_app_data_directory`
      (ported from the freshness-settings equivalent), exercising
      `write_user_settings(PathBuf, ..)` directly, without an `AppHandle`.
- [x] C3 (S) GREEN — In `crates/vertice-app/src/commands.rs`: rename
      `#[tauri::command]` fns `freshness_settings`/`set_freshness_settings`
      → `user_settings`/`set_user_settings`; widen the write command's
      signature to `(app, locale: Option<String>, enabled: Option<bool>,
      disclosure_seen: Option<bool>) -> Result<UserSettings, ScanError>`;
      retarget the seams `read_freshness_settings`/`write_freshness_settings`
      → `read_user_settings`/`write_user_settings`, backed by
      `settings::store::{load, resolve, save}` via `spawn_blocking` +
      `resolve_app_data_dir`, implementing read-modify-write so the write
      command returns what was actually persisted. Confirm C1/C2 green.
- [x] C4 (S) — Register the renamed commands in
      `crates/vertice-app/src/lib.rs`'s `generate_handler!` (`mod
      settings;` + `commands::user_settings, commands::set_user_settings`
      in place of the two freshness-settings entries). Final surface:
      `scan, rescan, freshness, user_settings, set_user_settings,
      log_file_path` — six commands total.

---

## Phase D — Freshness cache narrowing (depends on B; can run alongside Phase C)

`[component-freshness: The Check Is Enabled By Default..., The Response Cache Is The Only New Write...]`

- [x] D1 (P) RED —
      `unreadable_settings_document_disables_the_check_and_issues_no_request`
      in `freshness/mod.rs`: corrupt `settings.json` in a temp dir;
      assert `!report.enabled` and no checks performed.
- [x] D2 (S) GREEN — Rename `FreshnessStore` → `FreshnessCache` in
      `crates/vertice-app/src/freshness/cache.rs` (definition, `Default`
      derive, `load`, `save`, the `expect` message, its 6 test sites);
      drop the `enabled`, `disclosure_seen`, and `default_enabled` fields
      entirely (cache narrows to `HashMap<String, CacheEntry>` only).
      Update rename sites in `freshness/mod.rs:18,53,265,267` and
      `commands.rs:266,564,608`. Make `build_report` read `enabled` via
      `settings::store::{load, resolve}` instead of the cache. Confirm D1
      green.
- [x] D3 (S) Cleanup — Remove or rewrite any pre-existing
      `freshness/cache.rs` tests that asserted on `enabled` /
      `disclosure_seen` (now removed from the struct); keep the TTL-entry
      tests passing untouched.

---

## Phase E — Read-only audit (depends on both C and D being complete)

`[user-settings: The Settings Write Path Is A Sanctioned, Individually-Proved Exception..., desktop-shell: The Read-Only Audit Recognizes A Third Write Exception]`

- [x] E1 (S) RED — Update `crates/vertice-app/tests/read_only_audit.rs`:
      the expected six-command list, the `generate_handler!` literal
      check, and the `exported_tauri_commands` matcher arms (renamed
      pair); bump the `SANCTIONED_WRITERS.len() == 2` assert to `3`; add
      the third entry `SanctionedWriter { module: "settings/store.rs",
      allowed: &["fs::write", "create_dir", "fs::rename"] }`; add the new
      test `settings_store_allow_list_does_not_extend_beyond_its_own_three_entries`
      pinning that `remove_file`, `remove_dir`, `OpenOptions`,
      `File::create`, `.write_all(`, `.set_len(`, `set_permissions`,
      `hard_link`, `symlink_*` are all denied inside `settings/store.rs`.
      Confirm these fail against the pre-Phase-C/D command surface (or
      compile-fail if run before C4).
- [x] E2 (S) GREEN — Run the full audit against the completed Phase C/D
      code. `settings/store.rs` must satisfy
      `assert_write_path_is_derived_from_app_data_dir` the same way
      `cache.rs` does (`store_path(app_data_dir: &Path)`, no `std::env::`,
      no literal absolute path) — use `use std::fs;` + `fs::rename(...)`
      to match the audit's exact-pattern matching (it lists `fs::rename`
      and `std::fs::rename` as separate patterns). Confirm
      `an_unsanctioned_module_is_permitted_no_forbidden_pattern` and the
      unchanged cache/logging allow-list tests still pass.

---

## Phase F — Frontend pure logic (independent of Rust backend completion; can start anytime, gated only by A2's bindings for typechecking)

`[frontend-i18n: Supported Locale Resolution]`

- [x] F1 (P) RED — `frontend/src/lib/i18n/initialLocale.test.ts`:
      persisted `es` beats `["en-US"]`; `locale: null` → system;
      unsupported `"pt-BR"` → system → `en`; loader rejects → system;
      loader never settles → system after the 500 ms timeout (fake
      timers). `languages` must be a **parameter**, never a stubbed jsdom
      global.
- [x] F2 (P) GREEN — Create `frontend/src/lib/i18n/initialLocale.ts`
      (`SETTINGS_TIMEOUT_MS = 500`, `isSupportedLocale`,
      `resolveInitialLocale(load, languages, timeoutMs)` — never throws;
      timeout/rejection/unsupported all fall through to
      `resolveLocale(languages)`). Confirm F1 green.
- [x] F3 (P) RED — Add to `frontend/src/lib/i18n/locale.test.ts`:
      `setLocale` invokes `onLocaleChange` exactly once with the new
      locale, still switches translations, and survives a throwing
      callback.
- [x] F4 (P) GREEN — Modify `frontend/src/lib/i18n/locale.svelte.ts`:
      `createI18n(initialLocale, onLocaleChange?)` — callback invoked
      after state update, never awaited. Confirm F3 green.
- [x] F5 (P) RED — `frontend/src/lib/settings.test.ts`:
      `setUserSettings({ locale: "es" })` must invoke with `{ locale:
      "es", enabled: null, disclosureSeen: null }` — pins the omitted →
      `null` wire shape.
- [x] F6 (P) GREEN — Create `frontend/src/lib/settings.ts`
      (`fetchUserSettings`, `setUserSettings`), mirroring
      `frontend/src/lib/freshness.ts`'s existing `invoke` pattern. Modify
      `frontend/src/lib/freshness.ts` to drop the two settings wrappers,
      keeping `fetchFreshness` only. Confirm F5 green.

---

## Phase G — Frontend wiring: `main.ts` / `App.svelte` (depends on F)

`[frontend-i18n: Supported Locale Resolution, desktop-shell: renamed pair adds no new capability grant]`

- [x] G1 (S) RED — Update `frontend/src/App.test.ts`: split
      `vi.mock("../freshness")` from a new `vi.mock("../settings")`; add
      cases mounting with `props: { initialLocale: "es" }` (Spanish
      chrome, `documentElement.lang === "es"`); assert the Sidebar
      language change calls `setUserSettings({ locale })` only (no
      `enabled`/`disclosureSeen`).
- [x] G2 (S) GREEN — Modify `frontend/src/App.svelte`: `let {
      initialLocale = resolveLocale(navigator.languages) }: {
      initialLocale?: SupportedLocale } = $props();` +
      `provideI18n(createI18n(initialLocale, persistLocale))` where
      `persistLocale` calls `setUserSettings({ locale })` and swallows a
      rejection (`.catch(() => {})`). Modify `frontend/src/main.ts` to
      resolve before mounting: `export default
      resolveInitialLocale(fetchUserSettings, navigator.languages).then(
      (initialLocale) => mount(App, { target, props: { initialLocale }
      }));` — no top-level `await`, so no build-target dependency. Confirm
      G1 green. Do **not** touch `frontend/src/lib/Sidebar.svelte` — it
      already calls `i18n.setLocale`, which now writes through via the
      `onLocaleChange` callback; it stays byte-identical.

---

## Phase H — Frontend: `ClientsPage` retarget (depends on F)

`[desktop-shell: A partial patch changes only the fields it names, Two independent writers do not clobber each other's field]`

- [x] H1 (P) RED — Update `frontend/src/lib/pages/ClientsPage.test.ts`:
      split its mock between `../freshness` (report) and `../settings`
      (opt-out/disclosure); assert dismissing the disclosure sends
      `{ disclosureSeen: true }` only, and toggling the opt-out sends
      `{ enabled: false }` only — neither call carries the other field.
- [x] H2 (P) GREEN — Modify `frontend/src/lib/pages/ClientsPage.svelte` to
      call `fetchUserSettings` / `setUserSettings` from
      `frontend/src/lib/settings.ts` instead of the removed freshness-
      settings wrappers. Confirm H1 green.

---

## Phase I — Gotchas requiring an explicit task (not caught by CI)

- [x] I1 Stale-binding cross-check (belt-and-suspenders on A3): after all
      Rust changes land, run `cargo test -p vertice-core --locked` once
      more, then re-grep the entire `frontend/` tree (including
      `frontend/src/lib/**`, tests, and `bindings/`) for `FreshnessSettings`.
      Any hit is a leftover reference; CI's bindings-drift gate cannot
      catch an orphaned file, only a mismatch between Rust and generated
      output, so this check must be manual.
- [x] I2 Archive-time Purpose update (do this when the change is
      archived, not during `apply`): edit
      `openspec/specs/desktop-shell/spec.md` line 5 (the `## Purpose`
      provenance paragraph) to add one clause recording this change. By
      this repo's convention that paragraph is edited directly during the
      archive merge and is never carried in a delta file — it will not
      appear in this change's `specs/desktop-shell/spec.md` delta, and
      nothing else in this checklist will remind you, so do not let it
      get silently dropped at archive time.

---

## Phase J — Verification gates (run last, after all phases above are green)

- [x] J1 `cargo fmt --all --check`
- [x] J2 `cargo clippy --workspace --all-targets -- -D warnings`
- [x] J3 `cargo test --workspace --locked`
- [x] J4 `cargo deny check bans licenses`
- [x] J5 From `frontend/`: `npm run lint && npm run check && npm run test
      && npm run build` — `npm run check` is mandatory here (Vitest does
      not typecheck the bindings, and this change deletes one).

Note: `cargo` may not resolve on PATH in this environment. If any J1–J4
command fails to resolve, report it explicitly as "could not verify —
cargo not on PATH" rather than reporting the gate as passing.

---

## Requirement coverage cross-check

- `user-settings/spec.md`: all five requirements → Phases A, B, C, E.
- `desktop-shell/spec.md`: all four MODIFIED requirements (six-command
  surface, non-blocking execution, minimal capability grant, third audit
  exception) → Phases C, D, E.
- `component-freshness/spec.md`: both MODIFIED requirements (asymmetric
  enabled/disclosure default, cache narrowing) → Phase D.
- `frontend-i18n/spec.md`: the one MODIFIED requirement (persisted,
  restart-surviving locale resolution with fallback) → Phases F, G.

No requirement in the four deltas was left unmapped.
