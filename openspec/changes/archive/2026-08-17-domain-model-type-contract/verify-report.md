# Verification Report: Domain Model and Type Contract (T2)

Verified 2026-08-17. Mode: full artifacts (proposal, specs x2, design, tasks, apply-progress). Working tree: uncommitted on main, nothing pushed or branched. This report does not commit, push, or branch.

## Verdict: PASS WITH WARNINGS

At verification time: 0 CRITICAL, 3 WARNING, 2 SUGGESTION.

**After the post-verify follow-up (2026-08-17): 0 CRITICAL, 1 WARNING (W3, clippy not runnable locally -- an environment constraint, not a defect), 2 SUGGESTION.** W1 and W2 were closed by adding the two missing tests; see section 6.

## 1. Task Completeness

All 26 tasks in tasks.md are marked [x] (1.1-1.4, 2.1-2.7, 3.1-3.3, 4.1-4.2, 5.1-5.4, 6.1-6.3, 7.1-7.3). Re-inspected against the actual code state -- no task claims contradicted by the code (see per-task evidence below). Task completion: CONFIRMED.

## 2. Full Verification Matrix -- Re-run by verify, real output

| Command | Result | Notes |
|---|---|---|
| cargo fmt --all --check | PASS (exit 0, no diff) | Ran from MSVC 1.97.1 toolchain; rustfmt was in fact present there too (the environment note said GNU-only, not accurate for this machine, but does not change the result). |
| cargo test --workspace --locked | PASS -- 33/33 (21 lib incl. 15 export tests + 5 identity + 1 yaml seam; 6 model_contract.rs; 6 yaml_behavior.rs; 0 doc-tests) | Matches apply's claimed count exactly. |
| cargo build --release | PASS -- Finished release [optimized] in about 51s | Within budget, ran to completion, not skipped. |
| cargo deny check bans | PASS -- bans ok | cargo tree -p vertice-core -i tauri errors "did not match any packages" -- confirms zero tauri edges in vertice-core's dependency graph. Core Purity Invariant holds. |
| cargo deny check licenses | PASS -- licenses ok (2 pre-existing unrelated license-not-encountered warnings for BSD-2-Clause/ISC, same as apply reported) | Zero delta from ts-rs/unicode-normalization, as apply claimed. |
| cargo clippy --workspace --all-targets -- -D warnings | NOT VERIFIABLE LOCALLY -- MSVC toolchain lacks the clippy component; GNU toolchain fails with dlltool.exe: program not found | Per known environment constraint. Covered by CI (rust job runs it on all 3 OSes). Apply's claim of 0 warnings could not be independently re-run; treated as unverified, not confirmed pass. |
| npm run lint (frontend) | PASS -- clean, no output | |
| npm run check (frontend) | PASS -- 168 FILES 0 ERRORS 0 WARNINGS | |
| npm run test (frontend) | PASS -- 1 file, 2 tests | |
| RUSTUP_TOOLCHAIN=1.88 cargo check --workspace --locked --all-targets (MSRV) | Not re-run by verify (already re-confirmed by the orchestrator per task instructions); apply's evidence accepted | Low risk -- deterministic given locked Cargo.lock. |

## 3. Spec Compliance Matrix -- specs/domain-model/spec.md

