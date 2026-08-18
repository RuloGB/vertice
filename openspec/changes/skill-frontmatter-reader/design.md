# Design: Frontmatter and `SKILL.md` Reader

> Trace: **T3** (`internal-docs/plan-desarrollo-poc.md:92-107`) / closes CA-10 and CA-12 (partial).
> Proposal: `openspec/changes/skill-frontmatter-reader/proposal.md`. Exploration: `explore.md`.
> `rules.design` coverage: core data model impact (§8), core/Tauri isolation for the CLI pathway (§1), `ScanIssue` taxonomy and error paths (§7).
> `rules.design` items **N/A in T3, with reason**: **IPC contract surface** — T3 registers no Tauri command and adds no `TS`/`Serialize` derive, so `frontend/src/bindings/` is byte-identical after this change (§8). **Per-OS paths** — T3 receives an already-resolved `&Path` and performs zero path discovery; the `~/.claude/skills/` and `%APPDATA%` families are T4's and T14's to document. The only platform-specific matter T3 owns is fixture byte integrity and separator-safe test paths across the three-leg CI matrix (§9).

## 1. Technical Approach

One new module, `crates/vertice-core/src/frontmatter.rs`, turns a path into a typed value or a `ScanIssue`. It is the **first module in `vertice-core` to touch the filesystem**. Five ordered steps (§3), five failure arms, no panic on any of them.

The purity invariants do not move, and the distinction matters:

- **Crate purity** = Tauri-free (`lib.rs:1-5`, enforced by `deny.toml`). Unaffected: `std::fs` is not Tauri.
- **`model/` purity** = zero I/O, by a documented import allow-list (`model/mod.rs:8-15`) that names `std::fs` and `std::io` as forbidden. **Unaffected**: `frontmatter.rs` is a sibling of `model/`, not a member of it. `model/` gains no import.

```
frontend (Svelte 5) ──IPC──> vertice-app (Tauri) ──> vertice-core
                                                       ├── model/       (pure data, zero I/O)
   future vertice-cli ─────────────────────────────>   ├── frontmatter  (std::fs — NEW in T3)
                                                       └── yaml         (serde_norway seam)
                                                              ▲
                                       frontmatter ───────────┘  calls yaml::from_str, never serde_norway
```

The CLI pathway is preserved for free: nothing in `frontmatter.rs` knows what a Tauri command is, and its only inputs are a `&Path` and the file behind it. Both binaries would call the identical function.

## 2. Decision: module name, function names, public surface

| Question | Options | Decision |
|---|---|---|
| Module | `frontmatter.rs` / `reader.rs` / `skill.rs` | **`crates/vertice-core/src/frontmatter.rs`** — the crate names modules after the *thing* (`model`, `yaml`), never after the *role*. `reader.rs` names a role; `skill.rs` would be a lie, since the reader is generic and T5 reuses it for agents. |
| Reader | `read_frontmatter` / `from_path` / `read` | **`pub fn read<T: DeserializeOwned>(path: &Path) -> Result<T, ScanIssue>`** |
| Splitter | `pub` / private | **private `fn split(source: &str) -> Result<String, FenceError>`** |
| `lib.rs` | re-export / plain `pub mod` | **`pub mod frontmatter;` only** — no crate-root re-export, matching `pub mod model; pub mod yaml;` (`lib.rs:7-8`). |

`read_frontmatter` inside module `frontmatter` stutters (`frontmatter::read_frontmatter`). The crate's precedent is `yaml::from_str` — the module carries the noun, the function does not repeat it. `from_path` would mirror `yaml::from_str` most literally, and was the close call here; it was rejected because it reads as a cheap conversion in a crate whose defining property is that `model/` never touches disk. `read` makes the I/O boundary loud at every call site: `let fm: SkillFrontmatter = frontmatter::read(path)?;`.

