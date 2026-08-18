# Design: Skill Scanner over User Roots

> Trace: **T4** (`internal-docs/plan-desarrollo-poc.md:110-128`) / closes CA-6, CA-8 (partial), CA-9, CA-14; contributes to CA-12; bound by CA-16 and CA-17.
> Proposal: `openspec/changes/skill-scanner-user-roots/proposal.md`. Inherits T3's design (`openspec/changes/archive/2026-08-18-skill-frontmatter-reader/design.md`, hereafter **T3D**).
> `rules.design` coverage: core data model change (§2), core/Tauri isolation for the CLI pathway (§1), per-OS paths (§9), `ScanIssue` taxonomy and error paths (§7), IPC contract surface (§2.3).
> **Environment note, updated at apply time**: `cargo` was not on PATH in the authoring environment, so every dependency/MSRV/toolchain claim in §3 was originally recorded as unverified. It has since been verified locally during `sdd-apply` with `cargo 1.97.1`/`rustc 1.97.1`: `cargo fmt --all --check` and `cargo clippy --workspace --all-targets -- -D warnings` are clean (the `std::env::home_dir()` claim holds — no deprecation warning at this toolchain), `cargo test --workspace --locked` is fully green, and `PATH="$HOME/.cargo/bin:$PATH" cargo deny check bans licenses` reports `bans ok, licenses ok` (the `walkdir` `Unlicense OR MIT` question resolves via `MIT`, no `deny.toml` change needed).

## 1. Technical Approach

Two new sibling modules of `model/`, and one model field.

```
                                     vertice-core
 frontend ──IPC──> vertice-app ──>   ├── model/        (pure data, zero I/O)  ← §2 adds ONE field
                                     ├── roots         (std::env, std::fs::symlink_metadata — NEW)
 future vertice-cli ────────────>    ├── skills        (walk + Component assembly — NEW)
                                     ├── frontmatter   (T3, consumed unchanged)
                                     └── yaml          (serde_norway seam, untouched)

 roots::home_dir() ─> PathBuf ─> roots::skill_roots(home) ─> [ResolvedRoot; 3]
                                            │
                          skills::scan(home) ┴─> walk ─> frontmatter::read ─> Component
                                                  └────────────────────────> ScanIssue
```

**Why two modules and not the proposal's single `roots`.** This refines the proposal's working name; it does not reverse its decision. Root resolution still lives in a new sibling of `model/`, exactly as agreed. But T3D §2 fixed the crate's naming rule — *modules are named after the thing, never the role* — and "search roots" and "skills" are two things. Decisively, **T5 needs `roots::home_dir()` for `~/.claude/agents/` and must not import a module called `skills`** to get it. Splitting now costs one file; splitting at T5 costs a rename across two suites.

**The CLI pathway is preserved unchanged.** Neither module knows what a Tauri command is. `skills::scan` takes a `&Path` (the home directory) and returns owned data; the only ambient-environment read in the whole change is `roots::home_dir()`, isolated in one function so both binaries — and every test — bypass it by passing a path.

**`model/` purity survives.** `model/mod.rs:8-15` forbids `std::fs`, `std::io`, `std::env`. §2 adds a plain enum and a field; it adds **no import** to `model/`. All environment and disk access lives in the new siblings, exactly as `frontmatter.rs` did in T3 (T3D §1).

## 2. Decision: the `SearchRoot` model change (CA-9)

### 2.1 What actually needs representing

Three states are required: *absent*, *present-and-empty*, *present-with-entries*. Only **two** of them are missing from the model. "Present-with-entries" is already derivable with zero model change — a root has entries iff some `Component.locations[].root` equals its `SearchRootId` (`location.rs:16-18`). So the model must carry exactly one new bit: **did the scan find this directory on disk?**

### 2.2 Shape

