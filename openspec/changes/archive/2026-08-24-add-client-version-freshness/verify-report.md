# Verification Report: add-client-version-freshness

**Change**: `add-client-version-freshness` | **Mode**: full artifacts (proposal/design/tasks/specs present, openspec store) | **Branch**: `feat/client-version-freshness`, nothing committed

## Gate Results (actually executed, this session)

| Gate | Result |
|---|---|
| `cargo fmt --all --check` | PASS (no output) |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS (0 warnings) |
| `cargo test --workspace --locked` | PASS -- every suite green, `vertice-app` lib: 39 passed, 1 ignored (`freshness::fetch::tests::freshness_live_upstream_endpoints_still_match_the_documented_shape`, reason string names CA-17). Confirmed this is the only ignored test workspace-wide. |
| `PATH="$HOME/.cargo/bin:$PATH" cargo deny check bans licenses` | PASS -- `bans ok, licenses ok`. 3 informational warnings only (BSD-2-Clause/ISC unmatched allow-list entries, unused-wrapper for tauri on this invocation) -- none are failures. |
| `npm run lint` (frontend) | PASS |
| `npm run check` (frontend) | PASS -- 216 files, 0 errors, 0 warnings |
| `npm run test` (frontend) | PASS -- 11 files, 108 tests |
| `npm run build` (frontend) | PASS |

All eight gates are green, independently re-run, not trusted from apply-progress.md's own claims.

## Core Purity (verified, not trusted)

- `crates/vertice-core/Cargo.toml`: dependencies are jsonc-parser, semver, serde, serde_norway, thiserror, toml_seam (aliased toml), ts-rs, unicode-normalization, walkdir. No tauri, no reqwest, no HTTP crate.
- `cargo tree -p vertice-core -e no-dev | grep -iE "reqwest|tauri|hyper|tokio"` -> zero matches (exit 1). Confirmed structurally, not by reading deny.toml's comment.
- `crates/vertice-core/src/model/*.rs` grepped for `use std::fs`, `use std::io`, `use std::env`, `SystemTime`, `Instant` -> only one hit, and it is the doc comment in mod.rs stating the forbidden list, not a use of it. model/freshness.rs and model/slot.rs import only std::path::PathBuf (freshness.rs), serde, ts_rs. Satisfied.

## deny.toml Containment -- Reasoned, Not Assumed

The entry `{ name = "reqwest", wrappers = ["vertice-app", "tauri"] }` bans reqwest for every crate except direct dependents named vertice-app/tauri. Per cargo-deny's own documented semantics (and per this file's comment block, pinned by prior verified precedent for the tauri/tauri-build entries), wrappers only exempts direct dependents. vertice-core cannot become a direct dependent of reqwest without editing vertice-core/Cargo.toml itself and immediately failing this bans check on the next CI run -- a convenience refactor pulling reqwest in transitively would also fail, because the transitive introducer is not in the wrapper list either. This containment is real and would catch the regression it claims to catch. Confirmed by the actual cargo deny check bans licenses run above passing cleanly against the current graph, which already contains reqwest/native-tls/schannel under vertice-app only.

## Read-Only Audit Widening -- Judged, Not Just Read