| Requirement | Scenario | Covering evidence | Status |
|---|---|---|---|
| Component Identity Is Deterministic | Case variants collapse | identity.rs::case_variants_collapse_to_one_identity | PASS (runtime) |
| | Same pair equal | identity.rs::same_kind_and_name_always_yield_equal_ids | PASS (runtime) |
| | Different kind differs | identity.rs::different_kind_same_name_yields_different_identity | PASS (runtime) |
| Location Path Is Optional and Distinguishable | Pathless is representable | model_contract.rs::pathless_and_present_path_locations_are_distinguishable | PASS (runtime) |
| | Present/absent distinguishable | same test (assert_ne!, independent state checked) | PASS (runtime) |
| Component Holds Multiple Locations As One Entity | One component, multiple locations | model_contract.rs::one_component_holds_multiple_locations_under_one_shared_id | PASS (runtime) |
| Scope Is Always Populated | Scope set on construction | model_contract.rs::scope_is_explicitly_populated_on_construction | PASS (runtime) |
| | Scope enum exhaustively matchable | tests/model_contract.rs::scope_is_exhaustively_matchable_without_a_wildcard_arm -- a wildcard-free match over all three variants | COVERED (W1 CLOSED post-verify) |
| ComponentKind Is a Closed Enumeration | Exhaustively matchable | identity.rs::identity_prefix -- a real match ComponentKind { Skill, Agent } with no wildcard arm, compiled and exercised on every ComponentId::derive call in every passing test | PASS (compile-time proof + runtime exercise) |
| ScanIssue Severity Has Two Non-Aborting Levels | Warning accumulated | model_contract.rs::populated_scan_report_round_trips_through_json (constructs a Warning-severity issue, appends, round-trips) | PASS (runtime) |
| | Error does not abort | same test (constructs an Error-severity issue in the same report, no panic, round-trips) | PASS (runtime) |
| Empty Scan Result Is Not an Error | Empty ScanReport valid | model_contract.rs::empty_scan_report_round_trips_through_json | PASS (runtime) |
| provenance_hint Is Opaque, Not a Discriminator | Option String, not enum | Generated Component.ts: provenanceHint: string or null (not a tagged union) -- real artifact inspected directly; type signature in component.rs:31 | PASS (structural + generated-artifact evidence) |
| | None distinguishable from Some-empty-string and Some-claude-code | tests/model_contract.rs::provenance_hint_absent_empty_and_present_are_three_distinct_states -- pairwise inequality plus a serde_json assertion that None emits null and Some("") emits "" | COVERED (W2 CLOSED post-verify) |
| Rust Types Generate a Matching TS Contract | Struct exports a binding | 15 export_bindings_* tests, all passing; Component.ts inspected directly, field names match | PASS (runtime) |
| | Optional path maps to nullable string | Location.ts inspected directly: path: string or null | PASS (runtime + direct inspection) |

provenance_hint five-surface reconciliation (post-apply fix) -- all agree:
1. specs/domain-model/spec.md: requirement mandates Option String, forbids empty-string sentinel -- CONFIRMED.
2. component.rs:31: pub provenance_hint: Option String -- CONFIRMED.
3. tests/model_contract.rs: all four construction sites use None or Some("claude-code".to_string()), zero String::new() remaining -- CONFIRMED (grep, zero matches for the old sentinel pattern).
4. Component.ts: provenanceHint: string or null -- CONFIRMED.
5. design.md section 2: already showed Option String, needed no change -- CONFIRMED.

## 4. ci-quality-gates Spec vs .github/workflows/ci.yml -- agreement confirmed

The delta spec's hardened requirement (MUST detect a newly generated, never-committed binding file; register regenerated files with the index first via git add --intent-to-add) and the actual ci.yml step:

```yaml
- name: Generated contract in sync
  run: |
    git add --intent-to-add -- frontend/src/bindings
    git diff --exit-code -- frontend/src/bindings
```

match exactly. Placement (same quality job, after cargo deny, ubuntu-only) also matches the spec's "MUST run in the same job... not as a new matrix leg" clause. CONFIRMED.

Re-demonstrated the bare-git-diff failure mode this hardening fixes: with the current untracked frontend/src/bindings/, a plain git diff --exit-code -- frontend/src/bindings exits 0 (false pass) -- confirmed live. This is the exact silent-pass bug the delta spec calls out.

## 5. T2 Acceptance Criteria (plan-desarrollo-poc.md lines 82-86)

