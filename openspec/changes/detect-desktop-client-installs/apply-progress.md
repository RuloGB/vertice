# Apply progress: Detect desktop client installations

Status as of this session: **Phases 0-9 and 11 complete and green. Phase 10 (manual oracle)
explicitly NOT run — requires physical access to `C:\Users\raul_`.**

See `tasks.md` for the full, phase-by-phase checklist with `[x]`/`[ ]` marks and inline notes
on every deviation from the literal task text (all deviations are reasoned and documented in
place — search `tasks.md` for "Deviation").

## Summary of what was built

- **H1 (frontend selection fix)**: `presenceFor` extracted to a new pure module
  `frontend/src/lib/pages/presenceFor.ts` (deviation from keeping it inline in
  `ClientsPage.svelte` — needed to unit-test the N=3 selection rule, which no real product
  group can exercise through DOM rendering today). `ClientsPage.svelte` now imports and calls
  it; behavior identical to design §6.1's code block otherwise.
- **`BENCH-1`**: measured 30.9 ms average / 43.6 ms worst-of-20 on a synthetic ~1.73 MiB header
  (release build). Falls in the 25-100 ms band → `HEADER_MAX_BYTES` kept at 4 MiB; real cost
  stated in `design.md` §3.1, the delta spec, and `internal-docs/pendientes-desarrollo.md`
  (entry P17).
- **`crates/vertice-core/src/asar.rs`** (new): the asar reader, one public function
  `read_package_version(&Path) -> Result<String, AsarError>`, 15 unit tests, zero
  `unwrap`/`expect`/`panic!`/`as`-cast in production code.
- **14 committed fixture blobs** under
  `crates/vertice-core/tests/fixtures/client-installations/opencode-desktop/<case>/`, each with
  an `app.asar.layout.txt` sidecar and covered by `tests/asar_fixture_integrity.rs` (15 tests:
  14 integrity + 1 read-package-version sanity check against the design's taxonomy table).
- **`ClientInstallSlot::OpenCodeDesktop`** added to `model/slot.rs`, positioned after
  `OpenCodeNpm`; wired through `installations.rs` (`VersionSource::AsarPackageJson`,
  `resolve_opencode_desktop_slot`, the new probe entry) and
  `crates/vertice-app/src/freshness/upstream.rs` (`None` upstream, same as `ClaudeCodeBundled`).
- **`frontend/src/bindings/ClientInstallSlot.ts`** regenerated (four variants → five); every
  other binding file confirmed byte-identical.
- **`ClientsPage.svelte`**'s `openCode` group gains `openCodeDesktop`.
- **Pin sweep** (Phase 8) completed across `client_installations.rs`, `scan.rs`,
  `model_contract.rs`, `freshness/upstream.rs`; the `isolation` fixture test needed a real
  restructure (not just a count bump) because its "not Codex" grouping logic would have
  silently absorbed the new slot.
- **`openspec/changes/detect-desktop-client-installs/specs/client-installation-detector/spec.md`**
  corrected: an earlier draft had claimed a time-budget ceiling and blanket non-`Error`
  severity for every asar failure mode, both withdrawn during design (§3.4, §5.2) but not
  reflected in the delta spec text until this pass. Also added the `BENCH-1` measured-cost
  note. `openspec/specs/` (the merged living spec) deliberately NOT touched — that merge is
  `sdd-archive`'s job.
- **`internal-docs/pendientes-desarrollo.md`**: new entry **P17**, recording H3, the deferred
  freshness upstream, the fixture self-consistency blind spot, the `@opencode-aidesktop`
  rename risk, two stale-copy items, and the measured `BENCH-1` cost.

## Gates actually run (Phase 11) — all green except the manual oracle

- `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace --locked` (30 test binaries, 0 failures), `cargo deny check bans
  licenses` (via a PATH prefix to `/c/Users/Raul/.cargo/bin` — `cargo-deny` is not on the
  default shim PATH in this environment) — all pass.
- `npm run lint && npm run check && npm run test && npm run build` from `frontend/` — all pass
  (214 vitest tests, 286 files typechecked clean, production build succeeds).
- No new dependency, no new Tauri capability, no MSRV drift, no stray `node_modules`.

## What is NOT done

**Phase 10 — the manual oracle.** Requires physical access to `C:\Users\raul_` and the real
OpenCode desktop install (143 MB `app.asar`). Cannot be run by this agent or by CI. Five items
remain open: A2 (root `package.json` entry shape), version-string equality against OpenCode's
own UI, wall-clock cost of `read_package_version` on the real archive, whole-scan time with/without
the desktop app, and recording all four in the change folder. This is a correct, expected stop —
not a failure of this apply pass.
