## Exploration: T14 — Read-only invariant audit

### Current State
- T14 targets `internal-docs/plan-desarrollo-poc.md` and closes **CA-16**: no writes outside the app data directory, explicitly verified.
- The live scan path is already narrow: `frontend/src/App.svelte` triggers `scan()`/`rescan()` on startup and reload; `frontend/src/lib/scan.ts` only invokes Tauri commands; `crates/vertice-app/src/commands.rs` delegates both commands to `vertice_core::scan::scan()` through `spawn_blocking`; `crates/vertice-core/src/scan.rs` composes adapters and returns an in-memory `ScanReport`.
- The Tauri capability is intentionally minimal today: `crates/vertice-app/capabilities/default.json` grants only `core:default`, with no fs/shell/dialog permissions.
- Core scan tests already prove a useful part of the invariant: `crates/vertice-core/src/scan.rs` has `reference_fixture_is_fast_and_read_only`, but it only snapshots file bytes before/after the scan. T14 still needs the stronger proof from the roadmap: hash + `mtime`, explicit write-surface review, and documented ACL/manual evidence.
- Current frontend tests (`frontend/src/App.test.ts`, `frontend/src/lib/scan.test.ts`) exercise UI and IPC behavior only; they are not the right place to prove CA-16.

### Affected Areas
- `crates/vertice-core/src/scan.rs` — natural home for the fixture-based no-write proof because it already owns the reference-volume scan test.
- `crates/vertice-core/src/skills.rs`, `agents.rs`, `opencode_agents.rs`, `installations.rs`, related helpers — audit targets for the actual disk surface used during scanning.
- `crates/vertice-app/capabilities/default.json` — proof point for the Tauri ACL claim; T14 should lock/document why `core:default` is sufficient and why no fs capability exists.
- `crates/vertice-app/src/commands.rs` — confirms the shell remains a thin pass-through and does not add persistence behavior.
- `openspec/specs/workspace-architecture/spec.md` and/or a T14 delta spec domain — likely place to codify the read-only guarantee and allowed write boundary.
- `internal-docs/plan-desarrollo-poc.md` / verification artifact — manual system-tool evidence must be documented because the roadmap requires empirical verification beyond tests.

### Approaches
1. **Strengthen the existing core fixture test** — extend the current `scan.rs` reference-volume test to snapshot metadata before and after the scan.
   - Pros: Reuses the exact production scan path; cheap to maintain; directly aligned with T14 roadmap wording; keeps the proof in the pure core where filesystem reads already happen.
   - Cons: Only proves fixture behavior for the core path; does not by itself document ACL scope or manual machine evidence.
   - Effort: Medium.

2. **Add a dedicated read-only audit layer across core + shell evidence** — combine (a) a metadata-preserving core test, (b) a static audit of write APIs/permissions, and (c) a documented manual verification step.
   - Pros: BEST match for T14 scope; closes the three proof channels the roadmap asks for (code path, fixture behavior, Tauri ACL/manual evidence); resilient against future accidental writes.
   - Cons: More artifacts to maintain; spec/tasks/verify phases must clearly separate automated and manual evidence.
   - Effort: Medium.

3. **Rely mainly on manual/system verification** — keep code almost unchanged and prove CA-16 mostly with shell tools and human audit notes.
   - Pros: Fastest to start; satisfies the roadmap's empirical-verification clause.
   - Cons: Too weak as the primary guarantee; easy to regress; misses the existing opportunity to turn the invariant into an automated fixture gate.
   - Effort: Low.

### Recommendation
Adopt **Approach 2**. T14 should be a focused audit change named `audit-read-only-invariant`: keep the production behavior unchanged, but harden the proof. The implementation should extend the existing core reference-volume test from byte snapshots to **content hash + `mtime` snapshots**, add an explicit static audit around the scanner's allowed disk surface and Tauri capability file, and require the verify phase to record the manual system-level evidence the roadmap asks for. That gives Vertice a layered guarantee instead of a hand-wavy "we think it is read-only" claim.

### Risks
- Filesystem timestamps can be platform-sensitive; the automated proof must compare metadata in a cross-platform-safe way or it will create flaky CI.
- A naive grep-based write audit can miss indirect writes or produce noisy false positives unless the allowed surface is stated precisely.
- If T14 tries to introduce app-data writes just to prove the exception boundary, it risks scope creep; the change should audit and verify, not add persistence.
- Manual verification is required by the roadmap, so the verify artifact must distinguish automated evidence from machine-specific operator evidence.

### Ready for Proposal
Yes — the product intent is clear, the affected layers are bounded, and the next phase should propose a hybrid proof strategy (automated core metadata test + static permission audit + documented manual evidence) for CA-16.