crates/vertice-app/tests/read_only_audit.rs is a genuine widening:
- Was: 2 hardcoded files (commands.rs + lib.rs) checked for 16 mutation patterns.
- Now: every .rs file under crates/vertice-app/src/** (hand-rolled recursive walk), same 16 patterns, unchanged, still checked verbatim (FORBIDDEN_MUTATION_PATTERNS, lines 159-176, byte-identical list).
- #[cfg(test)] mod bodies are stripped first via strip_cfg_test_blocks (brace-depth counting, string-literal-aware) -- this is scoped to test scaffolding, not production code, and is a legitimate narrowing of the audit's subject (production surface), not a weakening of what it checks within that subject.
- freshness/cache.rs is exempted from the negative (forbidden-pattern) scan but is subjected to three positive checks instead: must reference app_data_dir, must not contain std::env:: (or " env::"), must not contain one of five literal-path markers (C:\, C:/, "/home/, "/Users/, "/etc/).

Judgment on the exemption boundary: sound in spirit, imperfect in exhaustiveness. The positive checks correctly pin the two things CA-16 actually cares about (derivation from app_data_dir(), not a literal/env path) and I independently confirmed cache.rs's production code (store_path, load, save) contains no std::env:: reference and takes app_data_dir: &Path as a parameter, resolved by the caller (commands.rs) via tauri::Manager::path(app).app_data_dir(), never inside cache.rs. However, the literal-path-marker list is a fixed set of five substrings -- it would not catch, e.g., a hardcoded D:\ or a bare /tmp without a leading quote-and-slash pattern, or a PathBuf::from("relative/literal") that never touches app_data_dir at all but also never matches any of the five markers. The test's own static_proof_is_limited: true field acknowledges this is heuristic, not a proof -- that self-awareness is appropriate, but the marker list itself is narrower than it could be (e.g., no bare D:\/E:\ check, no lowercase-drive check, no Unix path without a leading quote). WARNING, not CRITICAL: the exemption is genuinely scoped and reviewed in good faith, but a reviewer should not read "no literal path" as machine-proven -- it is grep-proven against five patterns.
- Confirmed by direct read: cache.rs writes exactly once (fs::write), whole-file, no temp-file-plus-rename, matching design section 8.
- The injected std::fs::write in upstream.rs catch claimed by apply-progress.md was NOT independently re-verified in this session (I did not inject and re-run it) -- I take apply-progress.md's word on that specific claim; everything else in this section I verified directly against the current source.

deny.toml/audit combined verdict: SATISFIED, with the one WARNING above.

## desktop-shell Spec -- CRITICAL Finding: Command Surface Exceeds The Spec's "Exactly Three"

specs/desktop-shell/spec.md, Requirement "Minimal Scan Command Surface": "The shell SHALL expose exactly three commands: scan, rescan... and a third, separate freshness command..." (line 8). design.md section 11 states: "One new command; no event, no capability change, no CSP change" (line 223) and file-changes table section 13 makes no mention of a settings-mutation command.

Actual implementation (crates/vertice-app/src/lib.rs:12-18, crates/vertice-app/tests/read_only_audit.rs:24-30): five commands -- scan, rescan, freshness, freshness_settings, set_freshness_settings. The read-only audit test was itself rewritten to assert exactly this five-command list, so the test and the code agree with each other -- but neither agrees with the spec text or the design's IPC contract surface (section 11), which both explicitly say "one new command"/"exactly three total."

This is not hidden: apply-progress.md's "Slice 3 completion by the orchestrator" section (line 261) mentions the two settings commands were added, and 8.1's binding-count correction (line 293) notes FreshnessSettings as "the sixth [binding], ... created when Slice 3 added the settings commands -- the task was written before that type existed." But this acknowledgment lives only in the progress log, not as a correction to design.md section 11/13, tasks.md (whose task list under Phase 7 never mentions a settings-mutation command), or specs/desktop-shell/spec.md (whose Requirement text still says "exactly three"). The capability grant (core:default) and CSP are genuinely unaffected -- confirmed via git status --short byte-identical, matching the spec's "adds no new capability grant" scenario, which does hold. But the command-count requirement, stated with RFC-2119 "SHALL... exactly," is violated by the shipped code, and no artifact was updated to reflect it.

Verdict: CRITICAL -- not because the two extra commands are unjustified (they close a real gap: nothing could mutate enabled/disclosure_seen without them, and the opt-out setting is a hard privacy requirement), but because the spec artifact itself was never corrected to match, and the design's IPC surface section (11) is now silently wrong. This is exactly the "design deviation exists" case the verify decision gate treats as CRITICAL when spec text is directly contradicted, not merely WARNING-level drift.

## domain-model Spec -- Related Gap

The spec's "Rust Types Generate a Matching TypeScript Contract" requirement enumerates exactly ten pre-existing types plus five new ones (Freshness, FreshnessSubject, FreshnessCheck, FreshnessReport, ClientInstallSlot) -- FreshnessSettings is not in the enumerated list, yet it exists, derives TS, and has a regenerated binding (frontend/src/bindings/FreshnessSettings.ts, confirmed present). The type itself is fine (correctly Serialize/Deserialize/TS-derived, tested for camelCase round-trip in model_contract.rs:389-402), but the spec text is stale relative to the code for the same reason as the command-count issue above -- both trace to the same undocumented Slice 3 addition. WARNING (the binding-drift CI gate would still catch a missing binding, but nothing catches a spec whose type enumeration undercounts).

## Component Freshness Spec -- Requirement-by-Requirement

| Requirement | Status | Evidence |
|---|---|---|
| Freshness is a closed three-valued verdict | Satisfied | model/freshness.rs:19-24; model_contract.rs::freshness_is_exhaustively_matchable_without_a_wildcard_arm passes |
| Comparison is total, fails closed to Unknown | Satisfied | freshness.rs::compare (core), never panics, catch-all arm returns Unknown; freshness_compare.rs 8/8 tests including both prerelease scenarios (0.150.0-rc.1 vs 0.150.0 -> Outdated; 0.151.0-rc.1 vs 0.149.1 -> UpToDate), MSIX-shaped and empty-string -> Unknown |
| Core depends on a reference-source abstraction only | Satisfied | ReferenceVersions trait, MapReferenceVersions stub in core; cargo tree above proves no HTTP crate reachable from core, so "no core test can perform network access" is structural, not disciplinary |
| No-upstream subject is permanently Unknown, never UpToDate, no request issued | Satisfied | freshness_evaluate.rs::no_upstream_slot_is_never_up_to_date; upstream.rs::claude_code_bundled_issues_no_request_by_construction (compile-time proof: no UpstreamIdentity value ever constructed for that slot) |
| Network/cache failures degrade to Unknown, never crash | Satisfied | fetch.rs -- every failure path (request failed, non-2xx, rate-limit, unparseable, oversize) returns Unavailable, never panics/errors; cache.rs::corrupt_file_is_treated_as_the_default_empty_store |
| Freshness lookups never enter the diagnostic channel | Satisfied | evaluate() (core) never constructs a ScanIssue, has no access to one; commands.rs::scan_never_produces_a_freshness_shaped_issue_and_runs_independently_of_it |
| Reference field ordering never trusted on one field alone | Satisfied | fetch.rs::parse_github_latest_release -- name then de-prefixed tag_name then Unavailable; tests for prefix-stripping, first-candidate-preferred, fallthrough, and github_raw_prefix_carrying_tag_is_never_used_as_is (nightly-build-42 case) |
| Cache is the only new write, confined to app data dir | Satisfied | cache.rs::save/store_path; audit test's positive checks (see above, with the noted marker-list limitation) |
| Enabled by default, disclosed, fully stoppable | Satisfied, one gap noted below | FreshnessStore::default().enabled == true; ClientsPage.svelte disclosure + toggle; but see "no identifying content" scenario below, NOT independently test-covered |

### Scenario-level gaps (no covering test -- stated explicitly, not inferred)

- "An outbound request carries no identifying content" (component-freshness spec, lines 163-167): NOT SATISFIED BY A TEST. fetch.rs::build_client sets only a static User-Agent: vertice/<version> header and upstream.rs::request_url() builds URLs with no query parameters -- I verified this by reading the code, and it is true -- but no test asserts the constructed request/URL/headers contain zero identifiers. build_client_succeeds_with_the_designed_timeout_budget only asserts build_client() returns Ok, not what it built. This is a genuine coverage gap on a privacy-critical requirement, not merely a nice-to-have; flagging as CRITICAL/UNTESTED per the instruction to not infer coverage from adjacent tests, even though direct code inspection finds no violation.
- inventory-ui spec, "An outdated client does not trigger the incident indicator" and "...does not affect the Home scan-status block" (lines 34-44): NOT SATISFIED BY A TEST. App.svelte never references freshness/Freshness at all (grepped, zero hits) and incidentCount() takes only Diagnostics derived from ScanIssue/roots -- so the guarantee is structurally true (there is no code path for Outdated to reach incidentCount) -- but there is no App.test.ts or HomePage test (no HomePage.test.ts file exists at all) exercising a report that is all-Outdated and asserting the incident indicator stays dark or the Home block stays healthy. CRITICAL/UNTESTED.
- tasks.md task 7.1's claimed scenario -- "incidentCount unchanged when a report is all-Outdated" -- was never actually written as a test anywhere in frontend/src/lib/scanDiagnostics.test.ts or App.test.ts (grepped both, zero hits for "outdated"/"Outdated"). The task is checked [x] but the specific assertion it describes does not exist. Ties into the same CRITICAL gap above.

## client-installation-detector Delta -- Satisfied

- ClientInstallSlot promoted to model::slot, closed, exhaustively matchable (model_contract.rs::client_install_slot_is_exhaustively_matchable_without_a_wildcard_arm).
- ClientPresence.slot added; tripwire test client_installations.rs:910 (slot_promotion_leaves_detection_output_unchanged_except_for_the_new_field) asserts byte-identical installations/issues/ordering across 15 fixture homes, plus record.label == record.slot.label(). Genuinely proves the "detection behavior unchanged" scenario, not just claims it.
- presenceFor in ClientsPage.svelte keys on record.slot, not .label -- matches the "dispatches on the discriminator" scenario directly. upstream_for(slot) (app-side) is the concrete consumer the spec anticipated, also slot-keyed.
- Pre-existing drift confirmed unworsened: ScanPage.svelte:110 still keys {#each report.clientPresence as record (record.label)} -- untouched by this change, exactly as apply-progress.md describes, out of scope.

## scan-orchestration Delta -- Satisfied

- run_scan/scan/rescan in commands.rs contain no reference to crate::freshness (grepped/read directly). CA-15 budget untouched: scan() is a one-line delegation to run_scan(), which does not call freshness::build_report.
- commands.rs::scan_never_produces_a_freshness_shaped_issue_and_runs_independently_of_it -- covers the observable half (no issue mentions "freshness"). The structural half (module can't even name crate::freshness) is real (verified by reading scan.rs's and run_scan's full bodies) but is documented as architectural rather than separately asserted, which is an honest and acceptable choice for something a test literally cannot regress on its own.

## workspace-architecture Delta -- Satisfied

- "vertice-core Stays HTTP-Free": proven above via cargo tree.
- "The Reference-Version Seam Is Owned By vertice-app": grepped crates/vertice-core/src/** for any ReferenceVersions implementor other than MapReferenceVersions -- none exists. crates/vertice-app/src/freshness/{fetch,upstream,cache,mod}.rs is the sole owner; no other vertice-app module implements the trait or names reqwest.

## frontend-i18n Delta -- Satisfied

- All clients.*/freshness.* keys present and complete in both en and es (catalogs.ts:100-120 type shape, 256-277 en values, 409-431 es values) -- verified directly, not inferred.
- Version/upstream strings render passthrough: freshness.outdated: "Update available: {latest}" interpolates the raw version string, never localizing it; ClientsPage.test.ts doesn't have a dedicated locale-switch assertion for the version string, but the interpolation mechanism itself makes mistranslation structurally impossible (the string is a template parameter, not catalog prose).