The splitter stays private because no caller needs it: T4 walks directories and calls `read`; T5 supplies its own `T` and calls `read`. Exposing it would freeze a text-slicing helper into the crate's API for no consumer. Its edge cases (§4) are covered twice — by in-module `#[cfg(test)]` unit tests hitting `split` directly with no disk access, and by fixtures through `read`.

**Public surface of the module, complete:**

```rust
pub fn read<T: DeserializeOwned>(path: &Path) -> Result<T, ScanIssue>;

#[derive(Debug, Clone, PartialEq, Deserialize)]   // NO Serialize, NO TS — never crosses IPC
pub struct SkillFrontmatter {
    pub name: String,               // required: identity derives from it (identity::normalize_name)
    pub description: Option<String>, // optional: mirrors Component.description (component.rs:23)
}
```

`SkillFrontmatter` is `pub` (not `pub(crate)`) for one concrete reason: the fixture-driven tests live in `crates/vertice-core/tests/`, an external crate, per the house pattern (`tests/yaml_behavior.rs`, `tests/model_contract.rs`). It stays out of `model/` because `model/` is reserved for IPC-crossing, `TS`-derived types; this is a reader artifact T4 consumes and discards while assembling a `Component`.

**`deny_unknown_fields` MUST NOT be used.** Verified against real files: `~/.claude/skills/sdd-design/SKILL.md` frontmatter carries `disable-model-invocation`, `user-invocable`, `license`, and a nested `metadata` map. Denying unknown fields would reject essentially every real skill. Serde's default (ignore unknown keys) is the contract. Forward note for T5: real frontmatter uses kebab-case keys, so `AgentFrontmatter` will need per-field `#[serde(rename = "...")]`; `SkillFrontmatter`'s two keys need none.

## 3. The parse pipeline (order is load-bearing)

| # | Step | Call | Catches |
|---|---|---|---|
| 1 | Read bytes | `std::fs::read(path)` | I/O failure |
| 2 | Validate UTF-8 | `std::str::from_utf8(&bytes)` | non-UTF-8 **content** |
| 3 | Split fence | `split(content)` | empty file, absent fence, unterminated fence |
| 4 | Deserialize | `yaml::from_str::<T>(&block)` | corrupt YAML, unexpected type, absent `name` |
| 5 | Map error | — | every arm → `ScanIssue { severity, path: Some(path.to_path_buf()), reason }` |

**Why step 2 must precede step 3.** Mechanically, `split` takes `&str`, so the only way to invert the order is to hand it a lossy string. That is precisely the trap: `String::from_utf8_lossy` substitutes U+FFFD and yields a string that splits cleanly and often deserializes, producing a **wrong `Ok`** — a `SkillFrontmatter` whose `description` silently contains replacement characters — instead of a `ScanIssue`. Validating first converts silent corruption into a value-level failure.

**Why `fs::read` + `from_utf8` and not `fs::read_to_string`.** `read_to_string` already fails on non-UTF-8, but it collapses two distinct failure classes into one `io::Error`, separable only by sniffing `ErrorKind::InvalidData`. Splitting them structurally is clearer, and `Utf8Error::valid_up_to()` gives a real byte offset for `reason` that `io::Error` does not.

## 4. The splitter: exact behavior

```rust
enum FenceError { Empty, NoOpeningFence, Unterminated }

fn split(source: &str) -> Result<String, FenceError> {
    if source.trim().is_empty() { return Err(FenceError::Empty); }
    let mut lines = source.lines();
    match lines.next() {
        Some(first) if first.trim_end() == "---" => {}
        _ => return Err(FenceError::NoOpeningFence),
    }
    let mut block = String::new();
    for line in lines {
        if line.trim_end() == "---" { return Ok(block); }
        block.push_str(line);
        block.push('\n');
    }
    Err(FenceError::Unterminated)
}
```

**Out-of-bounds slicing is avoided by not slicing.** The `content.find("---")` + byte-index form is where a hand-rolled splitter panics; this implementation touches no index and no byte offset, so the failure mode is structurally impossible rather than carefully avoided. The cost is one `String` allocation over a file already fully in memory — irrelevant.

