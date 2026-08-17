# Verification Report: bootstrap-workspace-ci

> Change: bootstrap-workspace-ci (T1 / CA-17, enforcement point for CA-16)
> Mode: Full artifacts (proposal + design + specs + tasks + apply-progress present)
> Verified: 2026-08-17, against commit e108fc7 on main (pushed, matches origin/main)

## Completeness (tasks.md)

All 39 tasks checked ([x] or [~]). No unchecked ([ ]) tasks found. No CRITICAL from incomplete tasks. Since apply-progress.md was written, a real GitHub Actions run has since gone green on all 6 jobs, independently re-verified below, so the [~] items are now fully confirmed rather than only locally-partial.

## Build/Test/Lint Evidence

Executed live in this session, toolchain 1.97.1-x86_64-pc-windows-msvc unless noted.

| Command | Result | Notes |
|---|---|---|
| cargo fmt --all --check | PASS (exit 0) | |
| cargo deny check bans | PASS (bans ok) | |
| cargo clippy --workspace --all-targets -- -D warnings | PASS (exit 0, zero warnings) | |
| cargo test --workspace --locked | PASS (7/7: 1 lib test + 6 yaml_behavior probes) | |
| cargo build --release -p vertice-app | PASS | |
| RUSTUP_TOOLCHAIN=1.88 cargo check --workspace --locked --all-targets | PASS | proves MSRV floor compiles |
| RUSTUP_TOOLCHAIN=1.85 cargo check --workspace --locked --all-targets | FAILS as expected, requires rustc 1.88, 6 crates including darling, icu_ families, time, serde_with, idna_adapter | proves floor is real |
| Core-purity negative test: injected tauri = 2 directly into crates/vertice-core/Cargo.toml, ran cargo deny check bans | FAILS as expected, bans FAILED, crate tauri 2.11.5 is explicitly banned | reverted via git checkout, git status clean afterward, bans ok re-confirmed |
| npm ci (frontend) | PASS | Node 22.22.0, npm 10.9.4 |
| npm run lint (ESLint 10 flat config) | PASS | |
| npm run check (svelte-check) | PASS, 153 files, 0 errors, 0 warnings | |
| npm run test (Vitest) | PASS (2/2) | |
| npm run build (Vite) | PASS, produces frontend/dist | |

## Real GitHub Actions Evidence

Independently re-fetched via gh run view / gh run list in this session, not just trusted from the task prompt.

- Run 32015782992 (commit e108fc7, push to main): all 6 jobs green: quality, frontend, msrv, and rust on ubuntu-24.04, windows-2022, macos-14. Confirms the macOS and Linux legs, never run locally in this sandbox, actually pass.
- Prior run 32014258734 (commit 561e1cb, push to main): the msrv job failed with glib-sys 0.18.1 build script exiting nonzero, pkg-config exited with status code 1. This confirms the documented root cause (missing libwebkit2gtk-4.1-dev / pkg-config glib-2.0 on the bare ubuntu-24.04 runner) was real, and that the fix commit resolved it.
- gh pr list --repo RuloGB/vertice --state all returns empty. No pull request has ever been opened; all commits were pushed directly to main. The pull_request trigger path in ci.yml has therefore never actually executed in this repo, only the push-to-main path has real execution evidence.
- gh api repos/RuloGB/vertice/branches/main/protection and the rulesets endpoint both return HTTP 403, "Upgrade to GitHub Pro or make this repository public". Branch protection and required-status-checks state on main is unverifiable on the current plan for a private repo.

## Spec Compliance Matrix

### workspace-architecture