## TLS Backend Decision (design section 4) -- Open Item Confirmed Accurately Described

native-tls is chosen (crates/vertice-app/Cargo.toml comment block, documenting the measurement rationale). Confirmed accurately flagged as Windows-only-verified in both tasks.md (4.1) and apply-progress.md (Slice 2 intro and "Issues found") -- the Linux CI leg (system OpenSSL headers for native-tls/openssl-sys) remains genuinely unverified locally. Not treated as a defect, per instructions -- correctly an open item, not silently dropped.

## Badge-In-Running-App Limitation -- Confirmed Honestly Stated

apply-progress.md's "Browser verification, and its limit" section states plainly that the four badge states have not been observed in the running Tauri desktop app, only under component tests and a degraded-IPC browser session. This is accurate -- no evidence in this session contradicts it, and no test or artifact overclaims otherwise. Confirmed honest.

## Privacy -- Disable-In-Flight Behavior (the highest-value check requested)

Verified directly in ClientsPage.svelte:
- loadFreshness() assigns a monotonically incrementing lookupToken; a response is discarded if token !== lookupToken at resolution time (lines 71-81).
- toggleEnabled -> disabled path calls cancelLookup(), which increments the token (invalidating any in-flight request's eventual response) and clears freshness/lookupFailed state (lines 163-168).
- badgeFor() additionally gates on settings?.enabled as its first condition (line 100) -- so even if a stale response somehow slipped through the token guard, the badge would still render nothing while disabled. Defense in depth, genuinely two independent guards, not one.
- Covered by two tests written specifically for the user-reported defect: "re-runs the check when the setting is switched back on..." and "ignores an in-flight response that lands after the check is switched off" (ClientsPage.test.ts). Both pass. Satisfied, and the fix/regression-test pairing in apply-progress.md's "Defect found by the user" section is accurately described -- I ran the tests and they pass as claimed.

One caveat: this guard is entirely frontend-side (lookupToken, a JS variable). Nothing on the Rust side prevents an in-flight fetch_reference future from completing its HTTP call after the frontend disables the check -- the guard stops the response from being rendered/used, not the outbound request from completing once already dispatched. The spec's wording ("no further outbound request... is made") is about NEW requests, which is correctly honored (no new fetchFreshness() call fires after disable); an already-in-flight request from before the toggle is not aborted mid-flight, only its result is discarded client-side. This matches the spec's literal text (no scenario requires aborting an in-flight request), so not a defect, but worth naming precisely rather than glossing as "no further request of any kind."

## Tasks Completion

51 of 51 checked task lines are [x] except task 3.9, which is [~] (partial) and still reads, uncorrected, "cargo deny check bans licenses could not be executed here." This is stale: apply-progress.md's own later "Orchestrator gate verification" section proves cargo deny was subsequently run and passed (bans ok, licenses ok), and I independently re-ran it in this session with the same result. tasks.md itself was never updated to reflect that resolution -- a minor documentation-fidelity gap, WARNING, not blocking (the underlying gate is genuinely green).

## Summary of Issues

CRITICAL
1. desktop-shell spec's "exactly three commands" requirement is contradicted by the shipped 5-command surface (freshness_settings/set_freshness_settings undocumented in design.md/spec text, only in apply-progress.md's narrative).
2. component-freshness spec scenario "An outbound request carries no identifying content" has no covering test.
3. inventory-ui spec's two "Outdated is never an incident" scenarios have no covering test (structurally true by code inspection, not proven by a runtime test).