- **Empty file**: the `trim().is_empty()` guard fires first. Note honestly that this guard prevents no panic — `"".lines()` yields nothing and would fall through to `NoOpeningFence` safely. It exists to produce a *distinct diagnostic*, so the empty-file fixture asserts something the absent-frontmatter fixture does not.
- **Unterminated fence**: the loop consumes to EOF and returns `Unterminated`. It never treats the remaining Markdown as YAML.
- **CRLF**: `str::lines()` splits on `\n` and strips a trailing `\r`, so CRLF fences match with no special handling — the same normalization already pinned at `tests/yaml_behavior.rs:69-75`. `trim_end()` therefore covers only trailing spaces.
- **Fence must be line 1.** Leading blank lines before `---` yield `NoOpeningFence`. This is deliberate strictness, consistent with T4's "no heuristics" discipline; the frontmatter conventions all require the fence on the first line.
- **Empty block** (`---\n---\n`) returns `Ok("")` and lets step 4 reject it as a missing-field error. No special case.

## 5. Decision: `ScanIssue.severity` per failure class

The generating rule, not eight verdicts:

> **`Error` if and only if the file declared itself a frontmatter document — an opening `---` fence was found — and then failed to yield a valid `T`. Otherwise `Warning`.**

Severity is therefore a pure function of one predicate: *did we get past the opening fence?* One invariant, one line of code, directly testable.

`Warning` means "skipped, and the reader cannot prove this file was ever meant to be a component". `Error` means "this file announced a frontmatter block and then broke its own promise" — an authoring defect a user can act on. That is a genuinely useful T11 filter: Errors are a fix-list, Warnings are an informational skip-list.

**Anticipated objection:** an I/O error on a file T4 identified as `SKILL.md` loses the user a real inventory entry, so surely it is an `Error`. **Answer:** T3 is caller-agnostic and path-agnostic — it does not know the file was expected to be a skill. That knowledge lives in T4's walker. Encoding caller intent inside a leaf function is the wrong layer. **T3 emits the caller-agnostic floor; T4/T5 MAY escalate a returned `ScanIssue`'s severity when their own context establishes stronger intent.** That is the forward contract, and it resolves the objection without weakening the rule.

## 6. Decision: `ScanIssue.reason` is a developer diagnostic, not localized copy

| Option | Consequence | Decision |
|---|---|---|
| User-facing copy, i18n'd EN+ES per design principle 7 | `ScanIssue` lives in `model/`, whose import allow-list (`model/mod.rs:10-12`) contains no i18n crate and forbids `std::env` — the core **cannot read a locale** without breaking the invariant or threading a locale through every adapter signature | Rejected |
| A closed enum of translatable reason codes | Requires changing `ScanIssue.reason` from `String` to a code — a T2 model change, out of T3's scope and locked | Rejected |
| **Developer diagnostic; T11 renders user-facing copy from `severity` + `path`, and shows `reason` as verbatim technical detail** | Raw `serde_norway` error text MAY be embedded verbatim | **Chosen** |

Principle 7 governs the presentation layer, and T3 ships none. Translating a YAML parse error that names YAML constructs and byte offsets would translate the wrong thing.

**Constraints this creates, which T11 and T12 inherit:**

- `reason` is **not localized, not stable across `serde_norway` versions, and MUST NOT be parsed or branched on.** It gets exactly the treatment `Component.provenance_hint` already has — "opaque display string, never parsed" (`component.rs:26-31`). The precedent exists; T3 reuses it rather than inventing a second policy.
- T11 renders `reason` as verbatim technical detail (monospace, collapsible), excluded from the i18n catalog. **T12 has zero T3-authored strings to translate.**
- T3 authors a stable English prefix per failure class (§7) so a human reading a raw string knows the class. The prefix is a readability convention for humans and logs, **not** a machine contract.
- Corroborating evidence for the choice: corrupt YAML, unexpected type, and absent `name` all funnel through the *same* code arm (step 4). T3 structurally cannot distinguish them — only `serde_norway`'s message can. A localized taxonomy would have to fabricate a distinction the reader does not possess.

