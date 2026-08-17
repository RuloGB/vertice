# Tasks: Bootstrap Workspace and CI

> Trace: T1 (Phase 0) / CA-16 (read-only enforcement point), CA-17 (cross-platform core tests).

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | ~700-900 hand-written; total incl. generated `Cargo.lock`/`package-lock.json` likely 3,000-6,000+ |
| 400-line budget risk | High (driven almost entirely by lockfiles, not reviewable logic) |
| Chained PRs recommended | No — a from-scratch scaffold cannot compile/test/lint in a broken intermediate state; CI's `rust`/`msrv` jobs need `frontend` and `vertice-core` to exist together |
| Suggested split | Single PR, `size:exception` |
| Delivery strategy | single-pr |
| Chain strategy | size-exception |

Decision needed before apply: Yes
Chained PRs recommended: No
Chain strategy: size-exception
400-line budget risk: High

### Suggested Work Units

| Unit | Goal | Likely PR | Notes |
|------|------|-----------|-------|
| 1 | Full T1 scaffold (workspace + core + app + frontend + CI) | PR 1 (`size:exception`) | Maintainer must accept the exception; hand-written diff is reviewable, lockfiles are generated and not line-by-line reviewed |

## Phase 1: Workspace Root

- [x] 1.1 Create root `Cargo.toml`: `[workspace] members = ["crates/vertice-core", "crates/vertice-app"]`; `[workspace.package]` (version, edition, rust-version, license, repository); `[workspace.dependencies]`; `[workspace.lints] unsafe_code = "deny"`.
- [x] 1.2 Create `rust-toolchain.toml` pinning one exact stable channel + `rustfmt`, `clippy` components.
- [x] 1.3 Create `deny.toml`: deny `tauri`/`tauri-*` except `wrappers = ["vertice-app"]`; `[graph] all-features = true`, `exclude-dev = true`, `targets` = all 3 triples.
- [x] 1.4 Update `.gitignore`: add `/target`, `frontend/node_modules`, `frontend/dist`.

## Phase 2: `vertice-core` Skeleton (Pure Library)

- [x] 2.1 Create `crates/vertice-core/Cargo.toml`: inherit workspace edition/version/rust-version; add `serde`, `serde_norway`, `thiserror`.
- [x] 2.2 Create `crates/vertice-core/src/lib.rs`: `pub mod yaml;` plus one real unit assertion.
- [x] 2.3 Create `crates/vertice-core/src/yaml.rs`: `from_str<T>` seam + `thiserror` error type — the only module allowed to import the YAML crate.
- [x] 2.4 Create `crates/vertice-core/tests/yaml_behavior.rs`: fixtures for folded `>`, literal `|`, unquoted `no`, unquoted `2.0`, CRLF, duplicate keys.
- [x] 2.5 Run `cargo test -p vertice-core --locked`; confirm all behaviour probes pass.

## Phase 3: `vertice-app` Skeleton (Tauri 2)

- [x] 3.1 Create `crates/vertice-app/Cargo.toml`: workspace inheritance, `tauri` dep, path dep on `vertice-core`.
- [x] 3.2 Create `build.rs` (`tauri_build::build()`).
- [x] 3.3 Create `tauri.conf.json`: `frontendDist: "../../frontend/dist"`, `beforeDevCommand`/`beforeBuildCommand` targeting `frontend`, `devUrl :1420`.
- [x] 3.4 Create `capabilities/default.json` with minimal scope — no broad filesystem access (CA-16).
- [x] 3.5 Create `src/main.rs` + `src/lib.rs`: minimal `generate_context!` entrypoint, no domain commands yet.
- [x] 3.6 Add `icons/` asset set required by Tauri's bundler.

## Phase 4: `frontend/` Skeleton (Svelte 5 + Vite + Tailwind)

- [x] 4.1 Scaffold `frontend/` (Vite + Svelte 5, no SvelteKit): `package.json`, `tsconfig.json`, `index.html`, `vite.config.ts`.
- [x] 4.2 Add Tailwind v4 via `@tailwindcss/vite` + `@import "tailwindcss"` — no `tailwind.config.js`.
- [x] 4.3 Add ESLint 9 flat config (`eslint.config.js`) + `eslint-plugin-svelte`.
- [x] 4.4 Wire `npm run check` (svelte-check).
- [x] 4.5 Create empty skeleton screen (`src/App.svelte`) + `src/main.ts` mount.
- [x] 4.6 Add one Vitest smoke test proving the test harness is wired; `npm run test` script.
- [x] 4.7 Run `npm ci && npm run build`; confirm `frontend/dist` is produced.

## Phase 5: CI Workflow (`.github/workflows/ci.yml`)

