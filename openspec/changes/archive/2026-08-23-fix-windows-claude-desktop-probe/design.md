# Design: Fix The Windows Claude Desktop Probe Path And Slot Vocabulary

## Technical Approach

`windows_install_probes` stops being a pure `[InstallProbe; 3]` and becomes a **slot-grouped resolver** returning `Vec<InstallProbe>`. Three slots stay; the bundled-Claude slot contributes *1..N* candidate paths instead of one. Slot verdicts (detected / not detected / broken) are computed **per slot**, not per candidate, so CA-11 still yields at most one "not detected" warning per slot.

## Core Data Model Changes

**None.** `Component`, `Location`, `Scope`, `SearchRoot`, `ScanReport`, `ClientInstallation` and `ClientKind` are untouched. `model/` keeps its zero-I/O import allow-list; all enumeration lives in the `installations.rs` adapter. Consequence: **`frontend/src/bindings/` does not change**, so the CI bindings-in-sync gate must show an empty diff — a non-empty diff means something leaked into `model/` and is a design violation, not a regeneration chore.

## Architecture Decisions

| # | Decision | Alternatives rejected | Rationale |
|---|---|---|---|
| 1 | `InstallKind` is replaced by a private 3-variant `InstallSlot { ClaudeCodeNpm, ClaudeCodeBundled, OpenCodeNpm }` with `client()`, `label()`, `version_source()`. `InstallProbe` shrinks to `{ slot, path }`. | Keep `{Npm, Desktop}` and rename `Desktop`; add a third `InstallKind` alongside a `client` field | The old `(ClientKind, InstallKind)` pair could express nonsense pairs and forced the `{Client} ({kind})` string grammar, which cannot produce `Claude Code CLI (npm)` and `Claude Code (bundled in Claude Desktop)` from the same template. One slot enum makes the label a single settled constant and makes an inconsistent probe unrepresentable. |
| 2 | `windows_install_probes(home) -> Vec<InstallProbe>` and performs the bounded enumeration. | Fixed-size array; a separate pure table plus an impure resolver | The bundled slot's candidate count is data-dependent. `installations.rs` is the adapter layer, so I/O belongs here. Tests asserting `probes.len() == 3` are replaced by per-slot assertions. |
| 3 | `ClientKind` is unchanged — the bundled runtime is still `ClaudeCode`. | Add `ClaudeCodeBundled` variant | CA-7 says *both Claude Code installations*; they are the same product at different paths. A new variant would change exported bindings, break UI grouping, and push a private naming concern into the public contract. |
| 4 | MSIX and legacy may both resolve; each candidate yields its own `ClientInstallation`. **No deduplication, not even on equal version.** | Dedup by version; report a conflict issue | Mirrors `ClientInstallation`'s "installed twice is two values, never merged" contract and the existing two-version precedent. Paths differ by construction, so values are always distinguishable. |
| 5 | Prefix match is byte-exact on `OsStr::as_encoded_bytes().starts_with(b"Claude_")`. | `to_string_lossy` + case-insensitive compare | No UTF-8 requirement, no allocation, deterministic on every CI leg. Case folding would be an OS-convention inference — the rule at `plan-desarrollo-poc.md:179` forbids that. |
| 6 | `HostPlatform::current()` keeps `cfg!` as an **expression**. | `#[cfg]` attribute on the Windows table | Unchanged and non-negotiable: the Windows table must stay compiled and testable on the Linux/macOS legs. |

## Data Flow

```
home ──> windows_install_probes(home)
           ├─ ClaudeCodeNpm   -> 1 path   (AppData/Roaming/npm/node_modules/@anthropic-ai/claude-code)
           ├─ ClaudeCodeBundled
           │    read_dir(home/AppData/Local/Packages)          <- only enumeration in the codebase
           │      filter: is_dir && name starts with b"Claude_"
           │      sort:   byte-wise on file name
           │      each -> <pkg>/LocalCache/Roaming/Claude/claude-code
           │    ++ [home/AppData/Roaming/Claude/claude-code]   <- legacy, ALWAYS appended
           └─ OpenCodeNpm     -> 1 path
                         │
      for each slot ─────┴──> resolve_slot(slot, candidates)
             no candidate path exists -> 1 Warning "{label} not detected", path = legacy/only path
             otherwise                -> per existing candidate: package.json or version-dir extraction
```