| # | Criterion | Evidence | Status |
|---|---|---|---|
| 1 | Model admits a component without a disk path, distinguishable from one with a path | model_contract.rs::pathless_and_present_path_locations_are_distinguishable | PASS |
| 2 | Model admits a component with N locations without duplicating the entity | model_contract.rs::one_component_holds_multiple_locations_under_one_shared_id | PASS |
| 3 | scope field exists and is populated, PoC emits only User | model_contract.rs::scope_is_explicitly_populated_on_construction; grep confirms only Scope::User is constructed anywhere in crates/ | PASS |
| 4 | A Rust type change that breaks the contract fails CI/compilation, not runtime | Re-demonstrated live (Section 12) with the HARDENED gate -- mutated ClientInstallation.version to client_version, regenerated, hardened gate exited 1 with a minimal single-file diff | PASS |

## 6. Core Purity Invariant

cargo deny check bans: PASS, bans ok. cargo tree -p vertice-core -i tauri: "did not match any packages" -- vertice-core has zero tauri/tauri-* edges in its dependency graph.

## 7. Zero-Disk-I/O Invariant

Grepped crates/vertice-core/src/model/ for std::fs, std::io, std::env, SystemTime, Instant, File::, OpenOptions: the only hits are in mod.rs's doc comment, which lists them as forbidden imports (documentation, not usage) and in error.rs's doc comment referencing std::io::Error as a foreign type that must never be wrapped. Zero actual usage. Invariant holds.

## 8. Read-Only Invariant (rules.apply)

Grepped all of crates/ for OpenOptions, File::create, fs::write, fs::remove, std::fs: zero hits outside the same doc-comment string in mod.rs. No write operation of any kind exists in the new code. Invariant holds.

## 9. Numeric-Width Contract

Grepped crates/vertice-core/src/model/ for u64/i64: zero matches. ScanReport.duration_ms: u32 is the only numeric field in the model; it maps to TS number, not bigint. Contract holds.

## 10. ts-rs Default Features

Cargo.toml (workspace) and crates/vertice-core/Cargo.toml: neither declares default-features = false for ts-rs. Grepped both files for default-features: zero matches, meaning defaults (including serde-compat) are ON. #[serde(rename_all = "camelCase")] renames are confirmed reaching the generated .ts (e.g. provenanceHint, not provenance_hint, in Component.ts). Contract holds.

## 11. Identity Does Not Incorporate Content Hashing

identity.rs inspected in full: ComponentId::derive composes "{kind}:{normalized_name}" from (kind, name) only -- no file content, no Location data, no hashing crate (no sha2/blake3/md5 in Cargo.toml). The one #[derive(Hash)] on ComponentId is the standard Rust trait derive (for HashMap/HashSet key usage), unrelated to content hashing. Invariant holds.

## 12. Task 7.1 -- CA-4 Negative-Path Re-Verification (re-done independently against the HARDENED gate)

Apply's original check used the old, un-hardened gate. Verify re-did it against the gate as it exists now:

