# Design: Application Logging

> Trace: new capability **`application-logging`**; deltas on `desktop-shell`, `component-freshness`,
> `workspace-architecture`, `inventory-ui`, `frontend-i18n`; cross-reference only on
> `scan-orchestration`. Bounded by **CA-16**.
> Inherits `archive/2026-08-24-add-client-version-freshness/design.md` (**FRD**) §8 (the sanctioned
> write location), §10 (the test seam: stub `app_data_dir()` with a temp directory), §14 (RED-first).
> `rules.design` coverage: core data model (§2), core/Tauri isolation (§3), IPC contract surface (§8),
> platform paths (§4), error paths (§10).
> Settled by the proposal and **not reopened here**: log inside `app_data_dir()`; CA-16 unamended;
> no PII redaction; `log` facade + single-owner sink; path shown as selectable text via a sixth
> command; 1 MiB + one predecessor; plain-text fixed columns; silent per line, loud once at init;
> no logging in `vertice-core`.

## 0. What is verified, and what is not

No shell was run in this phase — **the agent executing it had no Bash tool**, so every `cargo`
command below is prescribed, not executed. Facts marked V were read from files in this tree.

| # | Statement | Basis |
|---|---|---|
| V1 | `log 0.4.33` is a **non-optional direct dependency of `tauri 2.11.5`** and of `tauri-utils 2.9.3` | `Cargo.lock:1804, 3251, 3412` |
| V2 | `chrono 0.4.45`, `jiff 0.2.35` and `time 0.3.55` reach the lockfile **only** as feature-gated optional dependencies of `serde_with 3.22.0` (a `tauri-utils` dependency). `time` additionally has unconditional edges from `cookie` and `plist` | `Cargo.lock:2900-2919, 340, 2346` |
| V3 | `chrono`'s whole subtree (`iana-time-zone`, `num-traits`) exists in the lock **for chrono alone** — no other package names either | Grepped `Cargo.lock`: only hits are `313-322`, `1367`, `1937` |
| V4 | `num_threads` is **absent** from the lock, and `time`'s dependency list is `deranged, num-conv, powerfmt, serde_core, time-core, time-macros` — so `time`'s `local-offset` feature is **off** today, and `formatting` is off too (`itoa` is not among its deps, though `itoa` is already compiled via `http`/`hyper`) | `Cargo.lock:3517-3528`; grep for `num_threads` → zero hits |
| V5 | `deny.toml` sets `[graph] all-features = true`, `exclude-dev = true`. `[bans].deny` is exactly `tauri`, `tauri-build`, `reqwest`. Licence allow-list is ten SPDX ids | Read `deny.toml:20-75` |
| V6 | `read_only_audit.rs` hard-codes **one** exception (`:16`), skips it wholesale (`:109`), asserts the command list with `assert_eq!` (`:22-31`), greps `lib.rs` for the five handler names (`:89-99`), and applies three proof obligations to `cache.rs` only (`:132-148`) | Read the test |
| V7 | Every `create_dir_all` under `crates/vertice-app/src/` is inside a `#[cfg(test)]` block; `cache::save` (`cache.rs:80-84`) is a bare `fs::write`; both call sites discard with `let _ =` (`commands.rs:145`, `freshness/mod.rs:195`) | Read all three files |
| V8 | Edition is `2021` and `unsafe_code = "deny"`; no `set_var` call exists anywhere under `crates/` | `Cargo.toml:7,19-20`; grep → zero hits |
| **U1** | The declared MSRV of `chrono 0.4.45`, and whether promoting it enables a subtree that fails at the 1.88 floor | **Half closed (orchestrator, 2026-08-24).** `cargo info chrono` reports `rust-version: 1.62.0` for `chrono 0.4.45` itself, comfortably under the workspace floor of `1.88` (`Cargo.toml:8`), so the crate is not the risk. What remains unverified is the *transitive* set that `features = ["clock"]` pulls in (notably the `iana-time-zone` subtree on each platform): a dependency of chrono can declare a higher floor than chrono does. The §5 closing commands still apply, narrowed to that subtree |
| **U2** | Windows `app_data_dir()` mapping to *roaming* `%APPDATA%` | **CLOSED (2026-08-24), confirmed against a running build.** The app was launched on Windows 10 and wrote both `vertice.log` and `freshness-cache.json` to `C:\Users\<user>\AppData\Roaming\com.vertice.app\`. Roaming, not local — the convention held. `AppData\Local\com.vertice.app\` contains only the WebView2 `EBWebView` profile directory, which Tauri's webview owns and which is unrelated to `app_data_dir()`. This closes the risk empirically; the design still writes no literal path, so the guarantee does not depend on it |

**V2 corrects the proposal's D1 reasoning.** "Already in `Cargo.lock`, therefore already compiled at
the MSRV floor" is true for `log` (V1) and **false for every date crate** (V2/V3): the lockfile is
resolved feature-independently, so an optional dependency appears whether or not it is built. The
*licence* half of the argument survives — `deny.toml` runs with `all-features = true` (V5), so all
three date crates are already inside the gated graph. The *MSRV* half does not (U1).

## 1. Technical approach

One file, `crates/vertice-app/src/logging.rs`, owns the logging crate, the log file, the line
format, the size counter and the rotation — the same single-owner shape as `vertice-core/src/yaml.rs`
and `freshness/cache.rs`. `vertice-core` is untouched. Observation points read values that
already exist on returned reports and hand them to `log::info!`/`log::warn!`.

```
 lib.rs run()
   └─ .setup(app)  ── app_data_dir() ─┬─ logging::init()  → set_boxed_logger  (once; Err → stderr once)
                                      └─ log::info!("vertice {version} starting")

 commands.rs   scan/rescan ─ run_scan(label) ─ ScanReport ─┐
               freshness   ─ build_report()  ─ FreshnessReport ─┤
               set_freshness_settings ─ cache::save Err ────────┤
                                                                ▼
                                     log::info!/warn!  (file:line captured at THIS call site)
                                                                ▼
                    logging.rs   format_line()  →  Mutex<LogFile{ file, written }>
                                                        │  written + n > 1 MiB ?
                                                        ├─ yes → rename(current → predecessor); File::create
                                                        └─ write_all(line)   (no BufWriter)

 vertice-core: no dependency, no module, no port.  A future vertice-cli extracts logging.rs
 verbatim into a third crate that depends on neither tauri nor vertice-core (proposal D6).
