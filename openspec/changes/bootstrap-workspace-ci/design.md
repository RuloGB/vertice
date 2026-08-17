# Design: Bootstrap Workspace and CI

> Trace: T1 / CA-17. Proposal: `openspec/changes/bootstrap-workspace-ci/proposal.md`.
> Config rules N/A in T1: no domain model (T2), no IPC commands (T10), no per-OS data paths (T2/T14), no `ScanIssue` taxonomy (T3+). The core/Tauri isolation diagram required by `rules.design` is below.

## Technical Approach

A Cargo workspace with two members under `crates/`, a root-level `frontend/` SPA built by Vite and embedded at Rust compile time, and one GitHub Actions workflow that gates PRs and `main`. The core-purity invariant (stack decision #5) is enforced by `cargo-deny`, not by review. MSRV is pinned twice — toolchain and manifest floor — with a consistency check so the two cannot drift.

## Repository Layout

```
Vertice/
├── Cargo.toml              # [workspace] members/package/dependencies/lints
├── Cargo.lock              # committed (workspace ships binaries)
├── rust-toolchain.toml     # exact pinned dev/CI toolchain (e.g. "1.XX.Y")
├── deny.toml               # cargo-deny: core-purity ban
├── .github/workflows/ci.yml
├── crates/
│   ├── vertice-core/       # pure lib: src/lib.rs, src/yaml.rs, tests/yaml_behavior.rs
│   └── vertice-app/        # Tauri 2: build.rs, tauri.conf.json, capabilities/, icons/, src/
└── frontend/               # Svelte 5 SPA (src/, index.html, vite.config.ts, dist/ gitignored)
```

`crates/` stays Rust-only so the post-PoC CLI is a third sibling; the frontend lives at the root so `node_modules/` never sits inside a Cargo package directory.

```
frontend (Svelte 5) ──IPC──> vertice-app (Tauri) ──calls──> vertice-core (pure)
                                                                  ▲
                                     future vertice-cli ──────────┘
```

`vertice-core` never points upward. That is the whole invariant, and `deny.toml` is what proves it.

## Architecture Decisions

### Core-purity enforcement

| Option | Tradeoff | Decision |
|---|---|---|
| `cargo tree -p vertice-core -i tauri` | Success case is a *non-zero exit* ("did not match any packages"); inverted logic, shell-quoting differs per OS | Rejected (diagnostic only) |
| `cargo metadata` + jq script | Needs jq/PowerShell forks; custom code to maintain | Rejected |
| **`cargo-deny` `bans.deny` with `wrappers`** | One cross-platform binary, deterministic exit code, versioned config, extends to advisories/licenses later | **Chosen** |

`deny.toml` declares `tauri` denied except with `wrappers = ["vertice-app"]`, so only `vertice-app` may be tauri's direct parent; any transitive path from core fails the check. Configure `[graph]` with `all-features = true` (an optional core feature cannot smuggle tauri in), `exclude-dev = true` (dev-only helpers are not shipped code), and `targets` listing all three triples so one Linux job covers the whole matrix. Only `check bans` gates PRs in T1 — `check advisories` is time-dependent and would red-flag unrelated PRs; it belongs in a scheduled workflow.

### YAML crate

Criteria: active maintenance, block scalars (`description: >`), serde integration quality. All three libyaml-backed serde_yaml forks handle folded/literal scalars identically, so criterion 2 does not discriminate — maintenance and trust do.

| Candidate | Assessment | Verdict |
|---|---|---|
| `serde_yml` | Maintainership and provenance **publicly disputed** in the Rust community (contested "official successor" claims, attribution removal, bulk low-quality changes). Trust is the product's thesis; a contested dependency is a liability | Rejected |
| `yaml-rust2` | Low-level YAML 1.2 parser, **no serde integration**. Every frontmatter struct would be hand-deserialized from an event stream — real recurring cost, more code to test | Rejected (fallback only) |
| `serde_yaml_ng` | Conservative near drop-in fork of `serde_yaml` 0.9, low but real activity | **Pre-approved fallback** |
| `serde_norway` | Same drop-in serde surface, plus explicit YAML 1.2 core-schema tag resolution — relevant because we parse third-party frontmatter we do not control (`enabled: no`, unquoted `version: 2.0`) | **Chosen** |

**VERIFY BEFORE MERGING** — this is best-available-judgment without live network access. Before the T1 PR merges, confirm on the actual repositories: last commit recency, open-issue responsiveness, ownership/provenance of `serde_norway`, and that `serde_yml`'s dispute still stands. If `serde_norway` fails verification, switch to `serde_yaml_ng` without redesign.

Two structural mitigations make the choice cheap to reverse: (1) the crate is used **only** through `vertice-core/src/yaml.rs`, a thin seam exposing `from_str<T>` and a `thiserror` error — no other module imports it; (2) `tests/yaml_behavior.rs` pins the behaviours we depend on (folded `>`, literal `|`, unquoted `no`, unquoted `2.0`, CRLF, duplicate keys). Swapping crates later means one dependency line plus one green test file.

### Frontend stack and embedding

Svelte 5 + Vite SPA, **no SvelteKit** (no SSR, no server router; adapter-static would add config surface for zero gain). Tailwind v4 via the `@tailwindcss/vite` plugin and `@import "tailwindcss"` — no `tailwind.config.js`.

| Flow | Chain |
|---|---|
| Dev | `cargo tauri dev` → `beforeDevCommand` (`npm run dev`, `cwd: ../../frontend`) → Vite on `:1420` → `devUrl` → HMR in the WebView |
| Release | `beforeBuildCommand` (`npm run build`) → `frontend/dist` → `frontendDist: "../../frontend/dist"` embedded by `tauri-build` / `generate_context!` → single binary |

**Gotcha this design must respect:** `generate_context!` reads `frontendDist` at *compile* time, so a bare `cargo test --workspace` fails if `frontend/dist` is missing. CI therefore builds the frontend once and every Rust job that touches `vertice-app` consumes that artifact.

### MSRV pinning

`rust-toolchain.toml` pins one exact stable patch version for local + CI (kills "works on my machine" drift). `[workspace.package] rust-version` declares the floor; members inherit via `rust-version.workspace = true`. A dedicated MSRV job overrides the pin with the `RUSTUP_TOOLCHAIN` env var — it takes precedence over `rust-toolchain.toml`, which `dtolnay/rust-toolchain` alone does not — and runs `cargo check --workspace --locked --all-targets`. A cheap consistency step asserts the manifest floor is not above the pinned channel. MSRV value = highest floor among direct deps (Tauri 2 and its plugins move this; resolve the concrete number at apply time).

## CI Workflow

`on: pull_request` + `push: branches: [main]`; `concurrency` group per ref with `cancel-in-progress`.

| Job | Runner | Steps |
|---|---|---|
| `quality` | `ubuntu-24.04` | `cargo fmt --all --check`; MSRV consistency; `cargo deny check bans` (core purity, all 3 targets) |
| `frontend` | `ubuntu-24.04` | `npm ci`; `npm run lint` (ESLint 9 flat + `eslint-plugin-svelte`); `npm run check` (svelte-check); `npm run test` (Vitest smoke); `npm run build`; upload `frontend/dist` artifact |
| `rust` (matrix) | `ubuntu-24.04`, `windows-2022`, `macos-14` | needs `frontend`; apt deps on Linux (`webkit2gtk-4.1`, `libsoup-3.0`, `librsvg2`, `patchelf`); download `dist`; `Swatinem/rust-cache`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test --workspace --locked`; `cargo build --release -p vertice-app` |
| `msrv` | `ubuntu-24.04` | needs `frontend`; `RUSTUP_TOOLCHAIN=<msrv> cargo check --workspace --locked --all-targets` |

Runner images are **pinned, never `-latest`** — that rotation is precisely how the webkit2gtk break arrives. Installer bundling stays out (T15); `cargo build --release -p vertice-app` already proves WebView linking on all three OSes. `npm run test` exists from day one because `rules.apply.test_command` is `cargo test && npm run test`.

## Risk Mitigations

| Risk | Concrete choice |
|---|---|
| webkit2gtk break on Linux | Pin `ubuntu-24.04`; declare supported floor (Ubuntu 22.04+ / Debian 12+, webkit2gtk-4.1) in README |
| MSRV drift | `rust-toolchain.toml` exact patch + `rust-version` + consistency step + MSRV job |
| Wrong YAML crate | Single-module seam + behaviour test suite + pre-approved fallback |
| Tauri leaking into core | `cargo deny check bans` with `wrappers`, all features, all targets |
| Non-reproducible builds | `Cargo.lock` committed, `--locked` on every cargo invocation, `npm ci` not `npm install` |

## File Changes

| File | Action | Description |
|---|---|---|
| `Cargo.toml` | Create | Workspace members, shared `[workspace.package]` (version, edition, rust-version, license, repository), `[workspace.dependencies]`, `[workspace.lints]` (`unsafe_code = "deny"`) |
| `rust-toolchain.toml` | Create | Pinned channel + `rustfmt`, `clippy` components |
| `deny.toml` | Create | Core-purity ban configuration |
| `crates/vertice-core/{Cargo.toml,src/lib.rs,src/yaml.rs,tests/yaml_behavior.rs}` | Create | Pure lib skeleton, YAML seam, behaviour probes |
| `crates/vertice-app/{Cargo.toml,build.rs,tauri.conf.json,capabilities/default.json,src/main.rs,src/lib.rs,icons/}` | Create | Tauri 2 shell, minimal capability set (no broad FS scope) |
| `frontend/**` | Create | Svelte 5 + Vite + Tailwind v4 SPA, ESLint/Vitest config, empty skeleton screen |
| `.github/workflows/ci.yml` | Create | Four-job workflow above |
| `.gitignore` | Modify | `/target`, `frontend/node_modules`, `frontend/dist` |

## Testing Strategy

| Layer | What | How |
|---|---|---|
| Unit (Rust) | Core compiles and one real assertion exists; YAML behaviour contract | `cargo test --workspace --locked` on all 3 OSes |
| Unit (frontend) | Harness is wired | One Vitest smoke test |
| Static | Format, lints, types, dependency graph, MSRV | fmt / clippy `-D warnings` / svelte-check / ESLint / cargo-deny / MSRV job |
| E2E | — | Deferred to T16 (`tauri-driver`; known macOS gap) |

## Migration / Rollout

No migration — additive to an empty repo. Rollback is reverting the branch.

## Open Questions

- [x] Exact MSRV number (resolve against Tauri 2 + plugin floors at apply time). **Resolved: `1.88`**, verified empirically (not just from crate metadata) by actually compiling the workspace with each candidate pinned via `RUSTUP_TOOLCHAIN=<version> cargo check --workspace --locked --all-targets`:
  - `tauri` (2.11.5) itself declares `rust_version = 1.77.2`; its `tauri-build` build-dependency (1.5.7, edition2024) declares `rust_version = 1.85`. Naively taking the highest declared `rust_version` among *direct* dependencies suggested `1.85` — **this was wrong.**
  - Pinning `1.85` and running `cargo check --workspace --locked --all-targets` **failed to compile**: `darling@0.23.0`, `darling_core`, `darling_macro`, `icu_collections@2.3.0`, `icu_locale_core`, `icu_normalizer`, `icu_normalizer_data`, `icu_properties`, `icu_properties_data`, `icu_provider`, `plist@1.10.0`, `serde_with@3.22.0`, `serde_with_macros`, `time@0.3.55`, `time-core`, `time-macros` all declared `rust_version = 1.88` (and `idna_adapter@1.2.2` declared `1.86`) — all **transitive** dependencies pulled in through `tauri`'s own dependency tree, none of which are visible by inspecting `tauri`'s or `tauri-build`'s own `Cargo.toml` alone.
  - Pinning `1.88` and re-running the same command **passed**.
  - Lesson for future MSRV bumps on this workspace: **do not** infer MSRV from direct-dependency metadata alone — pin a candidate and actually run `cargo check --workspace --locked --all-targets` with it, since transitive floors can be, and here are, higher.
  - Set identically in `Cargo.toml` (`rust-version = "1.88"`), the `msrv` CI job, and the `MSRV` env var checked by the "MSRV consistency" step in the `quality` job. `rust-toolchain.toml` pins `1.97.1` (the actual current stable at apply time) for local/CI dev builds — newer than the floor, which is expected and checked by the consistency step.
- [x] **`serde_norway` live verification** (repo activity, ownership, issue responsiveness) before merge; fallback `serde_yaml_ng`. **Verified live against crates.io + GitHub APIs at apply time — decision: CONFIRMED (kept `serde_norway`), with a documented caveat:**
  - Ownership: single owner, `cafkafk` (Christina Sørensen), verified via `crates.io/api/v1/crates/serde_norway/owners` — not disputed, not multi-party.
  - Repo state: `github.com/cafkafk/serde-norway` (renamed from `serde-yaml`), **not archived**, 56 stars, 3 forks, `pushed_at: 2025-08-04`.
  - Yellow flag: no new crate version published since `0.9.42` (2024-12-21) — roughly 20 months stale at apply time. Several open issues (e.g. #28, #30/#31, #32) from 2025 have no visible maintainer response as of apply time; one bug report (#37) was opened days before verification with no reply yet.
  - Counter-evidence it is not abandoned: repo is not archived, dependency-bot commits still land, and a new issue was filed within the verification window (someone is still actively using/watching it).
  - `serde_yml` re-check (should stay rejected): **confirmed and strengthened** — `crates.io` now shows `serde_yml` `0.0.13` is an explicit deprecation shim ("`serde_yml` is unmaintained... forwards every call to `noyalib`... migrate to `noyalib`"), published 2026-05-27. This is no longer just a provenance dispute; it is now officially deprecated. `yaml-rust2` and `serde_yaml_ng` were not re-evaluated as replacements since `serde_norway` was not disqualified.
  - Why kept `serde_norway` despite the staleness flag: it is not archived/deprecated/ownership-disputed (the only conditions design.md set as disqualifying), and it is the only candidate with the explicit YAML 1.2 core-schema tag resolution the project actually depends on (unquoted `no`/`2.0` staying strings — the exact reason it was chosen over `serde_yaml_ng` in the first place). `tests/yaml_behavior.rs` (Phase 2) pins this behaviour with real, passing assertions, and the seam in `src/yaml.rs` keeps a future swap to `serde_yaml_ng` a one-dependency-line change. Maintenance staleness is noted here as a follow-up risk to monitor, not a T1 blocker.
- [ ] Should the `yaml.rs` seam be enforced mechanically (import lint) or by convention? T1 uses convention.
- [ ] `cargo deny check advisories` cadence — scheduled workflow, not a PR gate.