1. Staged the current (correct) frontend/src/bindings/*.ts with git add to simulate a committed baseline (since nothing in this repo is actually committed yet).
2. Confirmed the hardened gate (git add --intent-to-add + git diff --exit-code) is clean against that baseline: exit 0.
3. Mutated ClientInstallation.version to client_version in both installation.rs and tests/model_contract.rs.
4. Ran cargo test -p vertice-core --locked -- compiled and regenerated ClientInstallation.ts with clientVersion: string.
5. Ran the exact hardened gate: git add --intent-to-add -- frontend/src/bindings then git diff --exit-code -- frontend/src/bindings -- exit 1, single-file diff on ClientInstallation.ts only (version to clientVersion).
6. Reverted both Rust files exactly (byte-for-byte, via saved backups), touched the file to force a recompile, and re-ran cargo test -p vertice-core --locked to regenerate the binding back to its original content.
7. Re-ran the hardened gate: exit 0, clean.
8. Unstaged the simulated baseline (git restore --staged -- frontend/src/bindings).
9. Full cargo test --workspace --locked re-run after revert: 33/33 passing, matching the pre-check baseline.
10. git status --short after cleanup matches the pre-check baseline exactly (verified against the status captured before the check began).

Result: CONFIRMED, and this run is more meaningful than apply's original because it exercises the hardened gate, not the superseded bare-git-diff form.

## 13. Design/Code Reconciliation Item (flagged for archive, not blocking)

design.md section 5 states export_to = "../../frontend/src/bindings/". All 15 #[ts(export, export_to = "...")] attributes in the actual code use "../../../frontend/src/bindings/" (three levels up, not two). Apply documented this as an empirically-discovered, reproducible off-by-one and corrected the code; design.md itself was never updated. This is a known, already-flagged gap -- carry it into archive as a design-doc correction, not a re-open of the decision.

## Issues

### CRITICAL

None.

### WARNING

**Status update (post-verify, 2026-08-17): W1 and W2 are CLOSED.** Two tests were added to `crates/vertice-core/tests/model_contract.rs` and the contract suite now runs 8 tests (was 6); the full workspace suite is 35/35. The original findings are retained verbatim below as the record of what verify caught.

W1 -- Scope "exhaustively matchable" scenario has zero covering evidence. The spec's scenario requires a match over a Scope value covering User, Project, Local with no wildcard arm to compile. No such match statement exists anywhere in crates/vertice-core -- Scope is only ever constructed (scope: Scope::User), never matched. Unlike ComponentKind (which has a real exhaustive match in identity_prefix), this scenario's claim is currently unverified by any code or test. Low risk (the enum genuinely has no #[non_exhaustive], so a future exhaustive match would compile), but as written, nothing in the codebase demonstrates it. Recommend a trivial one-line test or doc-example match before or shortly after archive.

W2 -- provenance_hint "None vs Some-empty-string vs Some-claude-code distinguishable" scenario is still untested at runtime, even after the Option String fix. All four construction sites in tests/model_contract.rs use either None or Some("claude-code".to_string()); none constructs Some(String::new()). The type-level guarantee (Option String equality is structurally sound) makes this a low-risk gap, but per the spec's own literal scenario text and this project's "test proves scenario compliance" rule, it remains UNTESTED. apply-progress.md claims this is "RESOLVED... no longer an open reconciliation item" -- that statement is about the type being corrected (true), not about the scenario coverage (still not true). Recommend a 3-line test before archive asserting None, Some(String::new()), and Some("claude-code".into()) are pairwise distinct.

W3 -- cargo clippy --workspace --all-targets -- -D warnings could not be independently re-run. MSVC toolchain lacks the clippy component; GNU toolchain fails with dlltool.exe: program not found (documented, pre-existing environment limitation, not a code defect). Apply's claim of "0 warnings" is therefore unverified by this pass -- it will be enforced by CI's rust job on all three OS legs before merge, so this does not block archive, but it is not independently confirmed either.

### SUGGESTION

S1 -- design.md section 5's export_to path is stale ("../../frontend/src/bindings/" vs the actual "../../../frontend/src/bindings/" used in all 15 attributes). Recommend a one-line correction to design.md at archive time, plus (per apply-progress.md's own recommendation) a short doc-comment in model/mod.rs about the off-by-one so future contributors adding a 9th type don't rediscover it.

S2 -- ClientKind/SearchRootKind variant sets are explicitly provisional (design.md section 13 already flags ClientKind). No action needed now; just don't let T4-T7 treat the current two-variant sets as final without revisiting.

## Artifacts Verified Against Code (not just apply's claims)

- crates/vertice-core/src/model/{mod,component,location,installation,report,error,identity}.rs -- read in full.
- crates/vertice-core/tests/model_contract.rs -- read in full.
- frontend/src/bindings/Component.ts, Location.ts, ClientInstallation.ts -- read directly.
- .github/workflows/ci.yml -- read in full.
- crates/vertice-core/Cargo.toml, root Cargo.toml -- read in full.
- frontend/eslint.config.js, frontend/package.json -- read in full.

## Commands Run (this session, real output captured above)

cargo fmt --all --check, cargo test --workspace --locked, cargo build --release, cargo deny check bans, cargo deny check licenses, cargo tree -p vertice-core -i tauri, npm run lint, npm run check, npm run test, plus the full CA-4 negative-path re-enactment (mutate, regenerate, hardened-gate-fails, revert, regenerate, hardened-gate-passes, cleanup) documented in Section 12.