| Requirement / Scenario | Status | Evidence |
|---|---|---|
| Two-Crate Workspace Layout / Workspace resolves with two members | PASS | cargo metadata shows exactly 2 members: vertice-core, vertice-app |
| Two-Crate Workspace Layout / Shared package metadata | PASS | both crates inherit edition=2021, rust-version=1.88, license=MIT OR Apache-2.0 via .workspace = true |
| Core Purity Invariant / Dependency graph contains no Tauri crates | PASS | cargo deny check bans returns bans ok. The implementation substitutes cargo-deny for the literal cargo tree mechanism named in the scenario text; this substitution is explicitly reasoned about and documented in the design.md Core-purity enforcement table, not an undocumented deviation. |
| Core Purity Invariant / Accidental Tauri import is caught before merge | PASS (mechanism) / UNVERIFIED (merge gate) | Reproduced live: injecting tauri into vertice-core makes cargo deny check bans fail with exit 2, and the same command runs in the CI quality job. But the requirement that a pull request cannot merge needs branch-protection required-status-checks on main, which is unverifiable (GitHub API 403, private repo, no GitHub Pro) and, more importantly, no PR has ever been opened in this repo. The merge-blocking behavior has never been exercised, only the job-failure mechanism. |
| MSRV Pinned and Enforced / MSRV declared and consistent | WARNING, literal scenario wording not met | The scenario states both files declare the same Rust version. Actual state: Cargo.toml rust-version is 1.88, rust-toolchain.toml channel is 1.97.1; these are not the same version, by design (MSRV is pinned twice: toolchain and manifest floor). The CI MSRV consistency step checks a floor relationship, toolchain channel not below manifest rust-version, not equality. This is sound Rust practice and intentional, but the spec scenario text does not match what was built. |
| MSRV Pinned and Enforced / MSRV violation fails CI | PASS | Reproduced live: RUSTUP_TOOLCHAIN=1.85 fails to compile with explicit requires rustc 1.88 errors; 1.88 passes. The msrv job in CI does exactly this and is confirmed green in the real run. |
| YAML Crate Decision / Decision documented with justification | PASS | design.md YAML crate table lists all 4 candidates, serde_yml, yaml-rust2, serde_yaml_ng, serde_norway, with maintenance/serde-integration/verdict, plus a live-verification addendum in Open Questions |
| YAML Crate Decision / Block-scalar parsing verified before selection | PASS | tests/yaml_behavior.rs folded_scalar_joins_lines_with_spaces passes, part of the 6/6 run above |

### ci-quality-gates

| Requirement / Scenario | Status | Evidence |
|---|---|---|
| Cross-Platform CI Matrix / Matrix triggers on pull request | UNVERIFIED | ci.yml declares on: pull_request, but no PR has ever been opened in this repo, so this path has never actually executed |
| Cross-Platform CI Matrix / Matrix triggers on push to main | PASS | Both real runs (32014258734, 32015782992) triggered via push to main, ran the full 3-OS rust matrix plus quality, frontend, and msrv |
| Cross-Platform CI Matrix / One platform failure blocks merge | UNVERIFIED | Depends on branch-protection required status checks, unverifiable via API (403 on this plan and visibility level); no PR ever exercised this path. The design and mechanism are correct (fail-fast: false, independent jobs) but the merge-blocking guarantee itself is unproven in this repo. |
| Formatting Gate | PASS | cargo fmt --all --check clean locally; quality job green in real CI |
| Lint Gate (Clippy) | PASS | cargo clippy --workspace --all-targets -- -D warnings clean locally; rust job green on all 3 OSes in real CI |
| Test Gate / CA-17 scenario | PASS | cargo test --workspace --locked passed 7/7 locally on Windows; real CI green on ubuntu-24.04, windows-2022, macos-14, so the all-three-platforms claim is now genuinely demonstrated, not just structurally reviewed |
| Test Gate / Failing test blocks merge | UNVERIFIED (merge gate), mechanism sound | Same branch-protection caveat as above |
| Frontend Lint Gate | PASS | npm run lint clean locally; frontend job green in real CI |
| Application Build Gate | PASS | cargo build --release -p vertice-app succeeds locally on Windows and in real CI on all 3 platforms |

## Proposal Success Criteria

All 5 Success Criteria in proposal.md are functionally met per the evidence above, but the checkboxes in proposal.md itself remain unchecked (- [ ]) for all 5 items; the artifact was never updated to reflect completion. This is a documentation and artifact-hygiene gap, not a functional one.

