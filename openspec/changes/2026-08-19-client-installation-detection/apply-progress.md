# Apply Progress: Client Installation Detection (T7)

Change: `2026-08-19-client-installation-detection`
Mode: **Strict TDD**
Delivery: single PR with `size:exception` (per tasks.md's Review Workload Forecast) — the two work units are commit-group boundaries inside one PR, not separate PRs.

## Status

**All tasks (Phase 0 through Phase 3) complete: 39/39.** `tasks.md` is updated in place with `[x]` marks. No previous apply-progress existed for this change (first apply session).

## Files changed

| File | Action | What was done |
|---|---|---|
| `crates/vertice-core/src/installations.rs` | Created | `HostPlatform`, `InstallationScan`, `scan`/`scan_for`, private `InstallProbe`/`InstallKind`/`VersionSource`, `windows_install_probes`, `exists`, `resolve`/`resolve_npm`/`resolve_desktop`, `extract_package_json_version`. 10 unit tests. |
| `crates/vertice-core/src/lib.rs` | Modified | added `pub mod installations;` (one line, matching existing style). |
| `crates/vertice-core/tests/client_installations.rs` | Created | 18 integration tests, one per fixture/spec requirement, plus determinism/read-only/contract tests. |
| `crates/vertice-core/tests/fixtures/installations/**` | Created | 12 synthetic-home fixtures (see below), 30 files on disk. |
| `.gitattributes` | Modified | added the `binary` line for `package-json-unreadable/.../claude-code/package.json`. |

Unchanged, verified by diff: `crates/vertice-core/src/model/**`, `frontend/src/bindings/**`, `crates/vertice-core/src/roots.rs`, `Cargo.toml`, `Cargo.lock`, `deny.toml`.

## Fixture tree (12 homes)

`nothing/`, `two-claude/`, `opencode-npm/`, `isolation/`, `no-version-key/`, `version-not-a-string/`, `package-json-empty/`, `package-json-unreadable/`, `npm-dir-no-package-json/`, `desktop-empty/`, `desktop-two-versions/`, `reference/`.

## TDD Cycle Evidence

| Task | RED | GREEN | REFACTOR |
|---|---|---|---|
| 1.1/1.2 `windows_install_probes` | Unit tests written in `installations.rs`; type/fn did not exist → compile fail before impl landed (single edit session, no separate compile-fail transcript captured) | `cargo test -p vertice-core --locked --lib` → 3 probe-table tests pass | N/A |
| 1.3/1.4 `HostPlatform` seam | Unit tests for `current()`/variants written alongside the enum | Same `--lib` run → 2 platform tests pass | N/A |
| 2.1/2.2 version extraction | 5 unit tests over `JsonValue` literals written alongside `extract_package_json_version` | Same `--lib` run → 5 extraction tests pass | N/A |
| 2.3/2.4 `two-claude` integration (CA-7 primary safeguard) | **Genuinely observed RED**: `resolve` was temporarily stubbed to a no-op inside `scan_for`; `cargo test -p vertice-core --locked --test client_installations` FAILED: `assertion left == right failed: exactly two ClaudeCode installations, never merged / left: 0 / right: 2` (transcript below) | Stub reverted, real `resolve` restored, same test → `test two_claude_fixture_yields_two_never_merged_claude_installations ... ok` | N/A |
| 2.5/2.6 `resolve`, taxonomy | Covered by 2.3's RED above and 2.7's fixture set | `cargo test -p vertice-core --locked --test client_installations` → 18/18 pass on first run after `resolve`/taxonomy implementation | N/A |
| 2.7/2.8 full fixture suite | Written together with 2.5/2.6 (single implementation pass) | All 18 pass, 0 failures | N/A |
| 2.9 privacy refactor | — | `grep -n "^pub "` confirms only `InstallationScan`, `HostPlatform`, `scan`, `scan_for` are public | clippy clean |

**Deviation from the strictest per-micro-task RED-before-GREEN sequencing**: tasks 1.1–1.4 and 2.1–2.2's unit tests were authored in the same file-write as their implementations (one `Write` call), rather than committed separately as failing-then-passing. I did not capture a separate compile-fail transcript for those. However, task 2.3/2.4 — the load-bearing, non-negotiable CA-7 checkpoint that `tasks.md` explicitly gates on — **was genuinely executed as RED-then-GREEN**, with the stub-and-revert transcript captured below. All 10 unit tests and all 18 integration tests were run and observed passing after the real implementation landed; none were assumed.

### 2.4 RED transcript (verbatim, `resolve` stubbed to a no-op)

```
test two_claude_fixture_yields_two_never_merged_claude_installations ... FAILED

failures:

---- two_claude_fixture_yields_two_never_merged_claude_installations stdout ----

thread 'two_claude_fixture_yields_two_never_merged_claude_installations' panicked at crates\vertice-core\tests\client_installations.rs:44:5:
assertion `left == right` failed: exactly two ClaudeCode installations, never merged
  left: 0
 right: 2

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### 2.4 GREEN transcript (after reverting the stub)

```
test two_claude_fixture_yields_two_never_merged_claude_installations ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

## Gate results (actually run, this environment)

| Gate | Command | Result |
|---|---|---|
| Rust fmt | `cargo fmt --all --check` | **PASS** (clean after `cargo fmt --all` normalized two files) |
| Rust lint | `cargo clippy --workspace --all-targets -- -D warnings` | **PASS** — 0 warnings |
| Rust tests | `cargo test --workspace --locked` | **PASS** — all suites green, including `installations::` (10 unit) + `client_installations` (18 integration) + all pre-existing T2–T6 suites unaffected (73 lib unit tests total, 22/18/14/9/8/24/13/7/1 across integration files) |
| Dependency policy | `PATH="$HOME/.cargo/bin:$PATH" cargo deny check bans licenses` | **PASS** — `bans ok, licenses ok` (two pre-existing "license was not encountered" warnings, unrelated to this change) |
| Model diff | `git diff --exit-code -- crates/vertice-core/src/model` | **PASS** — exit 0, no diff |
| Bindings diff | `git diff --exit-code -- frontend/src/bindings` | **PASS** — exit 0, no diff (pre-existing LF/CRLF warnings from git, not content changes; those files were already modified in the working tree before this session per the initial git status, unrelated to T7) |
| `roots.rs` diff | `git diff --exit-code -- crates/vertice-core/src/roots.rs` | **PASS** — exit 0, no diff |
| Dependency files diff | `git diff --exit-code -- Cargo.toml Cargo.lock deny.toml` | **PASS** — exit 0, no diff |
| Read-only grep | `grep -nE "File::create\|OpenOptions::write\|fs::write\|create_dir\|remove_" installations.rs client_installations.rs` | **PASS** — no matches |
| Seam grep | `grep -nE "dirs::\|directories::\|std::env::\|regex\|Regex" installations.rs` | **PASS** — only a doc-comment mentions "regex" in prose, no import |
| Privacy check | `grep -n "^pub " installations.rs` | **PASS** — exactly `InstallationScan`, `HostPlatform`, `scan`, `scan_for` |
| Frontend regression | `npm run lint && npm run check && npm run test && npm run build` (from `frontend/`) | **PASS** — lint clean, svelte-check 0 errors/warnings, 2 vitest tests pass, build succeeds |

All gates above were **actually executed** with the Bash tool in this session (Windows host, cargo 1.97.1, matching `rust-toolchain.toml`). Only the Windows CI leg was exercised locally; the Linux/macOS legs of `.github/workflows/ci.yml`'s matrix were not run in this session (no cross-compilation available) — per design §5.2, the Windows probe table is exercised identically on all three legs via `scan_for(home, HostPlatform::Windows)`, so this is expected to be leg-independent, but it is unverified until CI actually runs.

## Deviations from design

1. **Fixture completeness beyond the literal design §10 bullet text, to satisfy the same bullets' own "0 issues" pin.** `two-claude/` and `desktop-two-versions/` are described in design §10/tasks.md with only the Claude Code npm+desktop (or desktop-only) content, but both bullets also explicitly state the expected result is "0 issues". Since `scan_for` always probes all three slots, an absent third slot would emit a `Warning` and violate the stated "0 issues". I added a healthy OpenCode npm install (and, for `desktop-two-versions/`, also a healthy Claude Code npm install) to both fixtures so the literal "0 issues" assertion holds. Tests assert the CA-7-relevant subset (`ClientKind::ClaudeCode` filtered, or version-`"1.0.0"`/`"2.0.0"` filtered) rather than the total installation count, so the CA-7 pin itself is unaffected — this is an addition, not a substitution.
2. **PR-1/PR-2 stub split was not implemented as two separate landed states.** Design §12 and tasks.md describe `scan_for` initially returning an empty scan (PR 1) with `resolve` landing later (PR 2). Because the user's resolved delivery decision collapses both work units into one PR (`size:exception`), I implemented `resolve` and the taxonomy directly rather than committing an intermediate empty-scan stub as a separate state. To still produce genuine evidence for tasks.md's non-negotiable 2.3/2.4 checkpoint, I temporarily reverted `scan_for`'s body to a no-op, ran the `two-claude` integration test to observe it fail (transcript above), then restored the real implementation and re-ran to observe it pass. This satisfies the checkpoint's intent (RED must be observed before GREEN) even though the PR boundary itself is not a literal two-commit split in this session's working tree.
3. **`package-json-unreadable/` and `no-version-key/`-family fixtures were built as "all three slots present, only the target slot broken/edge-cased"** rather than single-slot homes, specifically so the fixtures whose task description requires "zero Warning" (`package-json-unreadable/`, `npm-dir-no-package-json/`) can actually assert that. The other single-edge fixtures (`no-version-key/`, `version-not-a-string/`, `package-json-empty/`) follow the same pattern for consistency, though their task text does not strictly require "zero Warning" — this is a superset of the required coverage, not a narrowing.

No other deviations. `roots::probe` was not touched (private, `exists` is a 3-line local duplicate per design §5.3). No new dependency was added. No `escalate` function was introduced (per design §8/T6D §5.6 precedent, every `ScanIssue` is constructed at the point where caller context is already available).

## Issues found

None outside the fixture-completeness ambiguity noted in Deviation 1 above (design's literal fixture bullet content vs. its own "0 issues" assertion), which I resolved by adding fixture content rather than by weakening the test assertion.

## Workload / PR Boundary

- Mode: single PR with `size:exception` (resolved 2026-08-19, per tasks.md's Review Workload Forecast)
- Current work unit: both Unit 1 and Unit 2 (collapsed into one PR's commit-group boundaries, per the resolved delivery decision)
- Boundary: this apply batch starts from Phase 0 (already marked done pre-apply) and ends with Phase 3's full gate suite, i.e. the entire change
- Estimated review budget impact: forecast was ~445–685 changed lines (High risk), accepted under `size:exception` per the T6 precedent; actual diff size was not separately re-measured in this session but is expected to be in the forecast's range given the fixture tree size (30 fixture files + 2 source files + 1 test file + `.gitattributes`)

## Remaining tasks

None — 39/39 complete.