**Honest limitation.** T11 can then say "3 files could not be read" in Spanish but not "this file's YAML is corrupt" in Spanish. Acceptable for the PoC (CA-12 requires only that the issue carry its path and interrupt nothing). If T11 later needs class-level localized copy, the correct fix is an additive `ScanIssueKind` enum in `model/` with a `TS` binding — **never** string parsing. Recorded here so that future need does not get solved the wrong way.

## 7. Error paths: `ScanIssue` taxonomy

Every row produces exactly one `ScanIssue` with `path: Some(path.to_path_buf())`, and never a panic.

| Failure class | Caught at | `severity` | `path` | `reason` shape | Crosses IPC in T3? |
|---|---|---|---|---|---|
| I/O failure (unreadable, absent, permission) | 1 | `Warning` | `Some` | `could not read file: {io_err}` | No |
| Non-UTF-8 **content** | 2 | `Warning` | `Some` | `file content is not valid UTF-8 (valid up to byte {n})` | No |
| Empty / whitespace-only file | 3 | `Warning` | `Some` | `file is empty` | No |
| No opening `---` fence | 3 | `Warning` | `Some` | `no frontmatter block: file does not begin with a --- fence` | No |
| Unterminated opening fence | 3 | **`Error`** | `Some` | `unterminated frontmatter block: opening --- fence with no closing fence before end of file` | No |
| Corrupt YAML | 4 | **`Error`** | `Some` | `frontmatter is not valid YAML: {yaml_err}` | No |
| Unexpected type (`name` as a list) | 4 | **`Error`** | `Some` | same prefix; `{yaml_err}` carries `invalid type: sequence...` | No |
| Absent required `name` | 4 | **`Error`** | `Some` | same prefix; `{yaml_err}` carries `missing field \`name\`` | No |
| Absent `description` | — | *not a failure* | — | `Ok`, `description == None` | No |
| Non-UTF-8 **path** | not reachable in T3 | — | `None` (T2's contract) | — | T4's concern |

**The distinction that is easiest to get wrong, stated prominently:**

> **Non-UTF-8 *content* is not non-UTF-8 *path*.** T2's carried-forward contract (`archive/2026-08-17-domain-model-type-contract/design.md:161`) says `path: None` for a **path** that cannot be represented as UTF-8, because `PathBuf` serialization fails outright on such a path. T3 never meets that case: it receives an already-valid `&Path` from its caller, and T4 owns path discovery. When a file's **bytes** fail to decode, the path is perfectly good and MUST be carried: `path: Some(path)`. Nulling it would degrade CA-12's "carrying its path" requirement over a failure that has nothing to do with the path.

Nothing here crosses IPC in T3 — no command exists. These `reason` strings reach the UI only after **T9** aggregates them into `ScanReport.issues` and **T10** serializes the report. That eventual visibility is exactly what §6 decides the policy for.

## 8. Core data model impact

**None.** `Component`, `Location`, `Scope`, `SearchRoot`, `ScanReport`, `ScanIssue`, and `IssueSeverity` are consumed exactly as merged in T2; no field, derive, or serde attribute changes. `SkillFrontmatter` is a core-internal reader DTO, not a model type — it has no `Serialize` and no `TS`, so it emits no binding.

Concrete, checkable consequence: CI's existing `git diff --exit-code -- frontend/src/bindings` step stays green with **no** regeneration, because nothing T3 adds is `TS`-derived. That is the mechanical proof that T3 has no IPC surface.

T4 is the consumer: it calls `frontmatter::read::<SkillFrontmatter>(path)`, then assembles `Component { id: ComponentId::derive(Skill, &fm.name), name: fm.name, description: fm.description, kind, scope, locations, provenance_hint }`. T3 supplies exactly the two fields it can know and fabricates none of the rest.

## 9. Fixtures, `.gitattributes`, and the byte tripwire

**Directory convention** — decided now, because deciding at T4 costs a rename across suites:

```
crates/vertice-core/tests/fixtures/
├── frontmatter/     # T3+: single files addressed directly by path. NEVER walked.
│   ├── valid-minimal/SKILL.md
│   ├── valid-folded-description/SKILL.md
│   ├── valid-no-description/SKILL.md
│   ├── no-frontmatter/SKILL.md
│   ├── empty/SKILL.md
│   ├── corrupt-yaml/SKILL.md
│   ├── missing-name/SKILL.md
│   ├── type-mismatch-name/SKILL.md
│   ├── unterminated-fence/SKILL.md
│   └── non-utf8-content/SKILL.md
└── roots/           # RESERVED for T4+: whole trees walked from a root. T3 creates nothing here.
```

The split is not cosmetic. If T4 pointed its walker at a tree containing T3's deliberately-broken files, T4's "found N skills" assertions would couple to T3's fixture count — adding a T3 fixture would break a T4 test for no semantic reason. Separating *addressed files* from *walked trees* removes that coupling permanently. **T4 inherits**: add walked trees under `fixtures/roots/<client>/skills/...`; never aim a walker at `fixtures/frontmatter/`.

Each case is a directory containing `SKILL.md`, mirroring the real `skill-name/SKILL.md` shape, so a fixture can later be lifted into a `roots/` tree by moving one directory.

**Realism carried by the happy path**: `valid-folded-description` includes the extra keys real files carry (`license`, `disable-model-invocation`, a nested `metadata` map). This proves unknown-field tolerance without a dedicated fixture, and it doubles as the generic-reuse probe — a second, non-skill target type (e.g. `struct LicenseProbe { license: String }`) reads the same fixture, satisfying the proposal's "instantiated with a second target type" criterion with no new fixture and no new file on disk.

**`.gitattributes`** (repository has none today — verified). Created with exactly two rules, scoped to the fixture tree:

```gitattributes
# Test fixtures are byte-exact inputs. Never normalize line endings.
crates/vertice-core/tests/fixtures/** -text

# The non-UTF-8 fixture is binary content: also suppress diff/merge attempts.
crates/vertice-core/tests/fixtures/frontmatter/non-utf8-content/SKILL.md binary
```

Order is load-bearing: later rules win, so the `binary` macro (`-text -diff -merge`) must follow the `-text` line. The threat is concrete — Git's binary heuristic only looks for a NUL byte, so a fixture containing an invalid sequence such as a lone `0xFF` with LF endings is classified as **text** and is fully eligible for `core.autocrlf` rewriting on a Windows dev machine before it ever reaches the Windows CI leg.

**No repo-wide `* text=auto` line in T3.** It would renormalize every file in the repository on the next checkout — a whole-repo diff that dwarfs T3's and belongs with a tooling change, not a reader change.

**Tripwire test**, so a mangled fixture fails loudly instead of corrupting an unrelated assertion. A dedicated test named for its own failure — `non_utf8_fixture_is_still_non_utf8_on_disk` — asserts both:

1. `std::fs::read(path).len()` equals an exact literal, commented as a `.gitattributes` tripwire (catches CRLF rewriting, which changes length);
2. `std::str::from_utf8(&bytes).is_err()` (catches sanitization into valid UTF-8, which length alone would miss).

(2) is the property actually under test; (1) is the cheap corroborator. A maintainer seeing that test name fail knows immediately to check `.gitattributes`, not the reader.

**Path construction**: every fixture path is built from `env!("CARGO_MANIFEST_DIR")` and per-segment `PathBuf::push`/`join`, never a `"/"`-joined string literal. Machine-independent per `rules.verify`, and separator-correct on the Windows leg.

## 10. Unverified assumption: `serde_norway` on a type mismatch

**This is stated as an assumption, not a fact.** `tests/yaml_behavior.rs` pins folded and literal block scalars, bareword `no`/`yes`, CRLF, and duplicate-key rejection. It pins **nothing** about a scalar-typed field fed a YAML sequence. The plan asserts unexpected types must yield a `ScanIssue` and not a panic; T3 does not inherit that as verified.

Expected but unverified: serde's derived deserializer returns `Err("invalid type: sequence, expected a string")` by construction — a panic there would be a defect in `serde_norway`'s deserializer, not normal serde behavior. Same status for empty-input deserialization (`---\n---\n` → `Ok("")` → step 4).

**Apply protocol:** the `type-mismatch-name` fixture and its test are written and run **first**, before `read` exists, so the answer arrives at RED rather than in T4. If `serde_norway` panics rather than erroring, note the architectural consequence carefully: **the guard cannot live in `frontmatter.rs`**, because guarding requires a two-stage parse through `serde_norway::Value` and that module is forbidden from importing the crate (§11). The fix would belong in `yaml.rs` — a change to a T1-shipped seam with its own pinned behavior tests, which escalates back to the orchestrator rather than being applied silently.

## 11. Decision: enforce the `yaml.rs` single-import invariant mechanically

`yaml.rs:1-9` declares itself the only module allowed to import `serde_norway`. Today that is documentation with no check. T3 is the exact moment it becomes breakable — the first sibling module that parses YAML — and every later phase (T4–T7) adds another chance to break it.

**Decision: enforce it, with a Rust test, not a CI grep.** `crates/vertice-core/tests/yaml_seam_invariant.rs` walks `concat!(env!("CARGO_MANIFEST_DIR"), "/src")` and asserts that no `.rs` file other than `yaml.rs` contains `use serde_norway` or `serde_norway::`.

Why not the alternatives, concretely:

- **`cargo deny`**: bans operate on the *crate* dependency graph. It cannot express "only this module may import this crate". Not available.
- **`clippy.toml` `disallowed-types`**: crate-wide, with no per-module scoping. Not available.
- **A CI `grep` step**: works, but the `quality` job is Ubuntu-only, and — decisively — the proposal's locked success criterion states T3 passes "with no dependency, `deny.toml`, or CI-workflow change". A CI step would violate it. The test is the only option consistent with the locked proposal, and it also runs locally and on all three legs.

Honest caveats: it is a textual check. It can be fooled (a re-export alias, a macro), and it would flag a *doc comment* that writes a path-form occurrence. Both are acceptable — the target is accidental breakage, not an adversary, and false positives are loud and cheap. Consequence to respect at apply: `frontmatter.rs`'s module doc must state the constraint in prose (`MUST NOT import the YAML crate directly`) **without** writing `serde_norway::` or `use serde_norway`. The test's failure message says so.

## 12. File Changes

| File | Action | Description |
|---|---|---|
| `crates/vertice-core/src/frontmatter.rs` | Create | `read`, private `split`, `FenceError`, `SkillFrontmatter`, in-module `#[cfg(test)]` splitter units |
| `crates/vertice-core/src/lib.rs` | Modify | one line: `pub mod frontmatter;` |
| `crates/vertice-core/tests/frontmatter_reader.rs` | Create | fixture-driven behavior tests + byte tripwire + generic-reuse probe |
| `crates/vertice-core/tests/yaml_seam_invariant.rs` | Create | §11 single-import check |
| `crates/vertice-core/tests/fixtures/frontmatter/<10 cases>/SKILL.md` | Create | first fixtures in the repository |
| `.gitattributes` | Create | §9, two scoped rules |
| `openspec/specs/frontmatter-reader/spec.md` | Create | capability spec (parallel `sdd-spec` agent) |
| `crates/vertice-core/src/{model/,yaml.rs}`, `Cargo.toml`, `deny.toml`, `.github/workflows/ci.yml`, `vertice-app`, `frontend/` | **Unchanged** | zero new dependencies, zero IPC surface, zero binding regeneration |

## 13. Testing Strategy

`strict_tdd: true`. Test command: `cargo test`. Core-only — no `npm run test` leg is exercised by T3.

| Layer | What | How |
|---|---|---|
| Unit | `split`: opening fence, closing fence, `Empty`, `NoOpeningFence`, `Unterminated`, CRLF fence, empty block, fence not on line 1 | `#[cfg(test)]` inside `frontmatter.rs`, in-memory `&str`, zero disk (precedent: `model/identity.rs`) |
| Integration | CA-10 (folded description complete and correct); CA-12-partial (corrupt YAML → `path: Some`, `reason` carries the failure); one test per failure class asserting the exact `severity` from §5; `valid-no-description` → `Ok` with `None`; non-UTF-8 content → `path: Some`, never `None` | `tests/frontmatter_reader.rs` over the ten fixtures, plus a non-existent repository-relative path for the I/O-failure class (no file on disk) |
| Contract | Generic reuse: a second, non-skill target type reads `valid-folded-description` | same file, `LicenseProbe`-style local struct — proves T5's path with no refactor |
| Invariant | No `serde_norway` import outside `yaml.rs`; no regex anywhere in the new module | `tests/yaml_seam_invariant.rs`; the regex half is trivially true — no regex crate is a dependency, so it cannot compile |
| Tripwire | Non-UTF-8 fixture is still non-UTF-8 and still N bytes on disk | `non_utf8_fixture_is_still_non_utf8_on_disk` (§9) |

**Non-panic guarantee**: it is the sum of the integration rows, not a separate test. Every fixture returns a value; a panic in any arm fails its own test with an unambiguous name. There is no `unwrap`, `expect`, `panic!`, or indexed slice anywhere in `frontmatter.rs`.

**Read-only invariant (CA-16)**: T3 calls `std::fs::read` only. No `File::create`, no `OpenOptions::write`, no `fs::write` — `rules.apply`'s grep will find nothing.

## 14. Migration / Rollout

No migration. Purely additive; no consumer exists yet. Rollback is the proposal's plan: delete the module, its two test files, and `tests/fixtures/`, revert one `pub mod` line, and drop `.gitattributes`. `yaml.rs` and `model/` are read-only inputs and are untouched by the revert.

## 15. Open Questions

- [ ] **`serde_norway` type-mismatch behavior** (§10) — assumed, not verified. The `type-mismatch-name` fixture runs first at RED. If it panics, the fix belongs in `yaml.rs` and escalates to the orchestrator; it does **not** get patched inside `frontmatter.rs`.
- [ ] **UTF-8 BOM** — a leading `\u{FEFF}` makes line 1 `"\u{FEFF}---"`, so a BOM-prefixed file falls into `NoOpeningFence` and yields a `Warning`. Graceful, non-panicking, but arguably a false negative on a Windows-authored file. **Deliberately not handled in T3**: the fixture set is locked at ten, and strict TDD forbids shipping the untested branch. Flagged for T4 (which sees real trees) and T16 (real-machine validation) to fixture and fix if it occurs in practice.
- [ ] **Severity escalation by callers** (§5) — T3 emits the caller-agnostic floor. Whether T4 actually escalates an I/O `Warning` on a discovered `SKILL.md` to `Error` is T4's decision, not pre-decided here.
- [x] Non-UTF-8 **content** carries `path: Some(path)`; non-UTF-8 **path** (`path: None`) is T4's — settled in §7.
- [x] `reason` is a developer diagnostic, not localized copy; T12 has no T3 strings — settled in §6.
- [x] Fixture layout separates addressed files from walked trees; `fixtures/roots/` is reserved for T4 — settled in §9.