The legacy path is appended **unconditionally**, so the bundled slot always has a deterministic path to name in its not-detected warning and `ScanIssue.path` is never `null` (`scanDiagnostics.isMissingClientIssue` depends on that).

## Error Paths (ScanIssue taxonomy)

| Condition | Severity | Notes |
|---|---|---|
| `AppData/Local/Packages` absent (`NotFound`) | *none* | Absence is not an event; the slot verdict comes from `resolve_slot`. CA-11. |
| `Packages` present but `read_dir` fails | `Error` | Present-but-unreadable, same precedent as the old desktop `read_dir` arm. Legacy fallback still evaluates — slot isolation holds. |
| Individual `DirEntry` error mid-iteration | `Error` | Continue iterating; never abort the slot. |
| `Claude_*` match exists but its `claude-code/` has zero version subdirectories | `Error` | Present-but-broken, never "not detected". Per candidate. |
| Version directory name is not UTF-8 | `Error` | Existing behaviour, `path: None`. |
| No candidate path exists for a slot | `Warning` | Exactly one per slot. Never `Error` (CA-11). |

Labels (settled): `Claude Code CLI (npm)`, `Claude Code (bundled in Claude Desktop)`, `OpenCode (npm)`. Reason grammar collapses to `format!("{} not detected", slot.label())`. The bundled slot's `Error` strings drop the word "desktop" for the same label.

## File Changes

| File | Action | Description |
|---|---|---|
| `crates/vertice-core/src/installations.rs` | Modify | `InstallSlot`, resolver, per-slot verdict, enumeration, labels, module doc (supersede §-references to the old design) |
| `crates/vertice-core/tests/fixtures/installations/` | Delete | Encodes the superseded table |
| `crates/vertice-core/tests/fixtures/client-installations/` | Create | New non-reused tree (CA-17) |
| `crates/vertice-core/tests/client_installations.rs` | Modify | New `fixture_home` root; new cases; drop the `desktop-empty` tripwire in favour of two new ones |
| `frontend/src/lib/scanDiagnostics.ts` | Modify | `MISSING_CLIENT_REASONS` -> the three settled strings |
| `frontend/src/lib/scanDiagnostics.test.ts` | Modify | Same strings |
| `frontend/src/bindings/` | Unchanged | Assert empty diff |

## Testing Strategy

Strict TDD, RED first. Order: the CA-7 pin (`packaged-and-legacy` yields 4 never-merged Claude installs) is written **first** and must fail before any resolver code exists; then the CA-11 pin (`nothing` -> 0 installs, exactly 3 Warnings, 0 Errors).

| Layer | What | How |
|---|---|---|
| Unit (`installations.rs`) | Slot labels; candidate ordering; byte-prefix filter; `HostPlatform::current()` cfg! expression | In-module tests, no I/O beyond `tempfile`-free fixture paths |
| Integration (fixtures) | `packaged`, `legacy`, `packaged-and-legacy`, `two-packages`, `packaged-empty`, `non-claude-packages`, `nothing`, plus the carried-over npm error/isolation/determinism/read-only cases | `scan_for(home, HostPlatform::Windows)` over the new tree — runs identically on all three CI legs |
| Frontend | `isMissingClientIssue` matches the three new strings and rejects the old ones | Vitest, run from `frontend/` |

Machine verification (`CLAUDE_CODE_EXECPATH` from an ordinary shell, **not** from inside Claude Desktop) is manual and out of the automated gate.

## Migration / Rollout

No migration: no persisted data, no IPC surface change, `vertice-app` untouched. Single PR with an accepted `size:exception`; the fixture tree replacement is the bulk of the diff.

## Open Questions

- [ ] None blocking. The frontend can only tell the bundled install from the npm one by inspecting `path`; closing that gap is deferred to the `inventory-ui` follow-up.