| Option | Consequence | Decision |
|---|---|---|
| `pub exists: bool` on `SearchRoot` | Smallest diff, no new binding file. But `exists` is a present-tense claim the model cannot honour after serialization (TOCTOU), and a bool cannot grow a third state without a breaking TS change | Rejected |
| **`pub status: SearchRootStatus` — closed enum `{ Found, NotFound }`** | One new field, one new binding file. Matches the exact precedent of `LocationOrigin` (`location.rs:22-32`), which is an enum rather than a bool for the same reason. Serializes as `"found"` / `"notFound"` — self-describing at the IPC boundary | **Chosen** |
| Wrapper type `RootScan { root, status }` | Changes `ScanReport::roots_scanned`'s element type (`report.rs:23`), which ripples into T9's report assembly and every future consumer, to express one bit | Rejected |
| A `ScanIssue` for an absent root | Forbidden by the plan (`plan-desarrollo-poc.md:118`) and by `error.rs:3-7`, which already names "an absent root" as a `ScanIssue` case — *for issues that have no path*, not as a licence to report absence as a problem. A client the user never installed is a fact, not a failure | Rejected |

`status` is a **statement about what this scan observed**, which is why the enum names read `Found`/`NotFound` and not `Exists`/`Missing`. It is deliberately two-valued: a root that exists but cannot be read is `Found` **plus** a `ScanIssue` (§7), not a third status.

No client display label is added — settled by the product owner and recorded in the proposal. `SearchRootKind` plus `path` is everything the UI needs; naming the client is T11's, from the id.

### 2.3 IPC contract surface / bindings delta

Exactly two files change under `frontend/src/bindings/`, both regenerated by `cargo test -p vertice-core`, never hand-edited:

```ts
// SearchRootStatus.ts — NEW
export type SearchRootStatus = "found" | "notFound";

// SearchRoot.ts — MODIFIED (one field, one import)
export type SearchRoot = { id: SearchRootId, path: string, kind: SearchRootKind, status: SearchRootStatus, };
```

No Tauri command, no event, no frontend source change: nothing consumes `SearchRoot` yet, so this shape change has zero call sites to break. That window closes at T10. CI's bindings-drift gate must be green in the same PR as the Rust edit.

## 3. Decision: dependencies

**Recommendation: `std::env::home_dir()` from std for home resolution, and `walkdir 2.5.0` for the walk.** Not `dirs`.

| Need | Options | Decision |
|---|---|---|
| Home directory | `dirs 6.0.0` / hand-rolled `env::var("USERPROFILE"\|"HOME")` / **`std::env::home_dir()`** | **std** |
| Recursive walk | **`walkdir 2.5.0`** / hand-rolled recursive `read_dir` | **`walkdir`** |

**Why std over `dirs` for home.** Three reasons, in order of weight. (1) **It structurally removes the trap this whole task exists to avoid.** `dirs` exposes `config_dir()`, which returns `%APPDATA%` on Windows and would find zero skills (`alcance-poc-vertice.md:106`). Adopting `dirs` would mean adopting a policy — "only `home_dir` may be called" — enforced by a convention test. std offers no `config_dir` at all, so the wrong API is simply not reachable. Removing a hazard beats guarding it. (2) Zero added crates (`dirs` → `dirs-sys` → `option-ext`). (3) Identical signature, `Option<PathBuf>`, so the fallback is a one-line swap.