WARNING
1. read_only_audit.rs's cache.rs literal-path-marker exemption check uses a fixed 5-substring list that would not catch every conceivable literal-path construction (e.g., a bare D:\ or unquoted Unix path) -- sound in intent, incomplete in coverage.
2. domain-model spec's type enumeration for the TS-binding requirement does not list FreshnessSettings, which exists and ships a binding.
3. tasks.md task 3.9 is still marked [~] with stale text claiming cargo deny couldn't run, though it demonstrably did and passed later in the same apply run.

SUGGESTION
1. fetch_reference's 256 KiB size ceiling is enforced before parsing (matching spec text literally) but after the full response body is already downloaded into memory via response.bytes().await -- not a spec violation, but a hardening opportunity (e.g., a streamed byte-limit) if this ever becomes a real-world attack surface.

## Final Verdict

PASS WITH WARNINGS at the gate-execution and core-invariant level (all 8 gates genuinely green, core purity and deny.toml containment genuinely hold); 3 CRITICAL findings block clean archive until either (a) the spec/design artifacts are corrected to describe the actual 5-command surface and FreshnessSettings type, and (b) the two untested-but-structurally-sound privacy/incident-isolation scenarios get direct test coverage, or the orchestrator/user explicitly accepts the gap in writing.

---

## Addendum — orchestrator response to this report (2026-08-24)