```

## 2. Core data model changes: **none**

`Component`, `Location`, `Scope`, `SearchRoot`, `ScanReport`, `ScanIssue`, `IssueSeverity`,
`ClientPresence`, `FreshnessReport`, `FreshnessCheck` are **unchanged**. `crates/vertice-core/`
gains no file, no dependency, no signature change, and `model/`'s import allow-list is untouched.
No new `ts_rs::TS` type is introduced (§8), so `frontend/src/bindings/` must regenerate
byte-identical and CI's bindings-in-sync check stays green **with no bindings commit**. If a
bindings diff appears during apply, something in this design was violated — treat it as a failure,
not as a file to commit.

## 3. Core/Tauri isolation

`logging.rs` names `log` and a date crate; it does **not** name `tauri`. It receives
`app_data_dir: &Path` as a parameter, exactly as `cache::store_path` does — resolution stays in
`commands.rs::resolve_app_data_dir` / `lib.rs`'s setup hook. That is what makes the module
liftable into a shared crate later without touching a line of it, and what makes it unit-testable
without constructing a Tauri `App`. `deny.toml` is **not modified**: `log` and the date crate are
not banned, and banning them would be theatre — neither can smuggle Tauri or a socket into core.

## 4. Platform paths

The path is `app_data_dir()/vertice.log`, with `vertice.log.1` beside it after the first rotation.
`app_data_dir()` is resolved at runtime from the bundle identifier `com.vertice.app`
(`crates/vertice-app/tauri.conf.json:5`):

| OS | Directory (Tauri/`dirs` convention) |
|---|---|
| Linux | `$XDG_DATA_HOME/com.vertice.app/`, defaulting to `~/.local/share/com.vertice.app/` |
| Windows | `%APPDATA%\com.vertice.app\` — **roaming**, per convention only (U2) |
| macOS | `~/Library/Application Support/com.vertice.app/` |

**No literal absolute path, no environment read, and no OS-conditional branch appears in
`logging.rs`.** The three rows above are documentation of what Tauri resolves; the code sees one
`&Path`. This is what keeps U2 harmless: whatever Tauri returns is what is written and what the UI
displays. Any spec text or support instruction naming a literal Windows directory would be a guess.

## 5. Decision: the date crate, and the local-offset trap (proposal risk 3)

| Option | Consequence | Decision |
|---|---|---|
| `time 0.3.55` | Already compiled (V2), but `UtcOffset::current_local_offset()` returns `Err(IndeterminateOffset)` in a multi-threaded process on Unix — a deliberate soundness guard against `localtime_r`/`setenv`, not a bug to work around. Tauri's runtime is multi-threaded, so this fails **always**, on the two platforms it applies to. Would also need `formatting` and `local-offset` enabled (V4), the latter pulling `libc` + `num_threads` | **Rejected** — cannot satisfy D4 |
| `jiff 0.2.35` | Correct and thread-safe by design, but the whole subtree is optional-only today (V2): `jiff`, `jiff-core`, and on Windows the bundled `jiff-tzdb` + `jiff-tzdb-platform`, which embeds the IANA database into the binary. Most crates and most bytes for the same output | **Rejected** |
| **`chrono 0.4.45`, `default-features = false, features = ["clock"]`** | Smallest real delta: three compiled crates (`chrono`, `iana-time-zone`, `num-traits` — V3), all already licence-gated (V5). Since 0.4.20 `Local` does **not** call `localtime_r`; it parses the platform zone itself (`iana-time-zone` on Windows/macOS, `/etc/localtime` on Linux), so it is usable from any thread. `default-features = false` drops `serde`, `wasmbind` and `oldtime` | **Chosen** |

Format: `Local::now().to_rfc3339_opts(SecondsFormat::Millis, false)` →
`2026-08-24T14:03:11.482+02:00`. One whitespace-free token, lexicographically sortable, carrying the
explicit offset D4 requires.

**The residual caveat, stated rather than hidden.** `chrono::Local` reads `TZ`, and edition 2021
still allows safe `std::env::set_var`, so a concurrent `set_var` would be a data race. V8 shows no
`set_var` exists in this workspace and `unsafe_code = "deny"` blocks the FFI route to one; the
exposure is limited to a future contributor adding one, which is a reviewable diff. This is a
strictly smaller risk than `time`'s, which is not a caveat but a guaranteed failure.

**U1 is closed by measurement during apply, in this order** (`sdd-apply` must run these and record
the output; if any fails, the fallback ladder is `jiff` → UTC-only via `time` + `formatting`):

```
cargo tree -e no-dev -p vertice-app --target x86_64-pc-windows-msvc   # new-crate delta
cargo deny check bans licenses                                        # must need no new allow-list entry
cargo check --workspace --locked --all-targets                        # at the 1.88 floor
```

## 6. Decision: the sink module's shape (proposal risk 6)

**One file, not a folder** — `crates/vertice-app/src/logging.rs`. A `logging/` directory would need
one audit exception entry per file (§7 keys exceptions on the relative path), turning a two-entry
exception list into four. The module is small enough that this costs nothing.

```rust
// crates/vertice-app/src/logging.rs — the ONLY module that names `log` or `chrono`,
// and the second (and last) module CA-16 permits to write.
pub const FILE_NAME: &str = "vertice.log";
pub const ROTATED_FILE_NAME: &str = "vertice.log.1";
pub const MAX_BYTES: u64 = 1024 * 1024;

