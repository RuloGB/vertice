# Tasks: Frontmatter and `SKILL.md` Reader

> Trace: **T3** (Phase 1 — Reading, `plan-desarrollo-poc.md:92-107`) / closes **CA-10** (folded multi-line description), **CA-12 partial** (corrupt file carries its path); the absent-`description` success case indirectly feeds **CA-2**'s consolidation count. Core-only — no `npm run test` leg.

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | ~250-300 (`frontmatter.rs` incl. in-module unit tests) + ~250-300 (`tests/frontmatter_reader.rs`) + ~50-60 (`tests/yaml_seam_invariant.rs`) + ~1 (`lib.rs`) + ~120-180 (10 fixture `SKILL.md` files) + ~5 (`.gitattributes`) ≈ **~680-850 hand-written lines** |
| 400-line budget risk | High |
| Chained PRs recommended | Yes |
| Delivery strategy | ask-on-risk |
| Decision | **2-PR chain**, accepted by the user on 2026-08-17 |
| Chain strategy | `stacked-to-main` — PR 1 targets `main`; PR 2 targets PR 1's branch |

Decision needed before apply: **Resolved**
Chained PRs recommended: Yes — **accepted, as a 2-PR chain**
400-line budget risk: High for PR 2 (~500-650 lines); accepted deliberately

### Delivery Decision — Recorded

The forecast originally proposed a **3-PR** split (fixtures → module → integration suite). That split was **rejected** for a concrete reason: it puts `frontmatter.rs` in PR 2 while the fixture-driven suite proving **CA-10** and **CA-12** lands only in PR 3, so PR 2 would merge a module whose acceptance criteria are still undemonstrated. Code and the tests that justify it must travel in the same reviewable unit.

The T2 precedent (`archive/2026-08-17-domain-model-type-contract/tasks.md:22`) declined chaining entirely and took `size:exception`. T3 does not follow it: T2 mixed domain modeling with `ts-rs` generation plumbing, whereas T3's bulk is ten trivial fixtures plus mechanical test bodies over roughly 100 lines of real logic. The infrastructure genuinely separates from the logic here; in T2 it did not.

**PR 2 remains over the 400-line budget and that is accepted, not overlooked.** Splitting it further would re-introduce exactly the code/test separation this decision exists to avoid.

### Work Units

| Unit | Goal | PR | Base | Notes |
|------|------|----|------|-------|
| 1 | `.gitattributes`, 10 fixtures (incl. non-UTF-8 bytes), `yaml_seam_invariant.rs`, RED checkpoint probe (1.4) | PR 1 (~200 lines) | `main` | No `frontmatter.rs` yet; self-contained. Nearly all trivial content, except the byte-integrity reasoning behind `.gitattributes` and the non-UTF-8 fixture — the one part that earns focused review, and the reason this unit is separated at all. |
| 2 | `frontmatter.rs` (`split`, `FenceError`, `read`, `SkillFrontmatter`) + `lib.rs` wiring + in-module unit tests + `tests/frontmatter_reader.rs` full fixture-driven suite, tripwire, generic-reuse probe | PR 2 (~500-650 lines) | PR 1 branch | Phases 2 and 3 merged into one unit so the module ships with the tests proving CA-10 and CA-12. Over budget deliberately. |

## Phase 1: Fixture Infrastructure (read-only-safe, precedes implementation)

- [x] 1.1 Create `.gitattributes` at repo root: `-text` for `crates/vertice-core/tests/fixtures/**`, then `binary` for the non-UTF-8 fixture path (design §9, order load-bearing). **MUST land before 1.3.**
- [x] 1.2 Create nine plain-text fixtures under `crates/vertice-core/tests/fixtures/frontmatter/<case>/SKILL.md`: `valid-minimal`, `valid-folded-description` (add `license`, `disable-model-invocation`, a nested `metadata` map — doubles as the generic-reuse probe input), `valid-no-description`, `no-frontmatter`, `empty`, `corrupt-yaml`, `missing-name`, `type-mismatch-name`, `unterminated-fence`.
- [x] 1.3 Author `non-utf8-content/SKILL.md` with a deliberate byte-writing step (not a text editor) — e.g. a lone `0xFF` byte with LF endings. Depends on 1.1.
- [x] 1.4 **[Checkpoint — RED, unverified assumption, design §10]** Write a standalone test calling `yaml::from_str::<Probe>` directly with a YAML fragment matching `type-mismatch-name`'s content, hardcoded inline (`split`/`frontmatter.rs` don't exist yet). Run it. **Decision rule**: returns `Err` → assumption confirmed, proceed to Phase 2. Panics → STOP; do not patch `frontmatter.rs` — the fix belongs in `yaml.rs` (T1-shipped seam, own pinned tests) and MUST escalate to the orchestrator.
- [x] 1.5 Create `crates/vertice-core/tests/yaml_seam_invariant.rs`: walk `src/` and assert no `.rs` file besides `yaml.rs` contains `use serde_norway` or `serde_norway::` (design §11). Passes vacuously now; re-verify in 3.5.