Written by the orchestrator, not by the verify agent. The three CRITICAL findings were confirmed independently before acting on them, and all three are now closed.

### CRITICAL 1 — command surface contradicted the spec: CLOSED

Confirmed: `specs/desktop-shell/spec.md` said "SHALL expose exactly three commands" and "All three commands SHALL be async" while the shipped shell exposes five. The finding's diagnosis of the cause is correct and worth recording: the gap was **narrated** in `apply-progress.md` rather than **corrected** in the artifacts, and narration is not correction.

- `specs/desktop-shell/spec.md` now describes the five-command surface, states why the two settings commands are not optional (the confirmed default posture requires a visible opt-out and a first-run disclosure, and neither can function without reading and mutating persisted state), and adds the constraint that the settings-write command is the only command permitted to cause a write.
- `design.md` §11 corrected from "One new command" to three, with all three signatures, an explicit correction note naming this finding, and two constraints the original omitted: the write command takes the full desired state rather than a partial patch, and it is the only writer.

### CRITICAL 2 — `domain-model` type enumeration undercounted: CLOSED

Confirmed: the spec enumerated five new types and omitted `FreshnessSettings`, which exists, derives `TS`, and has a committed binding. Corrected to six new types plus the modified `ClientPresence`, with `FreshnessSettings` named in both the prose and the `TS`-derivation list.