- [x] 5.1 Create workflow: `on: pull_request` + `push: branches: [main]`; `concurrency` group per ref, `cancel-in-progress`. Pin all runner images (never `-latest`).
- [x] 5.2 `quality` job (`ubuntu-24.04`): `cargo fmt --all --check`; MSRV consistency step; `cargo deny check bans`.
- [x] 5.3 `frontend` job (`ubuntu-24.04`): `npm ci`; lint; `check`; `test`; `build`; upload `frontend/dist` artifact.
- [x] 5.4 `rust` matrix job (`ubuntu-24.04`/`windows-2022`/`macos-14`, needs `frontend`): Linux apt deps (`webkit2gtk-4.1`, `libsoup-3.0`, `librsvg2`, `patchelf`); download `dist`; `Swatinem/rust-cache`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test --workspace --locked`; `cargo build --release -p vertice-app`.
- [x] 5.5 `msrv` job (`ubuntu-24.04`, needs `frontend`): `RUSTUP_TOOLCHAIN=<msrv> cargo check --workspace --locked --all-targets`.
- [x] 5.6 Resolve concrete MSRV number against Tauri 2 + plugin floors; set identically in `Cargo.toml rust-version`, `rust-toolchain.toml`, and the `msrv` job.

> Note: `.github/workflows/ci.yml` is written and locally sanity-checked (YAML structure, job/step wiring against design.md's table). It has **not** been executed by real GitHub Actions — that only happens once this branch is pushed and a PR is opened.

## Phase 6: YAML Crate Pre-Merge Verification (do not skip)

- [x] 6.1 On the live `serde_norway` repo, verify: last-commit recency, open-issue responsiveness, ownership/provenance.
- [x] 6.2 Re-confirm `serde_yml`'s maintainership dispute still stands (no reason to reconsider it as an option).
- [x] 6.3 If `serde_norway` fails verification: swap to `serde_yaml_ng`... — **not triggered**: `serde_norway` passed verification (not archived, ownership not disputed) with a documented staleness caveat. Kept `serde_norway`; no swap performed. See `design.md` Open Questions for full findings.
- [x] 6.4 Record the verification outcome (confirmed / fell back) in `design.md`'s Open Questions.

## Phase 7: Closure Verification

- [~] 7.1 Confirm `cargo test --workspace --locked` and `cargo build --release -p vertice-app` pass on all 3 matrix platforms (Success Criterion 1; CA-17). **Windows: verified locally**, both `x86_64-pc-windows-gnu` and `x86_64-pc-windows-msvc` (`cargo test --workspace --locked` 7/7 passing on both; `cargo build --release -p vertice-app` succeeds on both). **macOS and Linux: NOT verified locally** — no such environment available in this sandbox; depends on the actual GitHub Actions run.
- [~] 7.2 Confirm `cargo fmt --check`, `cargo clippy -D warnings`, frontend lint all pass in CI (Success Criterion 2). **Verified locally** on Windows (both toolchains for the Rust checks): `cargo fmt --all --check` clean, `cargo clippy --workspace --all-targets -- -D warnings` zero warnings, `npm run lint` (ESLint) clean, `npm run check` (svelte-check) 0 errors/warnings. **Not executed inside the actual `ubuntu-24.04` CI container** (webkit2gtk apt deps, runner image specifics) — depends on the real CI run.
- [x] 7.3 Confirm `cargo deny check bans` fails a deliberately-introduced `tauri` dep in `vertice-core` (Success Criterion 3; workspace-architecture spec: "Dependency graph contains no Tauri crates", "Accidental Tauri import is caught before merge"). **Verified**: temporarily added `tauri = "2"` to `crates/vertice-core/Cargo.toml`, ran `cargo deny check bans` → `bans FAILED` with `crate 'tauri = 2.11.5' is explicitly banned`, then reverted the change and confirmed `bans ok` again.
- [x] 7.4 Confirm MSRV is declared identically in both files and the `msrv` job fails on a violation (Success Criterion 4; spec: "MSRV declared and consistent", "MSRV violation fails CI"). **Verified empirically**: `RUSTUP_TOOLCHAIN=1.85 cargo check --workspace --locked --all-targets` (a value below the true floor) fails to compile with explicit `requires rustc 1.88` errors from transitive deps; `RUSTUP_TOOLCHAIN=1.88` passes. `Cargo.toml rust-version`, the `MSRV` workflow env var, and the `msrv` job's pinned toolchain are all set to `1.88`.
- [x] 7.5 Confirm the YAML decision lists all 4 candidates with maintenance/block-scalar/serde-integration status and justifies the rejections (Success Criterion 5; spec: "Decision documented with justification", "Block-scalar parsing verified before selection"). Table already present in `design.md` ("YAML crate" section); live verification outcome added to `design.md` Open Questions (Phase 6).
- [~] 7.6 Walk every `ci-quality-gates` scenario: PR/push triggers, single-platform failure blocks merge, unformatted/clippy/test/lint/build failures each fail CI independently. Workflow structurally satisfies all of these (`on: pull_request` + `push: branches: [main]`; matrix jobs run independently per-OS with `fail-fast: false`; each gate is its own step that exits non-zero on failure). **Not exercised in a real PR/CI run** — structural review only, pending the actual GitHub Actions execution.
- [x] 7.7 Grep `vertice-core` and `vertice-app` for `OpenOptions::write()` / `File::create()` outside the app data dir — none MUST exist (CA-16 read-only enforcement point). **Verified**: no matches in `crates/`.