/// Mirrors `cache::store_path`: a child of `app_data_dir`, never a literal path.
pub fn log_path(app_data_dir: &Path) -> PathBuf;

/// Construct the sink and install it as the global `log` implementation.
/// Returns Err once, at startup, if the directory or the file is unusable.
pub fn init(app_data_dir: &Path) -> Result<(), InitError>;

/// The testable sink, deliberately separable from the global installation:
/// unit tests exercise rotation and format against a temp directory with no
/// global logger involved (FRD §10's stubbed-`app_data_dir()` seam).
pub(crate) struct FileSink { path: PathBuf, rotated: PathBuf, state: Mutex<LogFile> }
struct LogFile { file: File, written: u64 }
impl FileSink {
    pub(crate) fn open(app_data_dir: &Path) -> io::Result<Self>;
    pub(crate) fn write_line(&self, line: &str);   // infallible by contract (D5 class 1)
}
impl log::Log for FileSink { /* enabled / log / flush */ }

/// Pure, no I/O, no clock — asserted column-by-column by its own test.
fn format_line(timestamp: &str, level: log::Level, file: &str, line: u32, message: &str) -> String;
```

Load-bearing details:

- **The size is tracked in memory.** `LogFile.written` is seeded from `file.metadata().len()` once
  at `open()` and incremented by the byte length of each line actually written. The per-line size
  check is therefore an integer comparison, **never a `metadata()` syscall**. That is the answer to
  risk 6: under the lock there is at most one `write_all`, plus a `rename`+`create` on the rare
  rotation.
- **Formatting happens outside the lock.** `log()` builds the whole `String` (timestamp, level,
  `record.file()`, `record.line()`, `record.args()`) first, then acquires the mutex only to write it.
- **Poison is absorbed**: `state.lock().unwrap_or_else(|e| e.into_inner())`. A panic elsewhere must
  not turn logging into a second panic (D5 class 1).
- **No `BufWriter`.** A support log's value is what survives a crash; each record is one `write_all`
  of one complete line. At tens of lines per scan the syscall cost is irrelevant.
- **Embedded newlines in a message are replaced with a space** before writing, so "one line per
  event" is a real invariant and not a hope.
- **Level filter `LevelFilter::Info`**, fixed. `set_boxed_logger` returning `Err` on a second call
  is the mechanical guarantee that initialisation happens exactly once.

Line shape (fixed columns, two spaces between them, `LEVEL` left-padded to 5):

```
2026-08-24T14:03:11.482+02:00  INFO   crates/vertice-app/src/commands.rs:49  scan finished in 812 ms
2026-08-24T14:03:11.483+02:00  WARN   crates/vertice-app/src/commands.rs:57  search root not found: claude-skills
```

`record.file()` is kept verbatim (workspace-relative) rather than trimmed to a basename: two
`commands.rs` in different crates would otherwise be indistinguishable. Because the macros capture
the call site, every line from a shared observation helper reports that helper's location — which is
correct, since the observation point *is* `commands.rs`.

## 7. Decision: rotation mechanics (never lose a line, never tear the predecessor)

Rotation is evaluated **before** the write, never during it:

1. Compute the fully formatted line and its byte length `n`.
2. Acquire the lock. If `written > 0 && written + n > MAX_BYTES` → rotate.
3. Rotate = flush the handle, drop it, `fs::rename(current, rotated)` (a single atomic replace of
   the predecessor on both POSIX and Win32 `MoveFileEx(REPLACE_EXISTING)` — the predecessor is
   never truncated in place, so it cannot be observed half-written), `File::create(current)`,
   `written = 0`.
4. `write_all(line)`; on success `written += n`.

Consequences, each individually tested:

- A line is always written **whole, into exactly one file**. There is no partial line, because the
  line is a single `write_all` of a complete buffer and rotation cannot interleave with it (same
  lock).
- `written > 0` in the guard prevents rotating an empty file when a single line exceeds `MAX_BYTES`;
  that line is written whole and the file exceeds the bound once.
- **If rotation fails, the line is still written to the current file.** Preserving evidence beats
  honouring the ceiling: on Windows a text editor holding the file open can block the rename, and
  dropping log lines for the duration is exactly the silent-failure shape D5 forbids. The next line
  retries the rotation, so the condition self-heals; the ceiling is a soft ceiling under contention
  and this is a deliberate, documented trade.
- If the `write_all` itself fails, the failure is swallowed (D5 class 1) and `written` is not
  advanced.

## 8. IPC contract surface

One command is added; the total becomes **six**. `crates/vertice-app/capabilities/default.json`
stays exactly `["core:default"]` with no `"scope"` block — the grant is reaffirmed, not widened.

```rust
// crates/vertice-app/src/commands.rs, placed after `resolve_app_data_dir`
/// Returns the absolute path of the application log so the frontend can render
/// it as selectable text. Performs no I/O at all — a path join — which is why it
/// is the one command that does not offload to `spawn_blocking`. `async` because
/// the audit's `exported_tauri_commands` matcher keys on `pub async fn <name>(`.
#[tauri::command]
pub async fn log_file_path(app: tauri::AppHandle) -> Result<String, ScanError> {
    let app_data_dir = resolve_app_data_dir(&app)?;
    Ok(crate::logging::log_path(&app_data_dir).to_string_lossy().into_owned())
}
```

| Aspect | Contract |
|---|---|
| Name | `log_file_path` |
| Returns | `String` — `to_string_lossy` is total and cannot fail; `display()` would be equivalent but the owned `String` is what serde needs |
| Errors | `ScanError::Internal { reason }` only, reusing `resolve_app_data_dir`'s existing mapping. No new error variant, no new taxonomy |
| New `ts_rs` type | **None.** `ScanError.ts` already exists; `String` maps to `string` |
| Events | None. No Tauri event is emitted by this change |
| Frontend | New `frontend/src/lib/appLog.ts` → `invoke<string>("log_file_path")`, mirroring `lib/scan.ts` |
| UI | `ScanPage.svelte`, below `ScanIssueList`: a labelled, selectable `<code data-testid="log-path">` |
| i18n | `scan.logPathLabel` + `scan.logPathHint` added to the `scan` block of `catalogs.ts` (interface ~`:92`, `en` ~`:267`, `es` ~`:439`) — both locales, first commit |

`lib.rs` gains `commands::log_file_path` inside `generate_handler!` and a `.setup(...)` hook that
initialises the sink. `read_only_audit.rs` changes deliberately in three places: the `assert_eq!`
list (`:22-31`), the `lib_source.contains` checks and the `handler` description string (`:89-99`),
and the `exported_tauri_commands` matcher (`:290-300`). The `desktop-shell` spec's "exactly five
commands" becomes six, restating that the new one causes no write.

## 9. Decision: directory creation (Slice A), independently revertible

The fix goes in **`freshness/cache.rs::save`** — the module that is *already* the sole sanctioned
exception (V6/V7) — and nowhere else:

```rust
pub fn save(path: &Path, store: &FreshnessStore) -> std::io::Result<()> {
    let serialized = serde_json::to_string(store).expect("FreshnessStore serialization cannot fail");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;   // `path` came from `store_path(app_data_dir)`
    }
    fs::write(path, serialized)
}
```

| Question | Resolution |
|---|---|
| Why here and not in `commands.rs` or a shared bootstrap? | `create_dir` is a denied mutation pattern. Putting it in `cache.rs` needs **zero** change to the audit's exception list — the diff is one crate, one function, one test. Any other location forces the audit change into the same commit as the fix, destroying the independent revertibility the proposal requires |
| Does the audit still hold? | Yes, unchanged: `cache.rs` still contains `app_data_dir` (in `store_path`'s doc and signature), still contains no literal path, still reads no environment. `parent()` derives from the argument, which derives from `app_data_dir` |
| Does `let _ =` stay at the call sites? | **Yes, in Slice A.** Disk-full, permissions and antivirus interference remain genuinely transient; the correct fix was to remove the *permanent* failure, not to start rejecting settings writes. In **Slice C** the two sites become `if let Err(err) = … { log::warn!("could not persist freshness store: {err}") }` — the result is still not affected, but the silence becomes evidence. That ordering keeps the dependency pointing one way: logging depends on A, A depends on nothing |
| Independent revert | Slice A is one commit: `create_dir_all` + its regression test. Reverting the logging work cannot touch it |
| Does `logging.rs` also create the directory? | Yes — `FileSink::open` calls `create_dir_all` on `app_data_dir` before `OpenOptions::append`. The two are not shared: sharing a helper would put a write primitive in a third module and require a third audit exception. Duplicating four lines across the only two sanctioned writers is the cheaper structural choice |

## 10. Decision: the audit's second exception — narrower, not looser (proposal risk 4)

Today the exception is a blanket `continue` (`:109`): `cache.rs` may use **any** of the 16 patterns.
The new shape grows the list to two entries while *shrinking* what each entry may do.

```rust
struct SanctionedWriter { module: &'static str, allowed: &'static [&'static str] }

/// CA-16's complete exception surface. Every entry is audited by
/// `assert_write_path_is_derived_from_app_data_dir` below; membership alone
/// grants nothing.
const SANCTIONED_WRITERS: [SanctionedWriter; 2] = [
    SanctionedWriter { module: "freshness/cache.rs",
        allowed: &["fs::write", "create_dir"] },
    SanctionedWriter { module: "logging.rs",
        allowed: &["OpenOptions", ".write(", ".write_all(", "File::create",
                   "create_dir", "fs::rename", "std::fs::rename"] },
];
```

| Obligation | How it is enforced |
|---|---|
| Sanctioned modules are still scanned | The loop no longer `continue`s. It looks the module up in `SANCTIONED_WRITERS` and reports any forbidden pattern **not** in that module's own `allowed` list. `remove_file`, `remove_dir`, `.set_len(`, `set_permissions`, `hard_link`, `symlink_file`, `symlink_dir` stay denied *everywhere*, including inside both exceptions |
| The three path proofs apply per module | `assert_write_path_is_derived_from_app_data_dir(module, production_source, &mut findings)` — contains `app_data_dir`, contains no `std::env::`/` env::`, contains none of the five literal-path markers — is called **in a loop over `SANCTIONED_WRITERS`**. The proofs are structurally attached to membership: a third entry cannot be added without acquiring them |
| Growth is a reviewed event | `assert_eq!(SANCTIONED_WRITERS.len(), 2)` in the test body, and each entry's file must be readable (a missing module fails, exactly as `cache.rs`'s `expect` does today at `:129`) |
| Unused permission is not silently held | If a sanctioned module stops using one of its `allowed` patterns, the entry is over-broad. Reporting that is desirable but adds churn; recorded as an open question (§14), not implemented |

This is the concrete answer to "a name list is not the invariant": after this change, being named in
the list grants a *specific enumerated set of syscalls* in a module that must still prove its path
derivation, rather than a blanket pass.

## 11. Event coverage and levels

| Event | Level | Observation point | Source value |
|---|---|---|---|
| Startup | INFO | `lib.rs` `.setup`, after `logging::init` | `env!("CARGO_PKG_VERSION")` |
| Sink init failed | — | `lib.rs` `.setup` | `eprintln!` once; the app still starts (D5 class 2) |
| Scan start / end + duration | INFO | `run_scan(label)` in `commands.rs`, so `scan` and `rescan` are both covered and distinguishable | `ScanReport.duration_ms`, `components.len()` |
| Search root not found | WARN, one line each | `commands.rs`, over the returned report | `SearchRootStatus::NotFound` in `roots_scanned` |
| Client not detected | WARN, one line each | `commands.rs`, over the returned report | `ClientPresenceStatus::NotDetected` in `client_presence` |
| Freshness undetermined | WARN, one line each | `commands.rs::freshness`, after `build_report` | `Freshness::Unknown { reason }` in `checks` |
| Freshness store not persisted | WARN | `commands.rs:145`, `freshness/mod.rs:195` (Slice C) | the discarded `io::Error` |

`run_scan()` gains a `&'static str` label parameter (`"scan"` / `"rescan"`); its three existing
callers in `commands.rs`'s test module are updated in the same commit.

## 12. Error paths (ScanIssue taxonomy)

**The `ScanIssue` taxonomy is unchanged** — no new severity, no new reason string, no new producer.
`ScanIssue` is *read* by the observation points and mirrored into WARN lines; `scan-orchestration`'s
"Visible and Isolated Diagnostics" keeps owning what is recorded, and the log is an orthogonal sink.
Degradation ladder:

| Failure | Behaviour |
|---|---|
| `app_data_dir()` unresolvable at startup | `eprintln!` once; no logger installed; `log::*` calls become no-ops; app runs |
| Directory not creatable / file not openable | `InitError` → `eprintln!` once (D5 class 2); app runs |
| Per-line write fails | Swallowed, `written` not advanced; the command's result, timing and `ScanReport` are byte-identical (D5 class 1) |
| Rotation fails | Line written to the current file anyway; ceiling exceeded until the rename succeeds (§7) |
| Mutex poisoned | `into_inner()`; logging continues |
| `log_file_path` cannot resolve the directory | `ScanError::Internal` — the *only* error this command can return; the UI falls back to hiding the element |

## 13. File changes

| File | Action | Description |
|---|---|---|
| `crates/vertice-app/src/logging.rs` | Create | The sink: `log_path`, `init`, `FileSink`, `format_line`, rotation. Sole owner of `log` + `chrono` and of the log file |
| `crates/vertice-app/src/lib.rs` | Modify | `mod logging;`, `.setup(...)` init + startup line, `commands::log_file_path` in `generate_handler!` |
| `crates/vertice-app/src/commands.rs` | Modify | `log_file_path`; `run_scan(label)`; report/freshness observation points; `let _ =` → logged warning |
| `crates/vertice-app/src/freshness/cache.rs` | Modify | **Slice A**: `create_dir_all(parent)` in `save`, plus its regression test |
| `crates/vertice-app/src/freshness/mod.rs` | Modify | `:195` `let _ =` → logged warning (Slice C) |
| `crates/vertice-app/Cargo.toml` | Modify | `log = "0.4"`; `chrono = { version = "0.4", default-features = false, features = ["clock"] }`, with the same comment discipline as the `reqwest` entry |
| `crates/vertice-app/tests/read_only_audit.rs` | Modify | `SANCTIONED_WRITERS`, per-module allow-lists, looped proofs, six commands |
| `frontend/src/lib/appLog.ts` | Create | `invoke<string>("log_file_path")` |
| `frontend/src/lib/appLog.test.ts` | Create | Mocked-`invoke` unit test, mirroring `scan.test.ts` |
| `frontend/src/lib/pages/ScanPage.svelte` | Modify | Selectable log-path element + label |
| `frontend/src/lib/i18n/catalogs.ts` | Modify | `scan.logPathLabel`, `scan.logPathHint` — `en` and `es` |
| `crates/vertice-core/**` | **Unchanged** | Stated explicitly; any diff here is a design violation |
| `frontend/src/bindings/**` | **Unchanged** | Must regenerate byte-identical (§2) |
| `deny.toml`, `capabilities/default.json` | **Unchanged** | No new ban, no new grant |

## 14. Testing strategy (`strict_tdd: true` — RED first)

Every row is a failing test written before the code that satisfies it.

| # | RED assertion | Seam | GREEN |
|---|---|---|---|
| A1 | `save()` against a path whose parent does **not** exist returns `Ok` and the file is readable | `cache.rs` unit test; temp path built but **not** created, unlike the existing `temp_app_data_dir` helper | `create_dir_all(parent)` (§9) |
| A2 | Write settings → drop → read back through a never-created dir yields the written values (the "toggle survives restart" regression) | `commands.rs` unit test on `write_/read_freshness_settings` | Same |
| B1 | `format_line` emits exactly `ts␣␣LEVEL␣␣file:line␣␣msg\n`, `LEVEL` padded to 5, one trailing `\n`, no interior newline even when the message contains one | Pure function, no clock, no I/O | `format_line` |
| B2 | The timestamp token parses as RFC 3339 **with a non-empty offset** and contains no space | Format the current time, assert with a parser, not a golden string | `chrono::Local` (§5) |
| B3 | Writing N lines past `MAX_BYTES` leaves exactly two files; the predecessor holds the earlier lines whole; the current file holds the newest line whole; no line appears twice or truncated | `FileSink::open(temp_dir)` with a test-visible smaller `MAX_BYTES` **or** synthetic long lines | §7 |
| B4 | A sink whose file was removed under it keeps returning from `write_line` — no panic, no `Err` propagated | `FileSink` against a temp dir, file deleted mid-test | D5 class 1 |
| B5 | `init` on an unwritable directory returns `Err` **and** the process continues | `logging::init` with a path that cannot be created | D5 class 2 |
| C1 | A `ScanReport` carrying a `NotFound` root / a `NotDetected` client / an `Unknown{reason}` check produces one WARN line each, carrying the reason verbatim | Factor observation into `fn log_scan_report(&ScanReport)` / `fn log_freshness_report(&FreshnessReport)` taking a `&dyn Fn(&str)`-shaped sink or asserted through a temp-dir `FileSink` | §11 |
| C2 | `scan()`'s returned report is identical with and without a working sink | `commands.rs` unit test | D5 class 1 |
| E1 | The audit asserts **six** commands and fails while `lib.rs` still lists five | `read_only_audit.rs` | §8 |
| E2 | The audit fails while `logging.rs` does not exist, and fails again if `logging.rs` contains `remove_file` or a literal `C:\` | `read_only_audit.rs` | §10 |
| E3 | The audit fails if `cache.rs` gains a pattern outside its two-entry allow-list | `read_only_audit.rs` | §10 |
| F1 | `fetchLogFilePath()` invokes `"log_file_path"` and returns the string unmodified | Vitest, `vi.mock("@tauri-apps/api/core")` — same shape as `scan.test.ts` | `appLog.ts` |
| F2 | The scan route renders `[data-testid="log-path"]` with the returned path, in `en` and in `es` | `App.test.ts` | `ScanPage.svelte` |

Gates, all of which must pass: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -D warnings`, `cargo test --workspace --locked`, `cargo deny check bans licenses`, and from `frontend/`: `npm run lint && npm run check && npm run test && npm run build`. Run vitest from `frontend/`, not `frontend/src/`.

## 15. Migration / rollout

No data migration. On first run after this change the app data directory is created and
`vertice.log` appears; an existing installation gains both without user action. Nothing reads an
old format, because nothing existed.

**Expected, not a regression:** the freshness response cache begins persisting for the first time on
every machine (§9), so live reference lookups stop happening on every launch and follow the
already-specified 6 h TTL / 7 d stale ceiling (`cache.rs:17,21`). Verification should assert that
behaviour rather than flag the change in network traffic.

## 16. Open questions

- [ ] Should the audit also fail when a sanctioned module holds an `allowed` pattern it no longer
      uses (over-broad permission)? Deferred — correct in principle, noisy in practice (§10).
- [ ] `MAX_BYTES` is tested via synthetic long lines rather than a test-only override; if
      `sdd-apply` finds that awkward, a `pub(crate)` constructor taking the limit is acceptable and
      does not change any published behaviour (§14 B3).
- [ ] U1 (chrono's MSRV under the 1.88 floor) is closed by the three commands in §5 during apply,
      not here. The fallback ladder is stated; no code should be written against `chrono` before the
      first of those commands has been run.
