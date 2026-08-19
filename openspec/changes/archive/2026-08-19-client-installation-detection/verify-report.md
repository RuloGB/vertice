## Verification Report

**Change**: 2026-08-19-client-installation-detection
**Version**: N/A (no spec version field)
**Mode**: Strict TDD

### Completeness
| Metric | Value |
|--------|-------|
| Tasks total | 39 |
| Tasks complete | 39 |
| Tasks incomplete | 0 |

All 39 checkboxes were independently checked against the codebase (not trusted from the `[x]` marks alone): fixtures exist on disk for every task 1.7-1.11 case, `installations.rs`/`client_installations.rs` exist and contain the described types/tests, and every gate task (1.14, 3.1-3.11) was re-run in this session rather than accepted from `apply-progress.md`.

### Build & Tests Execution
**Build**: PASS (`cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` - both clean, re-run in this session)

**Tests**: PASS - 179 tests across the workspace, 0 failed, 0 skipped
```text
cargo test --workspace --locked
  vertice-core lib:              73 passed  (includes 10 new installations:: unit tests)
  client_installations.rs:       18 passed  (new - one per spec/CA requirement, see matrix below)
  frontmatter_reader.rs:         14 passed  (unaffected - regression)
  jsonc_behavior.rs:              9 passed  (unaffected - regression)
  model_contract.rs:              8 passed  (unaffected - regression)
  opencode_agent_scanner.rs:     24 passed  (unaffected - regression)
  skill_scanner.rs:              13 passed  (unaffected - regression)
  yaml_behavior.rs:               7 passed  (unaffected - regression)
  yaml_seam_invariant.rs:         1 passed  (unaffected - regression)
  vertice-app unit + doctests:    included, 0 failed
```
All commands were executed directly with the Bash tool in this session (cargo 1.97.1, Windows host) - not accepted from `apply-progress.md`'s transcript.

**Coverage**: Not available - no coverage tool configured for this workspace (`cargo tarpaulin`/`llvm-cov` not present). Reported as skipped, not a failure.

