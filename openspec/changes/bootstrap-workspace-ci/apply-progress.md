# Apply Progress: Bootstrap Workspace and CI

> Mode: Strict TDD. Test runner: `cargo test && npm run test`.
> Status: all 39 tasks addressed. See per-task notes in `tasks.md` for what was
> verified locally vs. what still depends on the real GitHub Actions run
> (marked `[~]` there instead of `[x]` — see "Checkbox convention" below).

## Checkbox convention used in `tasks.md`

- `[x]` — done and verified (ran the actual command/tool and observed the result).
- `[~]` — done (file/config written, matches design.md) but only **partially**
  verified: local Windows-only environment, no macOS/Linux runners, no real
  GitHub Actions execution available in this sandbox. Treat these as
  "implementation complete, cross-platform/CI confirmation still pending."
  This is a deviation from the strict `[ ]`/`[x]` openspec convention, made
  deliberately so completion claims stay honest rather than overstating what
  was actually confirmed.

## Environment Notes (read first)

This sandbox had **no Rust toolchain installed at all** at the start of this batch.
- Installed `rustup` 1.29.0 via `scoop` → Rust `1.97.1` (stable, real, network-fetched from `static.rust-lang.org`). `rust-toolchain.toml` pins this exact version (no host-triple suffix, so real CI resolves it against whatever the runner's default host is).
- No MSVC Build Tools initially. Two paths were used:
  1. Installed `mingw-w64` (scoop `main/mingw`, gcc 16.2.0) and used the **GNU** Windows target (`RUSTUP_TOOLCHAIN=1.97.1-x86_64-pc-windows-gnu`) first.
  2. Later installed **Visual Studio 2022 Build Tools** (C++ workload) via `winget` (`winget install --id Microsoft.VisualStudio.2022.BuildTools --source winget ... --add Microsoft.VisualStudio.Workload.VCTools`), which succeeded, so the **MSVC** target (`x86_64-pc-windows-msvc`) was also verified locally.
  - **Both Windows toolchains build, test, and lint clean.** Neither macOS nor Linux was available to verify locally.
- `cargo-deny` 0.20.2 installed via `cargo install cargo-deny --locked`.
- Node 22.22.0 / npm 10.9.4 were already present.
- Live network access to crates.io, GitHub API, and npm registry was available and used for the Phase 6 YAML crate verification and for resolving current package/toolchain versions.

## Real findings that changed the plan mid-apply (read this before trusting design.md's original numbers)

1. **`crate-type` for `vertice-app`'s lib target**: the original scaffold used `["staticlib", "cdylib", "rlib"]` (Tauri's mobile-ready default). Building that with the GNU Windows toolchain failed at link time: `error: export ordinal too large: 89966` (a known mingw `ld` limitation with very large DLL export tables, which the wry/webview2 dependency tree produces). Since T1 is desktop-only (no mobile in scope), the correct fix — not a workaround — was to drop `staticlib`/`cdylib` and keep just `["rlib"]`; `src/main.rs` only needs an `rlib` to link against. This resolved the GNU build immediately and is arguably more correct scoping regardless of the linker quirk.
2. **MSRV**: design.md's approach of reading `rust_version` off `tauri`/`tauri-build`'s own `Cargo.toml` (suggesting `1.85`) was insufficient. Empirically pinning `RUSTUP_TOOLCHAIN=1.85` and running `cargo check --workspace --locked --all-targets` **failed** — several transitive dependencies pulled in through `tauri` (`darling`, `icu_collections`, `icu_locale_core`, `icu_normalizer`, `icu_properties`, `icu_provider`, `plist`, `serde_with`, `time`, plus `idna_adapter` at `1.86`) declare `rust_version = 1.88`. Re-running with `1.88` passed. **True MSRV is `1.88`**, not `1.85`. This is now set identically in `Cargo.toml`, `.github/workflows/ci.yml` (`MSRV` env + `msrv` job), and documented with the full empirical trail in `design.md`'s Open Questions.
3. **`deny.toml` `wrappers` semantics**: cargo-deny's `wrappers` field only exempts a banned crate's **direct** dependents, not any ancestor. The first `deny.toml` draft banned `wry`, `tao`, `tauri-runtime`, `tauri-runtime-wry`, `tauri-utils`, `tauri-macros`, `tauri-codegen` with `wrappers = ["vertice-app"]` — this **failed** (`bans FAILED`) because `vertice-app` is never those crates' direct parent (`tauri` and its own sub-crates are). Fixed by banning only `tauri` and `tauri-build` (vertice-app's actual **direct** runtime/build dependencies) with `wrappers = ["vertice-app"]` (and `"tauri"` too for `tauri-build`, since `tauri` itself uses `tauri-build` as an internal build-dependency). This is a more correct and more maintainable enforcement of the same invariant.
4. **`serde_norway` maintenance signal is weaker than the design assumed but not disqualifying**: live GitHub/crates.io checks (Phase 6) show no new crate release since Dec 2024 and several unanswered issues from 2025–2026, but the repo is not archived and ownership is not disputed — kept `serde_norway`, documented the caveat, did not fall back to `serde_yaml_ng`. Full trail in `design.md` Open Questions.
5. Removed a stray `vite-project/` directory (accidentally created while probing `npm create vite@latest -- --version`, which ignored the flag and scaffolded a real project) before it could pollute the diff.
6. Added `crates/vertice-app/gen/schemas` to `.gitignore` — Tauri regenerates these ACL/capability JSON schemas (IDE autocomplete only) on every build; not meant to be committed. Discovered by actually running the build, not planned in advance.

