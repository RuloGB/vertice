# Vertice — Testing Capabilities

**Strict TDD Mode**: enabled
**Detected**: 2026-08-17

## Test Runner

- **Command (Core)**: `cargo test` / `cargo test --release`
- **Command (Frontend)**: `npm run test` (Vitest)
- **Command (E2E)**: `tauri-driver` (WebDriver, Tauri 2)
- **Framework**: Rust #[test], Vitest, tauri-driver

## Test Layers

| Layer       | Available | Tool              | Notes                                                    |
| ----------- | --------- | ----------------- | -------------------------------------------------------- |
| Unit        | ✅        | cargo test        | vertice-core logic, no filesystem dependencies           |
| Unit        | ✅        | Vitest            | Frontend components and utilities                        |
| Integration | ✅        | cargo test        | Adaptor integration with real fixture trees             |
| Integration | ✅        | Vitest            | IPC command/event contract between layers               |
| E2E         | ✅        | tauri-driver      | Full application workflows (Windows/Linux confirmed)     |
| E2E         | ⚠️        | tauri-driver      | macOS E2E limited (WebDriver support gaps); use manual   |

## Coverage

- **Available**: ✅ (not required for v0)
- **Command (Core)**: `cargo tarpaulin --out Html` or `cargo llvm-cov`
- **Command (Frontend)**: `vitest --coverage`
- **Threshold**: None (PoC prioritizes core correctness over coverage percentage)

## Quality Tools

| Tool         | Available | Command                          | Config                        |
| ------------ | --------- | -------------------------------- | ----------------------------- |
| Linter       | ✅        | `cargo clippy -D warnings`       | `Cargo.toml` lints             |
| Formatter    | ✅        | `cargo fmt --check` / `fmt`      | `Cargo.toml` edition = 2021    |
| Type Checker | ✅        | `rustc --crate-type lib` (via cargo) | Built into cargo/rustc       |
| Linter       | ✅        | `npm run lint`                   | ESLint config (frontend)       |
| Formatter    | ✅        | `npm run format`                 | Prettier (frontend)            |

## CI Matrix (GitHub Actions)

- **Operating Systems**: macOS, Windows, Linux
- **Rust Version**: MSRV (tracked in Cargo.toml) + stable
- **Node Version**: LTS
- **Triggers**: All commits, PRs, tags

## Build and Packaging

- **Core Build**: `cargo build --release` per platform
- **Frontend Build**: `npm run build` (Vite, embeds in Tauri)
- **Packaging**: `tauri-action` for installers (macOS .dmg, Windows .msi, Linux AppImage)
- **Signing** (PoC): None
- **Signing** (Post-v0): Add to workflow before public release (macOS notarization, Windows code signing)
- **Attestation**: SLSA provenance attached to GitHub Release

## Testing Strategy

### Fixtures-Based Testing
- All adaptor tests use versionable fixtures in `tests/fixtures/` (not machine-dependent paths)
- Fixtures cover:
  - Valid SKILL.md with normal and multi-line frontmatter
  - Broken YAML/JSON, missing fields, non-UTF-8 files
  - Skills in `_shared` (no filtering by name convention)
  - Agents in both opencode.json and opencode.jsonc with merge semantics
  - Empty and missing directories (no error)
  - Duplicate detection across three roots

### Platform-Specific Verification (T16)
- Fixture tests run on all three platforms in CI
- System verification (path discovery) is manual:
  - Windows: %APPDATA% paths confirmed
  - macOS: ~/Library/Application Support and XDG paths to be verified
  - Linux: XDG paths to be verified
- Oracles used for manual verification (not automated):
  - `opencode debug skill`, `opencode debug config`, `opencode debug paths`
  - `claude agents` (text output only, no --json)

### Read-Only Invariant (T14)
- Code review: grep for write-mode file opens outside app data dir
- Filesystem test: execute scan on fixture tree, compare hash + mtime before/after
- Tauri ACL review: verify capabilities scope restrictiveness

## Known Limitations

- **E2E on macOS**: tauri-driver WebDriver support is incomplete; critical flows covered by core integration tests + manual verification
- **Claude Code Agents Oracel**: `claude agents` outputs text only (no --json); validation against fixtures instead
- **YAML Crate Selection**: Decision point in T1; candidates under evaluation (serde_yaml replacement needed)
- **Platform Paths**: macOS and Linux verification not yet completed (T16 closes this)

## Acceptance Criteria Tied to Testing

| CA | Name                          | Test Method                                 |
| -- | ----------------------------- | ------------------------------------------- |
| 1  | Startup without config        | E2E: launch app, verify UI loads            |
| 2  | 25 skills, not 69 consolidation | Unit: aggregation logic on fixtures        |
| 3  | 22 duplicates marked, 3 routes | Unit + E2E: consolidation, duplicate marks |
| 4  | Single-root skills unmarked    | Unit: consolidation logic                   |
| 5  | Agents (Claude + OpenCode)     | Unit: adaptor tests on fixtures             |
| 6  | No plugin skills               | Unit: plugin exclusion logic                |
| 7  | Dual Claude Code versions      | Unit: client detection on fixtures          |
| 8  | `_shared` as skill, duplicated | Unit: no-filter-by-name logic               |
| 9  | Empty `skill/` (singular) handled | Unit: alias path handling                  |
| 10 | Multi-line description intact  | Unit: YAML frontmatter parser test          |
| 11 | Absent client as "not detected" | E2E: UI states for missing clients          |
| 12 | Broken file marked, scan continues | Unit: error aggregation in ScanReport     |
| 13 | Embedded agents marked, no actions | Unit: Location.path=None, E2E: UI disabled |
| 14 | Filesystem unchanged           | Unit: hash comparison test (T14)             |
| 15 | Scan completes in <2 seconds   | Benchmark: measure scan duration            |
| 16 | No writes outside app data dir | Code review + filesystem test               |
| 17 | Tests pass on all platforms    | CI matrix: 3 OS × 2 Rust versions           |

## Pre-Commit Checks

```bash
# Core
cargo fmt --check
cargo clippy -D warnings
cargo test

# Frontend
npm run lint
npm run test

# Full build (local)
cargo build --release
npm run build
```

## Release Validation (T15 / T16)

Before each release:
1. All tests pass on CI matrix
2. No formatting or lint issues
3. E2E workflows verified (macOS: manual; Windows/Linux: automated)
4. Oracles (opencode debug, claude agents) confirm adaptor correctness
5. Filesystem audit confirms no unintended writes (T14)
6. Installers generated and tested on reference machine
