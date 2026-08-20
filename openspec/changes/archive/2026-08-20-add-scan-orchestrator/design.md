# Design: Add Scan Orchestrator

## Technical Approach

Add `vertice_core::scan::scan() -> Result<ScanReport, ScanError>` as T9's sole public workflow. It resolves home once, invokes the four existing infallible adapters, accumulates their output, emits diagnostics for `NotFound` roots, applies T8 `consolidate`, and measures elapsed time outside `model/`. No adapter parsing, root/probe table, model type, binding, IPC, or persistence changes.

## Architecture Decisions

| Decision | Choice | Alternatives considered | Rationale |
|---|---|---|---|
| Public boundary | One `scan()` facade; `scan_for(home, platform)` remains module-private for tests. | Expose fixture/path API; put orchestration in Tauri. | Keeps one production core entry point, isolates Tauri, and keeps fixture tests deterministic. |
| Failure model | Only home-resolution returns `ScanError`; adapter and item failures accumulate as `ScanIssue`. | Abort on first adapter error; wrap adapter errors. | Existing adapters are infallible and already isolate per-item work; `ScanReport` reserves `Err` for a scan that cannot begin. |
| Diagnostics | Preserve every adapter issue, then add one warning for each unique `SearchRootStatus::NotFound`; reuse installation “not detected” warnings. | Change adapters; infer missing clients in orchestrator. | Root status is the source of truth; installation probes already own exact client-slot diagnostics. |
| Timing and test seam | Start `Instant` in `scan_for`; production selects current platform, tests inject `HostPlatform::Windows`. | Clock in `ScanReport`; machine-dependent tests. | Preserves the zero-I/O model invariant and executes reference fixtures identically on all CI platforms. |

## Data Flow

```text
scan()
  -> roots::home_dir()? -> scan_for(home, current platform)
       -> skills::scan       -- roots/components/issues
       -> agents::scan       -- roots/components/issues
       -> opencode_agents::scan -- roots/components/issues
       -> installations::scan_for -- installations/issues
       -> add NotFound-root warnings -> consolidate(components)
       -> ScanReport { ..., duration_ms }
```

`roots_scanned` is the concatenation of component-adapter roots (three skill, two Claude-agent, one OpenCode-agent root), including absent roots. Component and installation vectors are moved from adapter result structs; issues are appended without filtering. The private helper executes every adapter sequentially; no adapter `Err` can prevent later calls. Corrupt/unreadable inputs remain adapter issues with their original paths (CA-12). Duration uses `Instant::elapsed`, converts milliseconds with a documented saturating conversion to `u32`, and is passed to the existing model constructor only as data.

## File Changes

| File | Action | Description |
|---|---|---|
| `crates/vertice-core/src/lib.rs` | Modify | Declare the new `scan` module. |
| `crates/vertice-core/src/scan.rs` | Create | Public facade, private path/platform seam, aggregation, missing-root diagnostics, consolidation, timing, and fixture tests. |
| `crates/vertice-core/src/installations.rs` | Modify | Make the current-platform selector crate-visible so the facade and private test seam share production platform selection. |
| `crates/vertice-core/tests/fixtures/scan-orchestrator/` | Create | Versioned combined homes for reference volume, corrupt-component isolation, and absent roots/clients. |

## Interfaces / Contracts

```rust
pub fn scan() -> Result<ScanReport, ScanError>;

// private to `scan`; invoked by module tests with HostPlatform::Windows
fn scan_for(home: &Path, platform: HostPlatform) -> ScanReport;
```

The report contract is unchanged: consolidated `components`, unmerged `installations`, all six `roots_scanned`, all diagnostics, and measured `duration_ms`. A missing root produces a warning with no file path and a deterministic root-id reason; a missing supported installation retains the adapter's warning and probe path. The existing generated TypeScript bindings remain untouched.

## Testing Strategy

| Layer | What to Test | Approach |
|---|---|---|
| Core unit | Complete composition, six roots, T8 overlap consolidation, and duration | `scan.rs` tests call private `scan_for` against versioned combined fixtures with Windows probes. |
| Core unit | CA-12 and isolation | Corrupt `SKILL.md` yields its path while valid agent/OpenCode/install results remain. |
| Core unit | Missing roots and clients | Assert root warnings plus existing `not detected` warnings; no silent omissions. |
| Core unit | CA-15 and read-only | Time reference fixture below two seconds; compare fixture-tree bytes before/after scan. |

Run `cargo test -p vertice-core --locked`; regeneration is unnecessary because public model types do not change.

## Migration / Rollout

No migration required. The new core API is unused by Tauri/UI until later phases; rollback removes this module and fixtures only.

## Open Questions

None.