**Dependency policy gate**: PASS - `PATH="$HOME/.cargo/bin:$PATH" cargo deny check bans licenses` -> `bans ok, licenses ok` (two pre-existing `license-not-encountered` warnings for BSD-2-Clause/ISC, unrelated to this change, matching apply-progress's own note).

**Frontend regression**: PASS - `npm run lint` (clean), `npm run check` (169 files, 0 errors/warnings), `npm run test` (2/2 passed), `npm run build` (succeeded). All four re-run directly in this session.

**Platform coverage caveat**: only the Windows CI leg was exercised (Windows host, no cross-compilation available in this session). The Linux and macOS legs of `.github/workflows/ci.yml`'s matrix were **not run** and are **unverified**, not assumed passing. Per design section 5.2 the Windows probe table is exercised identically on all three legs via `scan_for(home, HostPlatform::Windows)` (confirmed by direct source read: `client_installations.rs` calls `scan_for(&home, HostPlatform::Windows)` everywhere except the two seam tests that branch on `cfg!(target_os = "windows")`), so the design intent is sound, but CI has not actually confirmed it in this session.


### Spec Compliance Matrix
| Requirement | Scenario | Test | Result |
|-------------|----------|------|--------|
| Windows Probe Paths Are Hardcoded, Never OS-Convention-Derived | All three probe paths resolve under the passed-in home | `installations.rs > windows_install_probes_paths_are_home_plus_hardcoded_segments` + `windows_install_probes_structure_is_identical_for_two_different_homes` | COMPLIANT |
| Claude Code npm And Desktop Are Never Merged | Both Claude Code installs, different versions, two entries | `client_installations.rs > two_claude_fixture_yields_two_never_merged_claude_installations` | COMPLIANT |
| Claude Code npm And Desktop Are Never Merged | OpenCode present alongside Claude Code | `client_installations.rs > opencode_npm_fixture_yields_one_opencode_installation` + co-occurrence in `two_claude_fixture_...` | COMPLIANT |
| Version Is Extracted From The Correct Source Per Slot | npm version from package.json | `installations.rs > version_string_present_yields_some` + `client_installations.rs > opencode_npm_fixture_...` | COMPLIANT |
| Version Is Extracted From The Correct Source Per Slot | Desktop version from directory name | `client_installations.rs > desktop_two_versions_fixture_yields_two_installations_never_merged` | COMPLIANT |
| An Absent Slot Is Reported As An Explicit "Not Detected" Signal | Absent slot yields no installation + named signal | `client_installations.rs > nothing_fixture_yields_zero_installations_and_three_warnings_never_an_error` | COMPLIANT |
| An Absent Slot Is Reported As An Explicit "Not Detected" Signal | Not-detected distinguishable from parse-error | `nothing_fixture_...` (Warning) vs `isolation_fixture_isolates_...` (Error) - distinct severity AND distinct reason shape | COMPLIANT |
| A Malformed Or Unreadable package.json Produces An Error, Never A Phantom Installation | Malformed -> Error, no installation | `isolation_fixture_isolates_one_malformed_slot_from_the_other_two` | COMPLIANT |
| A Malformed Or Unreadable package.json Produces An Error, Never A Phantom Installation | Missing "version" -> no phantom entry | `no_version_key_fixture_yields_no_phantom_installation_and_one_error` | COMPLIANT |
| Each Desktop Version Directory Is Its Own Installation | Zero subdirectories -> no installation, one issue | `desktop_empty_fixture_yields_no_installation_and_one_error_never_a_phantom` | COMPLIANT |
| Each Desktop Version Directory Is Its Own Installation | Two subdirectories -> two installations, never merged | `desktop_two_versions_fixture_yields_two_installations_never_merged` | COMPLIANT |
| Each Slot Fails Independently | One malformed slot does not block the other two | `isolation_fixture_isolates_one_malformed_slot_from_the_other_two` | COMPLIANT |
| Only Path Resolution Is Platform-Specific | No cfg(target_os) outside the dispatch point | Static: grep confirms `cfg!(target_os=...)` appears only in `HostPlatform::current()`; no covering runtime test asserts absence-of-cfg | PARTIAL - see note below |
| Scanner Performs No Writes | Full scan leaves fixture tree unchanged | `full_scan_leaves_the_reference_fixture_tree_unchanged` | COMPLIANT |
| Installation And Issue Ordering Is Deterministic | Two runs, identical order | `two_runs_over_reference_and_desktop_two_versions_are_byte_identical` | COMPLIANT |
| Every Case Is Traceable To A Repository Fixture | Fixture set covers every requirement | 12 fixture homes present; 18 integration tests, one per case; enumerated in design section 10 and cross-checked in this session | COMPLIANT |

**Compliance summary**: 14/15 requirement rows COMPLIANT, 1 PARTIAL (structural-only, not a functional gap - see Issues).

**Note on "Only Path Resolution Is Platform-Specific"**: the spec's own scenario ("the code is inspected") is explicitly a static-inspection scenario, not a runtime one - so the PARTIAL label is about test-suite rigor, not about behavioral correctness. Source inspection (this session, direct read of `installations.rs`) confirms zero `cfg(target_os)` branches outside `HostPlatform::current()`. Flagged as a SUGGESTION, not CRITICAL, because the spec's own scenario wording matches what was actually done (inspection), not a test assertion.


### Correctness (Static Evidence)
| Requirement | Status | Notes |
|------------|--------|-------|
| CA-7 (three independent Claude Code installations, never merged) | Implemented | `resolve_desktop` builds N `ClientInstallation` per candidate subdirectory; no `BTreeMap<ClientKind,_>` or any dedup exists in the module (grepped) |
| CA-11 (absent slot named, distinguishable, never silent) | Implemented | `resolve_npm`/`resolve_desktop` push `Warning` with `path: Some(probe.path)` and reason `"{Client} ({kind}) not detected"` before returning; all 3 probes always run even after a failure |
| CA-16 (read-only) | Implemented | grep for `File::create`, `OpenOptions`, `fs::write`, `create_dir`, `remove_` across `installations.rs` + `client_installations.rs` -> zero matches; only `symlink_metadata`, `read_to_string`, `read_dir` touch disk |
| CA-17 (fixture-based, new tree, machine-independent) | Implemented | all 18 tests build paths via `env!("CARGO_MANIFEST_DIR")`; no `std::env::var` outside that macro; no `claude`/`opencode` subprocess invocation anywhere in the test file |
| Design section 6 (N desktop installations, byte-wise sort, never merged, zero-candidates is Error) | Implemented | `resolve_desktop` collects `(OsString, PathBuf)` pairs, sorts via `as_encoded_bytes().cmp(...)` (byte-wise, not locale), empty -> one `Error`, N>=1 -> N installations with no anomaly issue |
| Design section 5.2 (`cfg!` expression, not attribute; `HostPlatform::current()` matches compiled target) | Implemented | `if cfg!(target_os = "windows") {...} else {...}` in `HostPlatform::current`; anchored by `host_platform_current_uses_cfg_expression_matching_the_compiled_target`, which independently recomputes the same `cfg!` expression and asserts equality |
| Model/binding/roots/dependency invariants | Implemented | `git diff --exit-code` clean for `crates/vertice-core/src/model`, `crates/vertice-core/src/roots.rs`, `Cargo.toml`/`Cargo.lock`/`deny.toml`; re-run in this session, not trusted from apply-progress |
| `.gitattributes` non-UTF-8 fixture | Implemented | `package-json-unreadable/.../package.json` registered as `binary` on `.gitattributes` line 6; hex-dump in this session confirms genuine non-UTF-8 bytes (0xff 0xfe mid-string), not merely claimed |

### Coherence (Design)
| Decision | Followed? | Notes |
|----------|-----------|-------|
| Section 2 - absence via `ScanIssue`, no model edit, no `ClientDetectionStatus` | Yes | `model/installation.rs` unmodified; `InstallationScan`/`InstallProbe`/`HostPlatform` derive no `TS`/`Serialize` (grepped, confirmed) |
| Section 3 - `InstallationScan` has no `roots` field | Yes | struct has exactly `installations`, `issues` |
| Section 4 - `Warning` severity + exact reason grammar | Yes | `"{Client} ({kind}) not detected"` reproduced verbatim in both `resolve_npm` and `resolve_desktop`; verified against fixture test assertions |
| Section 5.1 - `[InstallProbe; 3]` fixed array, private types | Yes | `windows_install_probes` returns `[InstallProbe; 3]`; only `InstallationScan`, `HostPlatform`, `scan`, `scan_for` are `pub` (grep `^pub ` confirms exactly those 4) |
| Section 5.2 - single `cfg!` dispatch point | Yes | see Correctness table above |
| Section 5.3 - `roots.rs` untouched, local `exists` helper | Yes | `git diff` clean on `roots.rs`; local `fn exists` mirrors `NotFound => false` semantics |
| Section 6 - verbatim directory-name version, byte-wise sort, N-never-merged | Yes | confirmed above |
| Section 7 - determinism (fixed probe order + sorted candidates) | Yes | `two_runs_over_reference_and_desktop_two_versions_are_byte_identical` passes |
| Section 8 - `ScanIssue` taxonomy (collapsed version-shape rows, distinct read-vs-parse errors) | Yes | `no-version-key`/`version-not-a-string`/`package-json-empty` fixtures all assert the identical collapsed reason string; `package-json-unreadable` asserts a distinct `"could not read package.json:"` prefix |
| Section 10 - fixture architecture (12 homes, non-negotiable fixtures, `.gitkeep` tripwire) | Yes | all 12 homes present on disk; `desktop_empty_fixture_directory_still_exists_on_disk` tripwire test present and passing |
| Section 11 - per-OS paths, hardcoded segments only | Yes | confirmed by unit tests and by grep (no `dirs::`/`directories::`/`std::env::` in `installations.rs`) |


### TDD Compliance
| Check | Result | Details |
|-------|--------|---------|
| TDD Evidence reported | Yes | `apply-progress.md` has a full "TDD Cycle Evidence" table |
| All tasks have tests | Yes | 39/39 tasks; test files exist for every RED/GREEN pair claimed |
| RED confirmed (tests exist) | Yes | all unit and integration test functions listed in the evidence table exist verbatim in `installations.rs`/`client_installations.rs` |
| GREEN confirmed (tests pass) | Yes | all 18 integration + 10 (of 73 lib) unit tests pass on execution in this session |
| Triangulation adequate | Partial | see Assertion Quality below - CA-7/CA-11 pins are well-triangulated; some edge-case fixtures triangulate the same collapsed-reason assertion three times, which is intentional per design section 8 |
| Safety Net for modified files | Yes | `lib.rs` (the one modified file) had its existing content re-verified unaffected by the full `cargo test --workspace` run, which stayed green including all pre-existing suites |

**TDD Compliance**: 6/6 checks passed (one Partial, explained as design-intentional, not a defect).

**On the RED evidence gap** (apply-progress deviation 2): tasks 1.1-1.4 and 2.1-2.2's unit tests were authored in the same write as their implementations, with no captured compile-fail transcript. This is a real, honestly-disclosed deviation from strict per-micro-task RED-before-GREEN. Downgraded from CRITICAL to WARNING here because (a) the load-bearing, spec-mandated checkpoint - `two-claude`/CA-7 (tasks 2.3-2.4) - was genuinely executed as RED-then-GREEN with a captured transcript that this session independently confirmed is consistent with the current code, and (b) every unit test lacking a captured RED transcript was re-run in this session and passes, so no regression risk survives - only the historical audit trail is thinner than ideal.

---

### Assertion Quality
No tautologies, no assertion-free tests, and no ghost loops (over possibly-empty collections without a companion guard) were found. Two items worth flagging at SUGGESTION level:

| File | Line | Assertion | Issue | Severity |
|------|------|-----------|-------|----------|
| `client_installations.rs` | ~383-396 (`no_installation_ever_carries_an_empty_version`) | loop over `scan.installations` asserting `.all(...)` | technically a loop over a collection that could be empty in principle; would vacuously pass if a regression made `scan.installations` empty for all 5 cases | SUGGESTION - not CRITICAL, because every one of the 5 cases already has a separate, stronger test elsewhere in the file asserting non-zero installation counts for that same fixture |
| `installations.rs` | ~456-472 (`host_platform_current_uses_cfg_expression_matching_the_compiled_target`) | recomputes the same `cfg!` expression the production code uses, then asserts equality | mildly self-referential - would not catch `HostPlatform::current()` being miswired to always return one variant | SUGGESTION - the test still legitimately proves the "expression not attribute" compile-everywhere property since it runs identically on every target |

**Assertion quality**: 0 CRITICAL, 0 WARNING, 2 SUGGESTION.

---

### Test Layer Distribution
| Layer | Tests | Files | Tools |
|-------|-------|-------|-------|
| Unit | 10 | 1 (`installations.rs`, `#[cfg(test)]` module) | rustc/cargo test |
| Integration | 18 | 1 (`client_installations.rs`) | rustc/cargo test, fixture-driven |
| E2E | 0 | 0 | not applicable - core library only, no IPC/UI surface added |
| **Total** | **28** | **2** | |

### Changed File Coverage
Coverage tooling (`cargo-tarpaulin`/`cargo-llvm-cov`) is not installed in this environment. Reported as: **Coverage analysis skipped - no coverage tool detected.** Not a failure.

### Quality Metrics
**Linter (`clippy -D warnings`)**: PASS - 0 warnings across the workspace (`--all-targets`, so test code is linted too).
**Formatter (`cargo fmt --check`)**: PASS - clean, no diff.
**Frontend type checker (`svelte-check`)**: PASS - 0 errors/warnings across 169 files (regression only; this change adds no frontend code).


---

### Issues Found

**CRITICAL**: None.

**WARNING**:
1. **RED-evidence gap for 5 of 12 unit-test groups** (apply-progress's own disclosed deviation 2, tasks 1.1-1.4, 2.1-2.2): tests and implementation were authored in one write, with no captured compile-fail transcript proving the tests would have failed before the implementation existed. Mitigated by: the one load-bearing checkpoint the tasks list explicitly gated (2.3/2.4, CA-7) was genuinely RED-then-GREEN with a captured transcript, and every test in this group passes on independent re-execution in this session. Recommend, for future changes: capture the compile-fail transcript even for "obvious" unit tests, since `strict_tdd: true` is meant to be auditable, not merely followed in spirit.
2. **Fixture completeness deviation** (apply-progress deviations 1 and 3): `two-claude/`, `desktop-two-versions/`, and several single-edge-case fixtures (`no-version-key/`, `version-not-a-string/`, `package-json-empty/`, `package-json-unreadable/`, `npm-dir-no-package-json/`) were built with all three slots populated, rather than a single isolated slot as design section 10's fixture bullets literally describe. This session independently verified (by reading every fixture's `package.json`/directory contents) that in every case the extra populated slots are genuinely healthy and the target broken/edge slot is genuinely the only source of the asserted issue - so the "0 issues" and "zero Warning" assertions are **not vacuous**: they exercise real multi-slot resolution, not an accidentally-trivial single-slot scan. This is legitimate superset coverage, not a weakened test, but it is flagged WARNING rather than accepted silently because it is a deviation from the design's literal fixture description that a future reader of design.md alone (without apply-progress.md) would not expect.

**SUGGESTION**:
1. No runtime test directly asserts "the scanner's version-extraction and assembly code contain no cfg(target_os) branch" - the spec's own scenario is a static-inspection scenario, and that inspection was done by this verify session, not automated. A grep-based CI check (or a doctest-style assertion) would make this self-verifying on every future change rather than requiring a human/agent re-read.
2. Two mildly self-referential/loop-shaped assertions noted above (Assertion Quality table) - not defects, but worth strengthening if the file is touched again.
3. `.gitattributes` binary registration and non-UTF-8 fixture bytes are correct today; no automated check pins this (a bit-rot risk if someone "cleans up" the fixture with a text editor that normalizes it). Precedent (`frontmatter/non-utf8-content/SKILL.md`) has the same unpinned risk, so this is consistent with existing project practice, not a new gap introduced by T7.
4. Pre-existing `frontend/src/bindings/*.ts` LF/CRLF working-tree noise (visible in `git status`) predates this change and was not produced by T7 - confirmed via `git diff --stat`, which shows only Git's line-ending normalization warnings and zero content lines changed. This is pre-PR housekeeping, not a T7 defect, and does not block archive.
5. Linux/macOS CI legs were not exercised in this session (no cross-compilation available) - unverified, not assumed passing. Design section 5.2's reasoning for why the Windows-only local run is representative was independently confirmed by source inspection (all fixture calls use `scan_for(home, HostPlatform::Windows)` uniformly), but actual CI execution on those two legs remains outstanding until the PR runs through GitHub Actions.

### Verdict
**PASS WITH WARNINGS**
39/39 tasks complete and independently re-verified; all Rust and frontend gates re-run in this session and green; 18/18 new integration tests and 10/10 new unit tests pass; every CA-7/CA-11/CA-16/CA-17 pin and every design decision (sections 2-11) was checked against the actual code, not trusted from apply-progress.md. The two WARNINGs (RED-evidence audit-trail gap on non-checkpoint tasks; fixture-completeness deviation from design's literal bullet text) are both real deviations from the strictest process reading but do not weaken any test's actual behavioral guarantee - independently confirmed by reading every affected fixture's raw content in this session. No CRITICAL issues block archive.