## Phase 2: Module Implementation — TDD (RED → GREEN)

- [x] 2.1 [RED] In `frontmatter.rs` (new), write `#[cfg(test)]` unit tests for `split`: opening fence, closing fence, `Empty`, `NoOpeningFence`, `Unterminated`, CRLF fence, empty block (`---\n---\n` → `Ok("")`), fence not on line 1. No disk access.
- [x] 2.2 [GREEN] Implement private `fn split(source: &str) -> Result<String, FenceError>` (design §4: line-based, no slicing, no regex) to pass 2.1.
- [x] 2.3 [GREEN] Implement `pub struct SkillFrontmatter { name: String, description: Option<String> }` (`Deserialize` only, no `Serialize`/`TS`) and `pub fn read<T: DeserializeOwned>(path: &Path) -> Result<T, ScanIssue>` — five-step pipeline (§3): `fs::read` → `str::from_utf8` → `split` → `yaml::from_str::<T>` → map every arm to `ScanIssue` per the severity rule (§5) and `reason` prefixes (§7). **Read-only (CA-16): `std::fs::read` only — no `File::create`, `OpenOptions::write`, `fs::write` anywhere in this file.**
- [x] 2.4 Word the module doc in prose only — state "MUST NOT import the YAML crate directly" without writing `serde_norway::` or `use serde_norway`, so 1.5/3.5's textual check does not false-positive on the doc comment itself.
- [x] 2.5 Wire `pub mod frontmatter;` in `crates/vertice-core/src/lib.rs` (no crate-root re-export, matching `pub mod model; pub mod yaml;`).
- [x] 2.6 [REFACTOR] Confirm `split`/`FenceError` stay private; `cargo clippy -D warnings` clean.

## Phase 3: Integration Tests — TDD (RED → GREEN, fixture-driven)

- [x] 3.1 [RED] Create `crates/vertice-core/tests/frontmatter_reader.rs`: one test per fixture from Phase 1 asserting the exact `Ok`/`Err(ScanIssue)` shape and `severity` from design §7, plus the I/O-failure class via a non-existent repository-relative path (no fixture). Include CA-10 (folded description asserted in full, not a prefix) and CA-12-partial (corrupt-yaml → `path: Some`, `reason` non-empty) explicitly.
- [x] 3.2 [GREEN] Run `cargo test -p vertice-core --locked`; confirm 3.1 passes against Phase 2's `read`.
- [x] 3.3 Add the generic-reuse test (spec: Generic Over the Deserialization Target): a local non-skill struct reads `valid-folded-description/SKILL.md` via `frontmatter::read`, asserts `Ok` — no new fixture.
- [x] 3.4 Add `non_utf8_fixture_is_still_non_utf8_on_disk`: assert `std::fs::read(path).len()` equals the exact literal from 1.3, and `str::from_utf8(&bytes).is_err()`.
- [x] 3.5 Re-run `tests/yaml_seam_invariant.rs` now that `frontmatter.rs` exists as a real sibling module — confirm still passing (no longer vacuous).

## Phase 4: Verification (local, pre-commit gates)

- [x] 4.1 `cargo fmt --check`.
- [x] 4.2 `cargo clippy --workspace --all-targets -- -D warnings`.
- [x] 4.3 `cargo test -p vertice-core --locked` — all three test files plus in-module units, full green.
- [x] 4.4 **Read-only grep (CA-16, `rules.apply`)**: confirm no `File::create`, `OpenOptions::write`, or `fs::write` anywhere in `frontmatter.rs`.
- [x] 4.5 Confirm `git diff --exit-code -- frontend/src/bindings` stays clean — T3 adds zero `TS` derives, no regeneration expected (design §8 mechanical proof).
- [x] 4.6 **Platform note**: fixtures run on all three CI platforms via the existing matrix automatically; T3 owns no per-OS path discovery, so no manual system verification is required here (contrast T4/T16).