### CRITICAL 3 — two spec scenarios had no covering test: CLOSED

Both were structurally true but untested, which the report was right to refuse to accept as coverage.

- **"An outbound request carries no identifying content."** `build_client` set the User-Agent inline, and `reqwest::Client` exposes no way to read back the headers it was built with — so the requirement was *unassertable* as written. Extracted `fetch::user_agent()` so it can be tested, then added two tests: `the_user_agent_carries_the_product_version_and_nothing_else` (the token is exactly `vertice/<version>`, the version segment is digits and dots only, it contains no whitespace that could carry platform detail, and it embeds none of `USERNAME`/`USER`/`COMPUTERNAME`/`HOSTNAME`) and `no_request_url_carries_a_query_fragment_or_machine_identifier` (every slot that resolves to an upstream builds an HTTPS URL with no query string, no fragment, and no machine identifier).
- **"`Outdated` never triggers the incident indicator or the Home block."** Added `keeps an all-outdated freshness report out of the incident channel and the Home block` to `App.test.ts`, which renders a clean scan report plus an all-`Outdated` freshness report, asserts the freshness path actually ran (so the test cannot pass vacuously), then asserts the incident indicator stays absent on both component pages and the Home block stays healthy. `App.test.ts` also gained a `./lib/freshness` mock with safe defaults.

**Honest note on this last test:** it passed on first run. The guarantee was already structurally true — there is no code path from a verdict to `incidentCount` — so this is a regression guard, not a TDD-driven design step. It is recorded as such rather than presented as a red-to-green cycle.

### WARNING acknowledged, not closed

The `cache.rs` literal-path-marker heuristic checks five substrings and would miss e.g. a bare `D:\` or a relative literal that never touches `app_data_dir()`. The report's judgment — "sound in spirit, imperfect in exhaustiveness", and a reviewer should read it as grep-proven rather than machine-proven — is accepted as written. The test's own `static_proof_is_limited: true` already says so in code. Left as-is deliberately: broadening the marker list would add confidence without changing the fact that it is a heuristic.

### Gates re-run after these changes — all green

`cargo fmt --all --check` OK · `cargo clippy --workspace --all-targets -- -D warnings` OK · `cargo test --workspace --locked` no failures · `cargo deny check bans licenses` bans ok, licenses ok · `npm run lint` OK · `npm run check` 216 files, 0 errors · `npm run test` 11 files, **109** tests · `npm run build` OK