**The claim, now verified by compiling.** `std::env::home_dir()` was deprecated for years because of wrong Windows behaviour; the Windows implementation was corrected in Rust **1.85.0** and the deprecation was lifted in Rust **1.87.0** (rust-lang/rust#137327). This workspace's MSRV floor is **1.88** (`Cargo.toml:8`, `.github/workflows/ci.yml:22`, `rust-toolchain.toml` pins 1.97.1), so the call is available and non-deprecated across the whole supported range. *Verified against upstream release notes on 2026-08-18, and confirmed by compiling during `sdd-apply`*: `cargo clippy --workspace --all-targets -- -D warnings` at rustc 1.97.1 is clean with `roots::home_dir` calling `std::env::home_dir()` directly — no deprecation warning fires. The `dirs = "6"` fallback described below was not needed.

**Why `walkdir` and not a hand-rolled walk.** T3's hand-rolling precedent does not transfer: T3 wrote a 20-line line-splitter *to avoid pulling a regex crate* — trivial code replacing a heavy dependency. Here the code is not trivial. A correct recursive walker needs an explicit work stack (naïve recursion blows the stack on a pathological tree), a per-entry `Result` so one unreadable subdirectory does not abort the walk, an explicit no-follow-symlinks policy, and deterministic ordering. Those are four silent-defect classes for a team new to Rust, against ~60 lines saved. `walkdir` supplies `follow_links(false)`, `sort_by_file_name()`, and per-entry `Result` directly.

**Supply-chain position — verified locally.** `dirs 6.0.0`, `dirs-sys 0.5.0`, `option-ext 0.2.0`, `walkdir 2.5.0`, `same-file 1.0.6`, and `winapi-util 0.1.11` were already present in `Cargo.lock` before this change. Promoting `walkdir` to a direct dependency of `vertice-core` therefore adds **no new crate to the resolved graph**. `deny.toml:46-52` bans only `tauri`/`tauri-build`, so no ban is touched. Licenses: `walkdir`/`same-file`/`winapi-util` publish `Unlicense OR MIT`; `Unlicense` is **not** in `deny.toml:55-66`, and cargo-deny satisfies the `OR` expression via the already-allow-listed `MIT` half — confirmed by running `PATH="$HOME/.cargo/bin:$PATH" cargo deny check bans licenses` locally during `sdd-apply` after adding `walkdir` as a direct dependency: `bans ok, licenses ok`, with no `deny.toml` change required.

**Rejected: hand-rolling both.** It is the only option with zero CI risk, and it is recorded as the standing fallback for either half independently. It is not the recommendation because it trades a verifiable, gated, already-present dependency for code whose failure modes are silent.

## 4. Module and function surface

```rust
// crates/vertice-core/src/roots.rs

/// The ONLY ambient-environment read in the crate. Every other function
/// takes `home` as a parameter, which is what makes fixtures possible.
pub fn home_dir() -> Result<PathBuf, ScanError>;

/// Exactly three roots — the array length is the CA-6/CA-14 guarantee,
/// expressed in the type rather than asserted in prose.
pub fn skill_roots(home: &Path) -> [ResolvedRoot; 3];

pub struct ResolvedRoot {
    pub root: SearchRoot,           // canonical identity, path, kind, status
    pub scan_paths: Vec<PathBuf>,   // 1 for .claude/.agents; 2 for opencode (skills + skill)
}
```

```rust
// crates/vertice-core/src/skills.rs

/// Owned result, not a tuple. Three heterogeneous collections make
/// `(Vec<SearchRoot>, Vec<Component>, Vec<ScanIssue>)` unreadable at every
/// call site and re-orderable by mistake. NOT a model type: no `Serialize`,
/// no `TS`, so it emits no binding — the same status `SkillFrontmatter` has
/// (T3D §2). T9 destructures it into `ScanReport`.
pub struct SkillScan {
    pub roots: Vec<SearchRoot>,
    pub components: Vec<Component>,
    pub issues: Vec<ScanIssue>,
}

pub fn scan(home: &Path) -> SkillScan;
```

`lib.rs` gains two plain lines, `pub mod roots;` and `pub mod skills;`, with no crate-root re-export — matching `lib.rs:7-9`.

**Root ids are hardcoded, never derived from the path**: `SearchRootId("claude-skills")`, `SearchRootId("agents-skills")`, `SearchRootId("opencode-skills")`. A path-derived id would embed the username and make every fixture assertion machine-dependent, violating design principle 4 and `rules.verify`.

**Alias policy.** `~/.config/opencode/skills/` is the **canonical** path carried by the `SearchRoot`; `~/.config/opencode/skill/` is a second entry in `scan_paths` under the *same* id (`plan-desarrollo-poc.md:116`). `status` is `Found` if **either** directory exists. Honest wart: in the unobserved case where only the singular exists, the reported `path` is the plural form that is not on disk. Accepted for the PoC — `SearchRoot` is a grouping identity, not a display-only path — and flagged for T11 in §11.

**Component assembly**, reusing T3 and T2 unchanged:

```rust
let fm: SkillFrontmatter = frontmatter::read(&path).map_err(escalate)?;   // §5
Component {
    id: ComponentId::derive(ComponentKind::Skill, &fm.name),
    name: fm.name,
    kind: ComponentKind::Skill,
    description: fm.description,
    scope: Scope::User,                                   // CA-14, the only value ever constructed
    locations: vec![Location { path: Some(path), root: root_id.clone(), origin: LocationOrigin::File }],
    provenance_hint: None,
}
```

Two consequences to state rather than discover later. **The frontmatter `name` wins over the directory name** — it is the declared identity, and `ComponentId::derive` consumes a name, not a path. So two directories declaring the same `name` inside one root produce two entries sharing one id; T4 emits both, T8 consolidates. **`provenance_hint: None`**: `component.rs:26-31` forbids branching on it, so filling it with the root name would duplicate `Location::root` as un-actionable display text.

## 5. Decision: severity escalation (T3D §5/§15 open item, resolved)

> **T4 escalates every `ScanIssue` returned by `frontmatter::read` for a discovered `SKILL.md` to `IssueSeverity::Error`, uniformly. `path` and `reason` are untouched.**

One function, `fn escalate(issue: ScanIssue) -> ScanIssue`, one invariant, directly testable.

**Rationale: the severity rule must agree with the detection rule.** T4's detection rule is verbatim *if there is a `SKILL.md`, it is a skill* (`plan-desarrollo-poc.md:117`). Under that rule there is no such thing as a stray `SKILL.md` beneath a skills root — every one of them is a declared skill. So every failure to parse one means the user has a skill on disk that is missing from their inventory. That is precisely the caller-context knowledge T3D §5 said the leaf reader lacked, and T3D §111 wrote the forward contract for.

**Alternative considered: escalate only the I/O class** (the objection T3D §5 actually named), leaving "empty file" and "no fence" as `Warning`. Rejected: it makes severity a two-predicate function with no user-visible payoff. A user looking at a `SKILL.md` that is empty and one whose YAML is corrupt has the identical action available — fix the file. Splitting them buys a distinction nobody can act on, at the cost of an invariant nobody can state in one line.

**Cost, stated honestly**: after escalation, `severity` no longer discriminates failure class. It never did the job well — `reason` carries the class (T3D §6), and severity's real job is triage: *did the user lose an entry?* Beneath a skills root the answer is always yes. T3's rule is untouched; T3 remains the caller-agnostic floor.

**Consequence for the deferred BOM case** (T3D §15, deferred to T16): a BOM-prefixed `SKILL.md` falls into `NoOpeningFence`, a `Warning` at the leaf, and now surfaces as an **`Error`** in the report. Still skipped, still no component, still deferred — but louder, which improves T16's chance of catching it on a real machine. Recording this so the severity change is a known state, not a surprise.

## 6. Walk policy

| Question | Decision | Why |
|---|---|---|
| Depth | **Recursive**, unbounded | OpenCode's own glob is `{skill,skills}/**/SKILL.md` (`alcance-poc-vertice.md:87`). All 69 real files sit at depth 1; implementing depth-1 would fit the code to one observation instead of the client's documented behaviour |
| Symlinks | **Do not follow.** `follow_links(false)` written explicitly with a comment, never left to the crate default | Zero symlinks exist under any of the three roots on the reference machine (verified 2026-08-18). Following them creates cycle risk and manufactures duplicate entries at physical paths that do not exist — which T8 would then have to un-duplicate. Consequence accepted: a symlinked skill directory is seen as an entry and not descended into, so its `SKILL.md` is not found. **Unverified**: whether Windows directory junctions are reported as symlinks by `std`; flagged in §11 |
| Detection | `entry.file_name() == "SKILL.md"`, files only | Verbatim from the plan. No name-convention heuristic anywhere, so `_shared` enters as an ordinary skill (**CA-8 partial**). Non-`SKILL.md` files are ignored silently — never an issue |
| Ordering | `sort_by_file_name()` | Determinism for debugging and diffs. Assertions are still written order-independently (sorted sets), so correctness does not depend on it |
| Plugin exclusion | **None written** (**CA-6**) | Structural: plugin skills live outside all three roots, and no `~/.claude/plugins/` exists on the reference machine. A filter would be code defending a case it cannot reproduce. Asserted by a decoy fixture (§8), revalidated at T16 |
| Project exclusion | **None written** (**CA-14**) | Structural: all three roots are home-relative and `Scope::User` is the only value constructed. Asserted by a decoy fixture, not by a filter |

## 7. Error paths: `ScanIssue` taxonomy

**No new `ScanIssue` variant, no new field, no `ScanIssueKind`.** `ScanIssue` is `{ severity, path, reason }` (`report.rs:36-42`) and every traversal failure below has exactly a severity, an optional path, and a diagnostic string. The `ScanIssueKind` enum T3D §6 sketched was explicitly deferred as the *correct* future fix for localized copy; introducing it here would regenerate bindings for a consumer that does not exist until T11.

| Failure | Root `status` | `severity` | `path` | `reason` shape | Walk continues? |
|---|---|---|---|---|---|
| Root probe → `ErrorKind::NotFound` | `NotFound` | *no issue* | — | — | yes, other roots |
| Root probe → any other `io::Error` (permission) | `Found` | `Error` | `Some(root)` | `could not inspect search root: {io}` | yes, other roots |
| Root path exists but is not a directory | `Found` | `Error` | `Some(root)` | `search root is not a directory` | yes, other roots |
| Entry-level walk error mid-tree (unreadable subdirectory) | `Found` | `Error` | `Some(entry)`, else `Some(root)` | `could not read directory entry: {io}` | **yes**, same root |
| Discovered path not representable as UTF-8 | `Found` | `Error` | **`None`** | `skipped a file whose path is not valid UTF-8: {lossy}` | yes (§8 / below) |
| Any `frontmatter::read` failure on a `SKILL.md` | `Found` | `Error` (escalated, §5) | `Some(file)` | verbatim from T3D §7 | **yes** — CA-12 |
| Home directory unresolvable | — | *not a `ScanIssue`* | — | — | **no** — §7.2 |

**Why `status: Found` on a permission failure.** `NotFound` is a positive claim about the machine. When the probe fails for any reason other than `NotFound`, absence cannot be claimed, so the safest report is "we tried, and something went wrong" — which is exactly what an issue is for.

### 7.1 Non-UTF-8 paths (T2's `path: None` becomes reachable)

T4 is the first module that discovers paths from disk, so T2's contract finally has a caller. `PathBuf` serialization fails outright on a path that is not valid UTF-8, so such a path can never reach the frontend.

**Decision: skip the file, emit `ScanIssue { severity: Error, path: None, reason: "... {to_string_lossy()}" }`, and never emit a `Component`.** The rejected alternative — emitting `Location { path: None, origin: File }` — violates the invariant documented at `location.rs:27-28` (`File` implies `path` is `Some`) and would put an un-serializable report one field away. `to_string_lossy()` in `reason` gives the user a fighting chance to find the file; the reason is a developer diagnostic that must not be parsed (T3D §6).

**Distinguish this from T3's case, which is the easiest thing here to get wrong:** non-UTF-8 **content** carries `path: Some` (T3D §7); non-UTF-8 **path** carries `path: None`. They are different failures.

**A non-UTF-8 *home* is not a per-file case** — it makes all three `SearchRoot.path` values un-serializable, so it is a whole-scan failure and is handled in §7.2.

**Testability, honestly**: no portable fixture can produce this. On Unix it is unit-testable via `std::os::unix::ffi::OsStrExt::from_bytes` behind `#[cfg(unix)]`; on Windows only unpaired surrogates qualify and the Windows CI leg **cannot** exercise it. The behaviour is therefore pinned by a `#[cfg(unix)]` unit test on the path-conversion helper, and stated in the module doc.

### 7.2 Home-directory resolution failure

`home_dir()` returning `None` — or returning a path that is not representable as UTF-8 — means **not one root can be constructed**. This is a scan-level failure, not a per-root one, and `error.rs:3-7` reserves `ScanError` for exactly that.

```rust
pub fn home_dir() -> Result<PathBuf, ScanError>   // Err(ScanError::Internal { reason })
```

`ScanError::NoRootsConfigured` is the wrong variant: roots *are* configured — they are hardcoded — and the failure is that the anchor they hang from is missing. `Internal { reason }` carries owned `String` text as `error.rs:9-13` requires. **No new `ScanError` variant** is added: a variant would regenerate `ScanError.ts` for a case T9 handles identically to any other `Err`.

`skills::scan(home)` itself is infallible — it takes an already-resolved `home` and returns `SkillScan`. That is what keeps the failure mode in exactly one place and out of every test.

## 8. Fixture architecture

**The seam that makes this testable is `home` as a parameter.** `roots::home_dir()` is the only function that reads the environment, and nothing else calls it; `skill_roots(home)` and `scan(home)` take a path. Tests pass a fixture directory as a synthetic home. **No test ever reads the author's machine, and no environment variable is set or read by any test** — no `std::env::set_var`, which is unsound under parallel test execution anyway.

```
crates/vertice-core/tests/fixtures/roots/          # reserved by T3D §9; T4 fills it
├── absent-roots/            .gitkeep only                      → 3× NotFound, 0 issues, 0 components
├── empty-alias/             .config/opencode/skill/.gitkeep    → CA-9: Found, 0 issues, 0 components
├── alias-populated/         .config/opencode/skill/demo/SKILL.md   → alias entries carry the plural root's id
├── underscore-shared/       .claude/skills/_shared/SKILL.md    → CA-8 partial
├── nested-skill/            .claude/skills/group/nested/SKILL.md   → recursion, depth 2
├── unreadable-entry/        .claude/skills/{good,broken}/SKILL.md  → CA-12 + §5 escalation
├── project-decoy/           .claude/skills/real/SKILL.md
│                          + projects/demo/.claude/skills/fake/SKILL.md   → CA-14
├── plugin-decoy/            .claude/plugins/p/skills/x/SKILL.md → CA-6, asserted not claimed
└── reference/               the 69-entry tree (tier 2)
```

**Every top-level directory is a synthetic home**, one per semantic case. This is T3D §9's `fixtures/frontmatter/<case>/` pattern lifted one level, and it exists for the same reason: a single shared tree would make every assertion depend on the whole fixture set, so adding a case would break unrelated tests.

**Tier 2, `reference/`**, reproduces the exact recorded distribution (`alcance-poc-vertice.md:57-59, 79-81`): 22 names present in all three roots, 1 name only in `.claude/skills/`, 2 names only in `.agents/skills/`. That is 23 / 24 / 22 per root, **69** entries, **25** unique names. Content is generated by rule — four lines, `name` plus a one-line `description` — so a reviewer verifies *the rule and the distribution*, not 69 diffs. The test asserts `components.len() == 69`; it additionally asserts 25 distinct `ComponentId`s as a non-binding corroborator that hands T8 a pre-validated fixture.

**`fixtures/frontmatter/` is never walked.** T3D §9's inherited rule stands. The corrupt file in `unreadable-entry/` is a deliberate **copy**, not a reference — coupling T4's counts to T3's fixture count is exactly what that rule forbids.

**The `.gitkeep` trap, and its tripwire.** Git cannot track an empty directory, so the CA-9 case needs a `.gitkeep`. That is safe for the walk — detection matches `file_name() == "SKILL.md"`, so `.gitkeep` is invisible — but it creates a genuine silent failure: **if the `.gitkeep` is ever lost, the directory vanishes and the "present and empty" test silently becomes the "absent" test, still passing with zero components.** The empty-root test therefore asserts, first and by name, that the directory exists on disk (`empty_alias_fixture_directory_still_exists_on_disk`) and that the resolved status is `Found` — the same tripwire discipline T3D §9 applied to the non-UTF-8 fixture.

`.gitattributes` needs **no change**: line 2 already scopes `-text` to `crates/vertice-core/tests/fixtures/**`, which covers `roots/`. Fixture paths are built from `env!("CARGO_MANIFEST_DIR")` with per-segment `push`, never `"/"`-joined literals (the `frontmatter_reader.rs:19-27` helper is the pattern to copy).

## 9. Per-OS paths

All three roots are `home` + a hardcoded relative suffix. **No OS config-dir convention is consulted anywhere** — `%APPDATA%`, `~/Library/Application Support`, and `$XDG_CONFIG_HOME` are all deliberately unused, because `opencode debug paths` shows OpenCode using XDG layout *on Windows* and Claude Code using `~/.claude` (`alcance-poc-vertice.md:106`). A `config_dir()`-style call would find zero skills on Windows. OS-idiomatic directory logic stays reserved for Vertice's own app-data directory (T14).

| Root | Windows (**verified**, Aug 2026) | macOS (**unverified**) | Linux (**unverified**) |
|---|---|---|---|
| Claude Code skills | `C:\Users\<u>\.claude\skills\` | `/Users/<u>/.claude/skills/` | `/home/<u>/.claude/skills/` |
| Agents skills | `C:\Users\<u>\.agents\skills\` | `/Users/<u>/.agents/skills/` | `/home/<u>/.agents/skills/` |
| OpenCode skills (+ `skill` alias) | `C:\Users\<u>\.config\opencode\skills\` | `/Users/<u>/.config/opencode/skills/` | `/home/<u>/.config/opencode/skills/` |

macOS and Linux are unverified by construction: ground truth is one Windows machine and `alcance-poc-vertice.md:71` states revalidation on the other two platforms is required before the adapters close (T16). Suffixes are built with per-segment `PathBuf::push`, never `"/"`-joined literals, so they are separator-correct on all three CI legs.

## 10. File Changes, Testing, Rollout

| File | Action | Description |
|---|---|---|
| `crates/vertice-core/src/roots.rs` | Create | `home_dir`, `skill_roots`, `ResolvedRoot`, root ids, alias policy |
| `crates/vertice-core/src/skills.rs` | Create | `scan`, `SkillScan`, walk, `escalate`, `Component` assembly |
| `crates/vertice-core/src/lib.rs` | Modify | two `pub mod` lines |
| `crates/vertice-core/src/model/location.rs` | **Modify** | `SearchRootStatus` enum + `SearchRoot::status` field |
| `crates/vertice-core/src/model/mod.rs` | Modify | re-export `SearchRootStatus` |
| `frontend/src/bindings/SearchRoot.ts`, `SearchRootStatus.ts` | Regenerate | §2.3; never hand-edited |
| `crates/vertice-core/Cargo.toml` | Modify | `walkdir = "2"` (plus `dirs = "6"` only on the §3 fallback path) |
| `crates/vertice-core/tests/fixtures/roots/**` | Create | §8, two tiers |
| `crates/vertice-core/tests/skill_scanner.rs` | Create | CA-driven suites |
| `deny.toml` | Possibly modify | only if CI rejects `Unlicense`; §3 |
| `vertice-app`, `frontend/src/**`, `.github/workflows/`, `.gitattributes`, `frontmatter.rs`, `yaml.rs` | **Unchanged** | no IPC, no capability, no gate change |

`strict_tdd: true`. Fixtures and RED tests land before implementation.

| Layer | What | How |
|---|---|---|
| Unit | Alias grouping; root-id stability; `escalate` maps every T3 severity to `Error`; path→UTF-8 conversion (`#[cfg(unix)]`) | `#[cfg(test)]` in `roots.rs`/`skills.rs`, in-memory, zero disk |
| Integration | CA-6, CA-8-partial, CA-9 (present-and-empty **and** absent, distinguished), CA-12-partial, CA-14, the 69 count, recursion at depth 2 | `tests/skill_scanner.rs` over `fixtures/roots/`, one synthetic home per case |
| Tripwire | `empty-alias` directory still exists on disk | §8; named for its own failure |
| Contract | `roots_scanned` always has 3 entries whatever the home; no `Scope::Project`/`Local` is ever constructed | assertions over `SkillScan` |

**Read-only (CA-16), structurally.** The complete disk surface of both new modules is `std::fs::symlink_metadata` (root probe), `walkdir`'s `read_dir`, and T3's `std::fs::read`. There is no `File::create`, `OpenOptions`, `fs::write`, `create_dir*`, or `remove_*` — including in the tests, which read committed fixtures and never materialize a temp tree. `rules.apply`'s grep finds nothing.

**Migration**: none. Rollback is the proposal's plan; the load-bearing part is that reverting `model/location.rs` and regenerating `frontend/src/bindings/` must be **one atomic revert**, or CI goes red on binding drift.

## 11. Open Questions

- [x] **`std::env::home_dir()` availability at MSRV 1.88** (§3) — **resolved**: un-deprecated in Rust 1.87.0, below the 1.88 floor. Confirmed against upstream release notes AND by compiling (`cargo clippy --workspace --all-targets -- -D warnings` clean at rustc 1.97.1). Fallback if it ever regresses: `dirs = "6"` + a `dirs::`-restriction invariant test.
- [x] **`cargo deny check bans licenses` with `walkdir`** (§3) — **resolved**: `Unlicense OR MIT` resolves via `MIT`. Verified locally: `bans ok, licenses ok`, no `deny.toml` change needed.
- [ ] **Windows directory junctions** (§6) — whether `std` reports a reparse point as a symlink, and therefore whether `follow_links(false)` covers junctions, is unverified. No fixture is portable; flagged for T16.
- [ ] **`$XDG_CONFIG_HOME` on Linux** (§9) — if a Linux user overrides it, `~/.config/opencode` may be the wrong path. Whether OpenCode honours the variable is unverified. T16.
- [ ] **Alias-only OpenCode root** (§4) — if only the singular `skill/` exists, the reported `SearchRoot.path` is the plural form that is not on disk. Unobserved; T11 decides whether the UI needs more.
- [ ] **UTF-8 BOM** — deferred to **T16** as agreed. Current state after this change: skipped, no component, reported as an **`Error`** (escalated from T3's `Warning`, §5).
- [ ] **Plugin-root exclusion logic** — deferred to **T16**. Structurally unnecessary today (§6); `plugin-decoy/` asserts it rather than assuming it.
- [x] `SearchRoot` gains `status: SearchRootStatus`, two binding files change — settled in §2.
- [x] T4 escalates every discovered-`SKILL.md` issue to `Error` — settled in §5.
- [x] Non-UTF-8 path → skip + `ScanIssue { path: None }`; non-UTF-8 home → `ScanError` — settled in §7.1/§7.2.
- [x] `skills::scan` returns an owned `SkillScan`, not a tuple — settled in §4.