- Met, documented as unchecked: cargo test and app build pass on all three matrix platforms
- Met, documented as unchecked: cargo fmt --check, cargo clippy -D warnings, frontend lint pass in CI
- Met, documented as unchecked: vertice-core imports nothing from tauri, verified mechanically
- Met, documented as unchecked: MSRV declared and CI job fails on violation
- Met, documented as unchecked: YAML crate decision written with justification

## Design Coherence

The three mid-apply corrections documented in design.md (crate-type set to rlib only, MSRV 1.88 rather than 1.85, deny.toml banning tauri and tauri-build directly rather than internal Tauri crates) all match on-disk code exactly. The two post-JD fixes described in the task context were independently confirmed against current on-disk files:

1. tauri.conf.json CSP is exactly: default-src self; connect-src self ipc: http://ipc.localhost -- confirmed, no style-src clause present.
2. .github/workflows/ci.yml: both quality and rust jobs contain the Read toolchain channel from rust-toolchain.toml step feeding dtolnay/rust-toolchain@master; the msrv job contains Install Linux WebView dependencies (libwebkit2gtk-4.1-dev, libsoup-3.0-dev, librsvg2-dev, patchelf) -- confirmed, and independently proven necessary by the real failed-then-fixed CI run pair above.

No other design deviations were found.

## CA-16 (read-only enforcement point)

grep -rn OpenOptions::write or File::create across crates/ returns no matches. capabilities/default.json grants only core:default, with no filesystem, shell, or dialog permissions. This matches tasks.md item 7.7 and the proposal CA-16 framing.

## Issues

### CRITICAL

None.

### WARNING

1. The workspace-architecture spec scenario "MSRV declared and consistent" does not match the implementation. The scenario text states both files declare the same Rust version, but the built system intentionally has 1.88 as a floor versus 1.97.1 as the pinned toolchain, checked as a floor relationship rather than equality. The design intent is correct and well reasoned; the spec prose should be corrected before or at archive so the artifact does not misdescribe the enforced invariant.
2. Merge-blocking behavior of CI is unverified, and the PR-trigger path has never executed. No PR has ever been opened on RuloGB/vertice; all 3 commits were pushed straight to main. Branch-protection and required-status-checks state on main is unverifiable via the GitHub API on the current plan (private repo, no GitHub Pro, HTTP 403 on the branch protection and rulesets endpoints). The ci-quality-gates spec cannot-merge scenarios (one-platform-failure, failing-test, build-failure) and the pull_request trigger scenario are therefore mechanism-proven but merge-gate-unproven. This matters because the real value proposition of CA-17 is that broken code cannot land, and that half of the guarantee has not actually been exercised or confirmed configured.
3. The Success Criteria checkboxes in proposal.md were never updated; all 5 remain unchecked despite being functionally met. This is cosmetic and hygiene-only, but should be fixed before archive so the artifact trail stays internally consistent.

### SUGGESTION

1. Consider opening at least one throwaway PR against main before declaring T1 fully closed, specifically to exercise the pull_request trigger and confirm or configure branch protection with required status checks. This is the only way to actually prove the cannot-merge-on-failure guarantee the spec claims.
2. The maintenance staleness of serde_norway (no release since Dec 2024, several unanswered 2025-2026 issues) is already flagged as a documented follow-up risk in design.md; carrying it forward into a future risk-tracking artifact, rather than only living in this change Open Questions section, would keep it visible past archive.

## Verdict

PASS WITH WARNINGS

All spec scenarios that can be verified through code and CI execution are met, with real, not just structurally-reviewed, green CI evidence on all three platforms, independently re-confirmed via gh run view in this session, including reproducing both the historical msrv job failure (missing Linux WebView dependencies) and its fix. Zero CRITICAL findings; zero unchecked tasks. The three WARNINGs are artifact-hygiene and verification-scope gaps (spec wording imprecision, unproven merge-gate enforcement, stale checkboxes in proposal.md) rather than functional defects in the shipped code.