## TDD Cycle Evidence

| Task | Test File | Layer | Safety Net | RED | GREEN | TRIANGULATE | REFACTOR |
|------|-----------|-------|------------|-----|-------|-------------|----------|
| 2.4 (yaml behaviour probes) | `crates/vertice-core/tests/yaml_behavior.rs` | Unit (Rust) | N/A (new) | ✅ Written (referenced non-existent `vertice_core::yaml`; confirmed `error[E0432]: unresolved import`) | ✅ 5/6 passed first run | ✅ 6 scenarios (folded, literal, unquoted `no`, unquoted `2.0`, CRLF, duplicate keys) | ➖ None needed |
| 2.2 (lib.rs real assertion) | `crates/vertice-core/src/lib.rs` (`mod tests`) | Unit (Rust) | N/A (new) | ✅ Written | ✅ Passed | ➖ Single (seam reachability) | ➖ None needed |
| 4.6 (Vitest smoke test) | `frontend/src/lib/appTitle.test.ts` | Unit (TS) | N/A (new) | ✅ Written (confirmed `Cannot find module './appTitle'`) | ✅ Passed | ✅ 2 cases (different product name/version) | ➖ None needed |

**Note on the duplicate-key probe**: the first assumption was "last value wins." The actual observed `serde_norway` behaviour is a hard parse error (`"duplicate field 'value'"`). The test was rewritten to pin the *real, observed* behaviour (`duplicate_keys_are_rejected_as_a_parse_error`) rather than forcing the original assumption — this is exactly what a behaviour-probe seam is for.

### Test Summary
- **Total tests written**: 9 (1 lib unit + 6 Rust behaviour probes + 2 frontend unit)
- **Total tests passing**: 9/9 (verified on both `x86_64-pc-windows-gnu` and `x86_64-pc-windows-msvc`)
- **Layers used**: Unit (9), Integration (0), E2E (0 — deferred to T16 per design.md)
- **Approval tests**: None — no refactoring tasks, only new code
- **Pure functions created**: 2 (`vertice_core::yaml::from_str`, `frontend/src/lib/appTitle.ts::appTitle`)

## What was actually run and passed locally

Rust (both `x86_64-pc-windows-gnu` and `x86_64-pc-windows-msvc`, toolchain `1.97.1`, unless noted):
- `cargo fmt --all --check` — clean
- `cargo clippy --workspace --all-targets -- -D warnings` — zero warnings
- `cargo test --workspace --locked` — 7/7 passing
- `cargo build --release -p vertice-app` — succeeds, produces `target/release/vertice-app.exe`
- `cargo deny check bans` — `bans ok`; verified it correctly fails (`bans FAILED`) when `tauri` is added directly to `vertice-core`
- `cargo deny check licenses` — `licenses ok`
- MSRV floor: `RUSTUP_TOOLCHAIN=1.88 cargo check --workspace --locked --all-targets` passes; `RUSTUP_TOOLCHAIN=1.85` fails (proves the floor is real, not just declared)
- `cargo metadata` — exactly 2 workspace members, both inherit `edition`/`rust-version`/`license` from `[workspace.package]`

Frontend (Node 22.22.0 / npm 10.9.4):
- `npm ci` — clean install from `package-lock.json`
- `npm run lint` (ESLint 10, flat config) — clean
- `npm run check` (svelte-check) — 0 errors, 0 warnings
- `npm run test` (Vitest) — 2/2 passing
- `npm run build` (Vite) — produces `frontend/dist`

Not run / not verifiable in this sandbox:
- macOS and Linux legs of the `rust` matrix job (no such environment here)
- The actual GitHub Actions workflow execution (structural review only — YAML wiring matches design.md's CI table; not exercised by a real PR)
- `cargo build --release -p vertice-app`'s produced binary was not launched/smoke-tested as a running GUI app (would require an interactive desktop session; out of scope for this batch)

## Phase Status

- [x] Phase 1: Workspace Root
- [x] Phase 2: `vertice-core` skeleton
- [x] Phase 3: `vertice-app` skeleton
- [x] Phase 4: `frontend/` skeleton
- [x] Phase 5: CI workflow (written; not executed by real CI)
- [x] Phase 6: YAML crate pre-merge verification
- [x] Phase 7: Closure verification (see `[~]` items in `tasks.md` for what's cross-platform/CI-pending vs. fully confirmed)

`tasks.md` in this same directory is the authoritative per-task checkbox state.
