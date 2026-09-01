# Design: Detect desktop client installations

> Inherits, unchanged: `archive/2026-08-19-client-installation-detection/design.md` (**T7D**) §5.2 (`cfg!` platform seam), §9 (never merge); `archive/2026-08-23-fix-windows-claude-desktop-probe/design.md` (**P30D**) decision 1 (slot vocabulary); `archive/2026-08-23-report-client-presence-as-status/design.md` (**P32D**) §2 (`ClientPresence`), §3 (`flatten_presence` is the only producer), §7 (`Detected` ≠ "usable"); `archive/2026-08-23-add-codex-client-support/design.md` (**T7CD**) §3.1 (a new `VersionSource` variant gets its own sibling resolver, never a parameterized shared one), §10.2 (no symlink, and no write, in any fixture).
> `rules.design` coverage: core data model (§4), core/Tauri isolation (§1), IPC contract surface (§7), per-OS paths (§4.3), `ScanIssue` taxonomy and error paths (§5), performance (§3).
> Scope guard: this design closes the proposal's four deferred decisions. It writes **no spec and no task**; `openspec/specs/` is owned by the parallel `sdd-spec` phase and is not touched here.

## 0. What is verified, and what is assumed

| # | Statement | Basis |
|---|---|---|
| **V1** | `AppData\Local\Packages\Claude_pzs8sxrjxfjjc` exists on the affected machine, so `ClaudeCodeBundled` resolved `Detected`. H1 is the sole cause of the Claude Code symptom | exploration §2b |
| **V2** | `presenceFor` is `Array.prototype.find` over the group's slots; records are emitted in probe-table order and **every** slot always emits a record. The bundled record is therefore structurally unreachable | `ClientsPage.svelte:50-51`, `installations.rs:82-90` |
| **V3** | The OpenCode desktop install root is `<home>\AppData\Local\Programs\@opencode-aidesktop` (literal `@`), an Electron app whose `resources/` holds `app.asar` (143 MB), `app.asar.unpacked/`, `elevate.exe`, `app-update.yml` | exploration §2b |
| **V4** | `resources/app-update.yml` carries `provider/owner/repo/channel/updaterCacheDirName` and **no version field**. No loose `package.json` belongs to OpenCode itself | exploration §2b |
| **V5** | The first 1200 bytes of the real `app.asar` are a **16-byte binary prefix** followed immediately by plain JSON beginning `{"files":{"node_modules":{`. OpenCode's own `package.json` is inside the archive | exploration §2b, "Version source decision" |
| **V6** | `IssueSeverity` has exactly two variants (`Warning`, `Error`); `jsonc::parse` returns an owned `JsonValue` tree with `BTreeMap` objects; `thiserror` is already a `vertice-core` dependency | `jsonc.rs:11-96`, `model/issue.rs` |
| **V7** | `.gitattributes` already declares `crates/vertice-core/tests/fixtures/** -text`, so fixture bytes are never line-ending-normalized. It does **not** declare them `binary` except for two named files | `.gitattributes:1-6` |
| **M1** | **MEASURED on the affected machine** (archive `143 971 328` bytes): `u32@0 = 4`, `u32@4 = 1 814 496`, `u32@8 = 1 814 492`, `u32@12 = 1 814 486`. The four words are self-consistent under the Pickle layout, and the derived `data_start` is corroborated byte-exactly (§2.2). **A1 is no longer an assumption** | Direct read of the real archive |
| **M2** | The real header JSON is **1 814 486 bytes ≈ 1.73 MiB** — well under `HEADER_MAX_BYTES`, and in the band §3 now has to defend rather than merely refuse | Same read (`u32@12`) |
| **A2** | OpenCode's own `package.json` is a **root-level** entry in the header tree (`files["package.json"]`), not nested and not `unpacked` | §2.4 makes this fail closed, with defense in depth |
| ~~U1~~ | The real header's byte size was unmeasured | **Closed by M2.** The answer changes the design: the ceiling was never the binding constraint — the *cost at the real size* is. §3 is rewritten around that |

**Incidental finding, not this change's scope.** `.gitattributes:6` names `crates/vertice-core/tests/fixtures/installations/...` but the directory is `fixtures/client-installations/...`. The stale line matches nothing; the file is still protected by the `fixtures/** -text` rule on line 2, so nothing is broken today. Worth a one-line fix, but it belongs to a hygiene change, not here.

## 1. Technical approach

Three independent additive slices. Nothing existing is refactored, and no crate dependency is added.

```
                                 vertice-core                        (no tauri; NO new dependency)
 frontend ──IPC──> vertice-app   ├── model/slot        + ClientInstallSlot::OpenCodeDesktop   (§4.1, the ONLY model edit)
 future vertice-cli ────────>    ├── asar              NEW module: archive layout, sole owner  (§2)
                                 ├── jsonc             UNCHANGED — asar parses THROUGH it      (§3.2)
                                 ├── installations     + probe, + VersionSource::AsarPackageJson,
                                 │                     + resolve_opencode_desktop_slot         (§4)
                                 └── scan              UNCHANGED — one more record flows through

 installations::scan_for(home, Windows)
   slot OpenCodeDesktop -> candidate: <home>/AppData/Local/Programs/@opencode-aidesktop
        -> exists? no  -> NotDetected, 0 installations, 0 issues
        -> exists? yes -> Detected, ALWAYS
             -> asar::read_package_version(<root>/resources/app.asar)
                  Ok(v)  -> 1 ClientInstallation { OpenCode, v, path = <root> }
                  Err(e) -> 0 installations + 1 ScanIssue (severity per §5.2)
```

The load-bearing shape: **presence is decided by the directory, the version is decided by the archive, and the two never interact.** `Detected` with zero installations is already a legal, shipped state (`resolve_npm_slot`, P32D §7), so the degradation contract needs no new model vocabulary.

`vertice-core` still imports nothing from `tauri`; `deny.toml` keeps that mechanical. `asar::read_package_version` takes a `&Path` and reads no environment, so a future `vertice-cli` calls exactly what `vertice-app` calls.

## 2. Decision 1 — the asar reader

### 2.1 Where it lives and what it exposes

**New module `crates/vertice-core/src/asar.rs`**, registered as `pub mod asar;` in `lib.rs`. It is **not** under `model/`: it performs `std::fs` I/O, which `model/`'s import allow-list forbids outright. It is also not folded into `installations.rs`, for the same reason `yaml.rs` and `jsonc.rs` exist — the byte layout of a third-party container format is exactly the kind of knowledge that must be containable to one file.

It is a **format module in the house style, but it is not a crate seam**: it wraps no dependency, so there is no `*_seam_invariant.rs` analogue to write. What it does own exclusively is the archive's byte arithmetic.

```rust
//! `app.asar` archive reader — read-only, single-purpose.
//!
//! The ONLY module in `vertice-core` that knows the asar container's byte
//! layout. It extracts exactly one thing: the `version` string of the
//! archive's root `package.json`. It never extracts a file to disk, never
//! enumerates the archive for a caller, and exposes no writer.
//!
//! The header is JSON and is parsed through the `jsonc` seam like every
//! other JSON document in this crate. Every failure is a typed
//! [`AsarError`]; nothing here panics, unwraps an I/O result, or indexes a
//! slice unchecked.

/// The most header bytes this module will read, allocate or parse (§3).
pub const HEADER_MAX_BYTES: u32 = 4 * 1024 * 1024;

/// The most bytes this module will read for one archived entry (§3.4).
pub const ENTRY_MAX_BYTES: u64 = 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum AsarError {
    #[error("could not read the archive: {0}")]
    Io(#[from] std::io::Error),
    /// The 16-byte prefix is not a self-consistent asar prefix, or the
    /// header text is not a JSON object. Carries a fixed discriminator, not
    /// a formatted string, so `ScanIssue` reasons stay stable copy.
    #[error("not a readable asar archive: {0}")]
    Malformed(&'static str),
    /// The DECLARED header length exceeds `HEADER_MAX_BYTES`. Distinct from
    /// `Malformed` because it maps to a different severity (§5.2).
    #[error("asar header declares {declared} bytes, above the {limit}-byte ceiling")]
    HeaderTooLarge { declared: u64, limit: u64 },
    #[error("asar header is not valid UTF-8")]
    HeaderNotUtf8,
    #[error("could not parse the asar header: {0}")]
    HeaderParse(#[from] crate::jsonc::JsoncError),
    #[error("the archive has no usable root package.json entry: {0}")]
    Entry(&'static str),
    #[error("the archive's package.json has no \"version\" string")]
    NoVersion,
}

/// Read the `version` of the archive's root `package.json`. Reads at most
/// 16 + `HEADER_MAX_BYTES` + `ENTRY_MAX_BYTES` bytes and never the whole
/// archive. Read-only: opened with `File::open`, never `OpenOptions`.
pub fn read_package_version(archive: &Path) -> Result<String, AsarError>;
```

That single function is the entire public surface. Deliberately **not** exposed: any `list`/`entries`/`read_file` API, any `AsarHeader` type, any offset arithmetic. A future caller that wants a second field goes through a second named function here — it does not get a general archive reader with which to hand-roll offsets elsewhere.

The four pure, I/O-free helpers below it are private and are where nearly all the test weight lands (§8.1):

```rust
fn parse_prefix(prefix: &[u8; 16], file_len: u64) -> Result<Prefix, AsarError>;
struct Prefix { json_len: u32, json_start: u64, data_start: u64 }

fn locate_package_json(header: &JsonValue, payload_len: u64) -> Result<Entry, AsarError>;
struct Entry { offset: u64, size: u64 }

fn extract_version(package_json: &JsonValue) -> Result<String, AsarError>;
```

`extract_version` is byte-for-byte the same rule as the existing `extract_package_json_version` (present, `JsonValue::String`, non-empty), restated as a `Result` so its failure carries a reason. It is **not** shared with `installations.rs`'s copy: that one returns `Option` and its caller builds a different `ScanIssue`; merging them would couple two slots' failure vocabularies for four saved lines (T7CD §3.1's trade, applied again).

### 2.2 The archive layout, exactly — measured, not assumed

Bytes, little-endian throughout, `u32@n` meaning "the `u32` at byte offset `n`":

```
offset  0 ..  4   u32  pickle_header_size    MUST be 4
offset  4 ..  8   u32  header_len            length of the header pickle buffer
offset  8 .. 12   u32  header_payload_len    header_len minus its own 4-byte prefix
offset 12 .. 16   u32  json_len              length in bytes of the header JSON text
offset 16 .. 16+json_len   the header JSON text (UTF-8)
                           then 0..3 bytes of padding to a 4-byte boundary
offset 8 + header_len ..   the data region ("payload"); entry offsets are
                           relative to THIS position, not to the file start
```

`json_start = 16`. `data_start = 8 + header_len`.

**M1 confirms both, byte-exactly, on the real archive.** The measured words are `u32@0 = 4`, `u32@4 = 1 814 496`, `u32@8 = 1 814 492`, `u32@12 = 1 814 486`, in a file of `143 971 328` bytes. Working the layout forward:

| Derived value | Arithmetic | Result |
|---|---|---|
| `header_payload_len == header_len - 4` | `1 814 492 = 1 814 496 - 4` | ✓ exact |
| JSON text span | `16 .. 16 + 1 814 486` | ends at **1 814 502** |
| `data_start` **by the formula** | `8 + 1 814 496` | **1 814 504** |
| The 2-byte gap | `1 814 502` rounded up to the next multiple of 4 | **1 814 504** ✓ |
| Padding length | `header_payload_len - 4 - json_len = 1 814 492 - 4 - 1 814 486` | **2**, and `2 <= 3` ✓ |

The gap between where the JSON ends and where the formula puts the data region is **exactly** the format's 4-byte alignment padding, independently derived. A wrong reading of `header_len` would not produce that coincidence on a 1.8 MB header. **A1 is therefore a measured invariant, not an assumption.**

> **`data_start` MUST be computed as `8 + header_len`, and NEVER as `json_start + json_len`.**
> On this archive the latter yields `1 814 502` — two bytes short — silently shifting *every* payload read by the padding length. That is precisely the systematic offset error §2.4's defense in depth exists to catch, and it is the single easiest mistake to make here. A unit test pins the two formulas as **different** on a padded fixture, so an implementer who "simplifies" it goes red.

`parse_prefix` returns `Malformed` — never a guess — unless *all* of the following hold. Every step is written as the `checked_*` call the implementation must use; there is no bare subtraction anywhere, and no ordering in which a guard is evaluated after the arithmetic it guards:

1. `u32@0 == 4`
2. `header_payload_len == header_len.checked_sub(4).ok_or(Malformed)?` — the `checked_sub` **is** the `header_len >= 4` guard; it is not a separate precondition, and it fires on `header_len` in `{0,1,2,3}` before any comparison (§8.2 fixture `tiny-header-len`)
3. `json_len >= 2` and `json_len <= header_payload_len.checked_sub(4).ok_or(Malformed)?`
4. `header_payload_len.checked_sub(4).and_then(|n| n.checked_sub(json_len)).ok_or(Malformed)? <= 3` — padding is strictly less than the alignment
5. `json_len <= HEADER_MAX_BYTES`, else `HeaderTooLarge` (§3.3)
6. `data_start = 8u64.checked_add(u64::from(header_len)).ok_or(Malformed)?` and `data_start <= file_len`

All four words are widened with `u64::from`, never `as`, and every combination uses `checked_add`/`checked_sub` on `u64`, so neither a hostile prefix nor a small-value prefix can wrap or underflow into a plausible offset. A file shorter than 16 bytes yields `Io(UnexpectedEof)` from `read_exact`, not a slice panic.

**How wrong offsets fail — a probability argument, not a proof.** The original claim here was that a wrong `data_start` would yield bytes that cannot parse as JSON with a non-empty `version`. **That claim was too strong and is withdrawn.** §2.4 establishes the opposite fact about this archive: the payload is a **dense forest of thousands of small, valid, version-bearing `package.json` documents** from `node_modules`. A systematic offset error shifts every read by the same constant delta, so the realistic bad outcome is not a parse error — it is **silently reporting a native dependency's version as OpenCode's**, in a field the UI renders as fact.

So the honest statement is: with the six checks above, the byte-exact M1 corroboration, and the §2.4 defenses, a wrong version is **low probability, not provably zero**. The mitigation is defense in depth (§2.4), not a single argument — and §8.4's oracle compares the extracted string against OpenCode's own reported version precisely because no static argument closes this.

### 2.3 The read sequence

`File::open` (not `OpenOptions`, so no write capability is ever requested), then:

1. `metadata()?.len()` → `file_len`.
2. `read_exact` into a `[u8; 16]` stack buffer. No heap allocation yet.
3. `parse_prefix(&prefix, file_len)?` → `Prefix`. **All ceilings and cross-checks fire here**, before a single byte is allocated.
4. `Vec::with_capacity(json_len)`, `read_exact` exactly `json_len` bytes from the current position (offset 16 — no seek needed; the file cursor is already there).
5. `String::from_utf8(buf)` — a move, not a copy, so there is exactly one 4 MiB-worst-case buffer, not two. `Err` → `HeaderNotUtf8`.
6. `jsonc::parse(&text)?`. A non-`Object` result → `Malformed("header is not a JSON object")`.
7. `locate_package_json(&header, file_len - data_start)?` → `Entry`.
8. `seek(SeekFrom::Start(data_start + entry.offset))`, `read_exact` exactly `entry.size` bytes.
9. `String::from_utf8` → `jsonc::parse` → `extract_version`.

The archive is opened once and read three times, forward-only apart from one seek. At no point is the 143 MB read, mapped, or hashed. Nothing in this sequence calls `unwrap`, `expect`, `panic!`, or slice-indexes with a computed range.

### 2.4 Locating `package.json` in the header tree

The header shape asar emits:

```json
{"files":{"node_modules":{"files":{ ... }},
          "package.json":{"size":19,"offset":"0"},
          "native.node":{"size":123,"unpacked":true}}}
```

`offset` is a **JSON string** (asar writes it as a string because it can exceed `2^53`); `size` is a **number**; `unpacked: true` marks a file that lives in `app.asar.unpacked/`, not in the archive.

`locate_package_json` looks up **`root["files"]["package.json"]` and nowhere else** (A2). It walks no subtree and searches no other key.

| Option | Consequence | Decision |
|---|---|---|
| Recursively search the tree for any `package.json` | The archive contains thousands of them under `node_modules` (V4 confirms the same for the unpacked tree). The first hit in `BTreeMap` order would be some dependency's version, reported to the user as OpenCode's — a **silently wrong** value in a field the UI shows as fact | **Rejected, emphatically** |
| Try `files["app"]["files"]["package.json"]` as a fallback | electron-builder's `asar` layout for a packaged app puts the app root at the archive root; a fallback here would be speculative generality guarding a layout nobody has observed, and each extra candidate widens the surface on which a wrong entry could be picked | **Rejected** |
| **Root-level `files["package.json"]`, or nothing** | One lookup, one meaning. If A2 is wrong the slot degrades to `Detected` with no version and an `Error` naming the cause — visible, not silent | **Chosen** |

The entry is then validated, again with `checked_*` arithmetic, before it is trusted:

- `unpacked == true` → `Entry("root package.json is unpacked")`. Reading it from `app.asar.unpacked/package.json` is a deliberate non-goal: it re-introduces a second, differently-shaped path to the same value.
- `offset` must be a `JsonValue::String` parsing as `u64` (a `Number` is accepted too, for robustness, via its verbatim source text — the seam keeps numbers as `String`, so both cases are one `str::parse::<u64>()`).
- `size` must be a `JsonValue::Number` parsing as `u64`, with `2 <= size <= ENTRY_MAX_BYTES`.
- `offset + size <= payload_len` where `payload_len = file_len - data_start`.

Any violation → `Entry(&'static str)`. Absent key, or a value that is an object with a `files` key (i.e. `package.json` is a *directory* in the tree) → `Entry("no root package.json entry")`.

#### Defense in depth against a systematic offset error

§2.2 withdrew the claim that a wrong `data_start` cannot yield a plausible version. Because the payload is dense with valid dependency `package.json` documents, **three independent checks** must all pass before an extracted string becomes a `ClientInstallation`. Any one of them failing degrades to `Detected` with no version and an issue, per the §5.2 contract — none of them is a panic and none of them changes the presence verdict.

| # | Check | What it catches |
|---|---|---|
| **D1** | The entry is the **root-level** entry (§2.4's chosen lookup). It is never reached by any subtree walk | A dependency's entry can never be *selected* in the first place |
| **D2** | **Offset plausibility.** The root `package.json` is the archive's own manifest; on the happy path electron-builder places it at `offset: 0`. Require the resolved absolute position `data_start + entry.offset` to be **exactly where the header says the root entry is**, recomputed independently, and treat a resolved offset that does not land inside `[data_start, data_start + payload_len)` as `Entry("root package.json offset is out of the payload")`. A constant shift by the padding length pushes the read off the entry the header actually described | A systematic delta, including the `json_start + json_len` mistake §2.2 forbids |
| **D3** | **Shape plausibility.** The parsed entry must be a JSON object carrying **both** a non-empty `version` string **and** a non-empty `name` string. A bare `{"version":"…"}` blob, or an object with a `version` but no `name`, is rejected as `Entry("package.json is not shaped like a manifest")` | Landing mid-payload on a fragment that happens to parse |

D3 deliberately does **not** assert `name == "opencode"`. The npm package name for the desktop build is not verified (V4 shows the folder name `@opencode-aidesktop` is an electron-builder derivation, not necessarily the manifest `name`), and a hardcoded equality check would turn a harmless upstream rename into a dead probe — trading a rare wrong value for a guaranteed silent failure. Requiring `name` to be *present and non-empty* is the strongest check available that cannot rot. Rejected alternatives: `name` equality (rots), a `version` regex (banned by `AGENTS.md`, and a dependency's version matches it just as well), and a SHA256 integrity verification of the entry (the header carries the block hashes, but hashing would mean reading and digesting the entry with no digest implementation in the dependency set — a new crate, which §3.2 forbids).

The one thing none of these can catch is a delta that lands exactly on another valid, `name`-bearing manifest. That residual is why §8.4's oracle compares against OpenCode's own reported version, and why the risk is recorded as low-probability rather than eliminated.

## 3. Decision 2 — the performance ceilings

### 3.1 What is actually committed to — and the retracted 20 ms

An earlier revision of this design named a **20 ms per-scan budget**. **That number is withdrawn.** It was not enforced by anything: the only runtime signal, `HeaderTooLarge`, fires solely above 4 MiB, while §3.3's own estimate put a 4 MiB header at 70–200 ms — three to ten times the stated budget. A number in a design document that nothing checks and that the document's own arithmetic contradicts is worse than no number, so it is removed rather than restated.

M2 makes the problem concrete rather than hypothetical. **The real header is 1 814 486 bytes (1.73 MiB)** — comfortably under the ceiling, and therefore in what the review correctly named the **silent middle band**: large enough to cost real time, small enough that no mechanism in this design ever mentions it. At the (unmeasured) 20–60 MB/s figure, 1.73 MiB costs roughly **30–90 ms on every scan**, against a whole scan that today takes 12–45 ms. That is not a tail risk; on the affected machine it is the expected case, and it would roughly double to quadruple scan time.

**What this design commits to, and nothing more:**

| Commitment | Enforced by |
|---|---|
| At most `HEADER_MAX_BYTES` of header text is read, allocated and parsed, ever | `parse_prefix` check 5, before any allocation (§2.2) |
| At most `ENTRY_MAX_BYTES` is read for the entry | §2.4 validation |
| At most ~5.25 MiB is read from a 143 MB archive, and the archive is never read whole, mapped or hashed | §2.3's read sequence |
| **Exactly one** `read_package_version` call per scan | one slot, one candidate |
| The cost at the **measured** 1.73 MiB header size | **`BENCH-1` — MEASURED: 30.9 ms average / 43.6 ms worst-of-20 (release build)** |

**`BENCH-1` result, recorded 2026-09-01.** A throwaway `--release` micro-benchmark of `crate::jsonc::parse` against a synthetic 1 814 766-byte header of this exact shape (deeply nested `node_modules` tree, one entry per file, each carrying `size`, a string `offset`, and an `integrity` block with a 64-char SHA256 hex digest and a `blocks` array) measured **30.9 ms average / 43.6 ms worst-of-20** on the development machine. That places the real-size cost in the **~25–100 ms band**: the ceiling is kept (`HEADER_MAX_BYTES = 4 MiB`, unchanged from §3.3), but the real cost is **stated plainly** here and in `client-installation-detector`'s delta spec and `internal-docs/pendientes-desarrollo.md` (entry P17) as a known scan-time regression on machines carrying the OpenCode desktop app, with a follow-up opened for the streaming option (§3.2) rather than reopening it now. The benchmark harness itself was a temporary `crates/vertice-core/examples/bench_asar_header.rs`, deleted after the measurement — never committed, per §3.1's own instruction below.

**`BENCH-1` — required before `HEADER_MAX_BYTES` is locked.** A micro-benchmark of `crate::jsonc::parse` against a **~1.8 MiB synthetic header of this exact shape** (deeply nested objects, one entry per file, each carrying `size`, a string `offset`, and an `integrity` block with a 64-char SHA256 hex digest and a `blocks` array). This is a task for `sdd-tasks`, blocking on the constant, and it exists because the 20–60 MB/s figure was an estimate and the review is right that it could be off by an order of magnitude in either direction: `jsonc.rs:65-96` performs a **full borrowed parse and then a full owned conversion**, with a `BTreeMap::insert` per key and a fresh allocation per string (`into_owned()` on every key and every value). That conversion, not the parse, is plausibly the dominant term, and nobody has measured it.

`BENCH-1`'s outcome decides the constant, on a rule fixed here in advance so the result cannot be rationalised after the fact:

| Measured cost at ~1.8 MiB | Response |
|---|---|
| **≤ ~25 ms** | Keep `HEADER_MAX_BYTES = 4 MiB`. Record the measured figure in the design as the committed bound at the real size, and state the extrapolated 4 MiB worst case honestly |
| **~25–100 ms** | Keep the ceiling but **state the real cost plainly** in the spec and in `internal-docs/pendientes-desarrollo.md` as a known scan-time regression on machines with the desktop app, and open a follow-up for the streaming option |
| **> ~100 ms** | **Stop.** Re-open §3.2: at that cost the hand-written scanner or a streaming dependency is no longer the more expensive option, and shipping a scan that takes a visible fraction of a second is not acceptable. Do not ship by raising the ceiling — the ceiling is not what is hurting |

The benchmark is a development-time measurement, not a committed test: a wall-clock assertion in the suite would be flaky across the three CI legs and on a cold page cache (§3.4's reasoning).

### 3.2 Parsed through `jsonc.rs`, not a new path

**Decision: the header goes through `crate::jsonc::parse`.**

| Option | Consequence | Decision |
|---|---|---|
| A streaming/SAX pass that stops at the first root-level `package.json` key | Best asymptotics — could ignore header size entirely. But `jsonc-parser` exposes no incremental API this crate uses, so it means a hand-written JSON scanner in `asar.rs`. That is the same class of thing `AGENTS.md` already bans for frontmatter ("must not use regex — go through the seam"): a bespoke parser for a format the project has a seam for, whose bugs are silent and whose failure mode is a wrong value | **Rejected** |
| A new dependency (`serde_json` with a streaming reader, `simd-json`, …) | The entire justification for choosing the asar route over the Windows registry was **zero new dependencies**. Adding one here inverts the trade. The instruction on this point is explicit and I am honouring it: no dependency is smuggled in | **Rejected** |
| **`crate::jsonc::parse`, bounded by a hard size ceiling** | Zero new code paths for JSON, zero new dependencies, one constant to defend. Cost is linear in the header text, which the ceiling bounds by construction | **Chosen** |

### 3.3 `HEADER_MAX_BYTES = 4 MiB` (4 194 304), and what the worst case costs

Worst case, when the declared header is exactly at the ceiling:

| Resource | Worst case |
|---|---|
| Bytes read from disk | 16 + 4 194 304 + 1 048 576 ≈ **5.25 MiB** (never the 143 MB archive) |
| Peak heap for the header **text** | one 4 MiB `Vec<u8>`, moved into a `String` by `from_utf8` — **not** doubled |
| Peak heap for the parsed tree | `jsonc-parser`'s borrowed tree plus the seam's owned `BTreeMap` tree. For integrity-heavy asar headers (~250–300 bytes per file entry) 4 MiB of text is roughly 14–16 k entries, so on the order of **20–40 MiB** transient, freed before `read_package_version` returns |
| Bytes read for the entry | ≤ 1 MiB (`ENTRY_MAX_BYTES`), realistically ~2 KiB |

**Why 4 MiB, now that the real size is known.** M2 fixes the real header at 1.73 MiB, so the ceiling's job is no longer to guess — it is to leave **headroom above a measured value** without opening the door to a pathological one. 4 MiB is ~2.3× the measured header, which absorbs plausible growth (OpenCode adding dependencies, or shipping more unbundled files) without the feature silently dying on the next release. Rejected: **2 MiB**, only 1.13× headroom — one dependency bump away from turning the probe off permanently, which is a worse failure than a slow scan because it is invisible. **16 MiB**, whose worst case would be a visible multi-hundred-millisecond to second-long hang. **512 KiB**, which is *below the measured header* and would make the feature dead on the very machine that motivated this change.

Note what the ceiling does and does not buy, now that M2 is in hand: it bounds the pathological case, and it is **not** the mechanism protecting scan time in the expected case. Nothing protects the expected case except the cost being acceptable, which is exactly what `BENCH-1` exists to establish (§3.1).

### 3.4 There is no runtime time guard, and the document no longer pretends otherwise

**Decision: no wall-clock timer, and no abort mechanism.**

Aborting a synchronous parse mid-flight requires either a worker thread with a cancellation channel or an incremental parser with a per-step budget. The first adds a thread and a join per scan and leaks a still-running parse when it times out; the second is §3.2's rejected hand-written scanner. A post-hoc "that took too long" `ScanIssue` was also rejected: it would fire differently on the three CI legs and on a cold page cache, making a fixture-driven suite non-deterministic.

**So the only runtime guard is `HEADER_MAX_BYTES`, and it guards *size*, not *time*.** The previous revision claimed these were equivalent because parse time is a deterministic function of `json_len`. That is true asymptotically and false operationally: a bound of "at most 4 MiB" says nothing useful about a scan whose real input is 1.73 MiB and whose real cost is unmeasured. The size bound is a **refusal threshold for the pathological case**, and it is the whole of the runtime protection. Time is addressed by `BENCH-1` choosing a design, not by a guard choosing a moment to stop.

## 4. Decision 3 — the new slot and its wiring

### 4.1 `model/slot.rs`

```rust
pub enum ClientInstallSlot {
    ClaudeCodeNpm,
    ClaudeCodeBundled,
    OpenCodeNpm,
    OpenCodeDesktop,   // NEW, positioned immediately after OpenCodeNpm
    CodexStandalone,
}

ClientInstallSlot::OpenCodeDesktop => "OpenCode (desktop app)",
```

The variant stays **plain data**: no path, no probe, no I/O attached to it, so `model/`'s import allow-list is untouched and no new import needs auditing.

**Position is load-bearing and is chosen, not incidental.** Placing `OpenCodeDesktop` after `OpenCodeNpm` and before `CodexStandalone` fixes three things at once: the probe-table order, therefore the `ClientPresence` record order, therefore — under the §6 selection rule — which record an OpenCode card shows when *both* slots are detected. It shows **npm**, mirroring Claude Code, where npm also precedes bundled. Appending the variant last (after `CodexStandalone`) was rejected: it would put the desktop record after the Codex record in `scan.installations`, splitting the two OpenCode rows in the scan-route table for no reason.

Label grammar is the house's `{product}[ CLI] ({distribution})`. No `CLI` token, because this distribution is a GUI application and calling it a CLI would be false. `(desktop app)` matches `(bundled in Claude Desktop)`'s register. Rejected: `"OpenCode (Electron)"` (implementation detail, and it would date badly), `"OpenCode Desktop"` (drops the distribution parenthetical, so a future second desktop channel would have no way to differ). The label is core-owned, never localized, rendered verbatim in both locales (P32D §6).

### 4.2 `installations.rs`

```rust
// client()
ClientInstallSlot::OpenCodeNpm | ClientInstallSlot::OpenCodeDesktop => ClientKind::OpenCode,

// version_source()
ClientInstallSlot::OpenCodeDesktop => VersionSource::AsarPackageJson,

// VersionSource
enum VersionSource { PackageJson, DirectoryName, ReleaseDirectoryName, AsarPackageJson }

// resolve_slot
VersionSource::AsarPackageJson =>
    resolve_opencode_desktop_slot(slot, &candidates[0], issues),
```

Both `client()` and `version_source()` are exhaustive matches, so the compiler forces every site. `VersionSource::AsarPackageJson` is a new variant with its own sibling resolver rather than a reuse of `PackageJson`: `PackageJson`'s contract is "`<root>/package.json` is a file on disk", which is false here (V4 — no loose `package.json` belongs to OpenCode), and bending it would put a hidden per-slot branch inside `resolve_npm_slot`. This is T7CD §3.1's decision replayed with the same reasoning.

Probe entry in `windows_install_probes`, inserted **between** the `OpenCodeNpm` push and the `CodexStandalone` push:

```rust
let mut opencode_desktop = home.to_path_buf();
for segment in ["AppData", "Local", "Programs", "@opencode-aidesktop"] {
    opencode_desktop.push(segment);
}
probes.push(InstallProbe { slot: ClientInstallSlot::OpenCodeDesktop, path: opencode_desktop });
```

`home` plus four hardcoded segments, pushed one at a time so it stays separator-correct. No `dirs`/`directories` crate, no environment read, no `%LOCALAPPDATA%` expansion — the standing "Windows Probe Paths Are Hardcoded, Never OS-Convention-Derived" requirement. Exactly one candidate, like the npm slots.

```rust
/// Resolve the OpenCode desktop slot from its single candidate root.
/// Absent root -> `NotDetected`, zero issues (CA-11). Present root ->
/// `Detected`, ALWAYS, whatever the archive does; the version is a best
/// effort and its failure is an issue, never a verdict.
fn resolve_opencode_desktop_slot(
    slot: ClientInstallSlot,
    root: &Path,
    issues: &mut Vec<ScanIssue>,
) -> ClientPresence
```

Shape: `debug_assert_eq!(slot.version_source(), VersionSource::AsarPackageJson)`; `probed_paths = vec![root]`; `if !exists(root) { return NotDetected }`; build `<root>/resources/app.asar` by two `push`es; `match crate::asar::read_package_version(&archive)` → `Ok(version)` pushes one `ClientInstallation`, `Err(err)` pushes one `ScanIssue` per §5.2; return `Detected`.

`ClientInstallation.path` is the **install root** (`…/@opencode-aidesktop`), not the `.asar` file. Rationale: `path` answers "where is this installation", and the install root is what the user recognises and what `probed_paths` already names. Rejected: the archive path, which would make the only OpenCode row in the table point at a 143 MB opaque file, and would differ in shape from every other slot's `path` (all directories today).

### 4.3 `crates/vertice-app/src/freshness/upstream.rs`

```rust
ClientInstallSlot::ClaudeCodeBundled | ClientInstallSlot::OpenCodeDesktop => None,
```

`None`, for the reason the proposal states and this design does not relax: `app-update.yml` names `anomalyco/opencode` (V4), but it is unverified whether the desktop app and the `opencode-ai` CLI share a release-tag namespace. If they do not, wiring `GitHubReleases { owner: "anomalyco", repo: "opencode" }` would compare a desktop version against a CLI tag and paint a **false "outdated"** badge — a wrong claim is worse than `Unknown`, which is already a first-class, non-alarming state in this UI. Deferred and logged in `internal-docs/pendientes-desarrollo.md`.

Consequence, stated so it is not a surprise: the freshness log will now carry "no established queryable upstream" lines for this slot too, exactly as it does for `ClaudeCodeBundled`. That is designed behaviour, not noise.

### 4.4 Paths, by OS

| Purpose | Windows (this change) | macOS / Linux |
|---|---|---|
| OpenCode desktop install | `<home>\AppData\Local\Programs\@opencode-aidesktop\` | **Out of scope.** `HostPlatform::Unsupported` leaves `client_presence: None` entirely (P32D §4), so nothing about OpenCode desktop is claimed off Windows |
| Its version | `…\resources\app.asar` → header → root `package.json` → `version` | — |
| Rejected | `%LOCALAPPDATA%\opencode\`, `%APPDATA%\OpenCode`, `%APPDATA%\opencode` — none exist on the affected machine (V4); the Windows registry — an OS-convention read the spec forbids; PE version resources of `OpenCode.exe` — needs a parser or a new dependency, and Electron shells commonly carry the *shell's* version there, not the app's | |

## 5. Decision 4 — the degradation contract

### 5.1 The invariant

**Presence never depends on version extraction.** Once `<home>/AppData/Local/Programs/@opencode-aidesktop` exists, the record is `Detected` on every path out of the resolver. There is no code path in which an archive problem produces `NotDetected`, an `Err` returned from `scan_for`, an aborted scan, or a panic.

Mechanically, in `asar.rs` and in the resolver: no `unwrap`, no `expect`, no `panic!`, no `todo!`, no bare slice index on a computed range, no `as` narrowing cast, and no arithmetic on offsets that is not `checked_*`. `read_exact` is the only read primitive, so a truncated file is `Err(UnexpectedEof)` rather than a short buffer read as data. This list is a review checklist, not prose.

### 5.2 The taxonomy

| Condition | Status | Installations | Issue severity | `path` | Reason |
|---|---|---|---|---|---|
| Install root absent | `NotDetected` | 0 | **none** — CA-11 | — | — |
| Root present, archive parses, D1+D2+D3 all pass, non-empty `version` | `Detected` | 1 | none | — | — |
| Root present, `resources/app.asar` absent or unreadable | `Detected` | 0 | `Error` | archive | `could not read the OpenCode (desktop app) version: {err}` |
| 16-byte prefix inconsistent / not an asar | `Detected` | 0 | `Error` | archive | same |
| **Declared header above `HEADER_MAX_BYTES`** | `Detected` | 0 | **`Warning`** | archive | `skipped the OpenCode (desktop app) version: {err}` (the `err` names the declared size and the ceiling) |
| Header not UTF-8, or not a JSON object, or malformed JSON | `Detected` | 0 | `Error` | archive | `could not read the OpenCode (desktop app) version: {err}` |
| No root `package.json` entry / unpacked / offset or size out of range | `Detected` | 0 | `Error` | archive | same |
| Entry bytes are not a JSON object | `Detected` | 0 | `Error` | archive | same |
| **D2 fails** — resolved offset outside the payload / not where the root entry was described | `Detected` | 0 | `Error` | archive | same |
| **D3 fails** — parsed object carries no non-empty `name` alongside `version` | `Detected` | 0 | `Error` | archive | same |
| `version` absent, non-string, or empty | `Detected` | 0 | `Error` | archive | same |

The two new D2/D3 rows are `Error`, not `Warning`: unlike the ceiling, they mean *we read something and it did not add up*, which is the same class as a malformed header. They exist to convert a possible silent wrong version into a visible missing one.

**An oversized header is an issue, and it is a `Warning`, not a silent skip.** Both halves of that were decided against alternatives:

| Option | Consequence | Decision |
|---|---|---|
| Silent skip | The user sees "detected, version unavailable" with no explanation and no signal that a ceiling was involved. With M2 now measured the diagnostic value is lower than it was, but it is not zero: the ceiling firing at all would mean OpenCode's header has grown past 4 MiB since the measurement, which is exactly the event the next change needs to know about | **Rejected** |
| `Error` | `Error` in this codebase means "the user's installation is broken or unreadable". An oversized header is not the user's fault or problem; it is Vertice deciding not to spend the time. Grading it `Error` would train users to ignore `Error` rows | **Rejected** |
| **`Warning`, carrying the declared size** | Truthful ("we chose not to look, and here is how big it was"), matches the existing `Warning` for "detection is not implemented on this platform" — also a Vertice-side limitation rather than a user-side defect — and reports the growth event with a number attached rather than as a silent disappearance | **Chosen** |

No new `IssueSeverity` variant; the enum stays at exactly two (V6), and that is a review check. The reason strings are built with `slot.label()`, keeping `label()`'s established dual role (presence label + issue reason prefix). Two distinct verbs — `could not read` for defects, `skipped` for the deliberate ceiling — so the two classes are distinguishable in the log without parsing severity.

### 5.3 CA-16

The disk surface this change adds is `File::open`, `Metadata::len`, `Read::read_exact`, `Seek::seek`, and `symlink_metadata` (via the existing `exists`). There is **no** `File::create`, `OpenOptions`, `fs::write`, `create_dir*`, `remove_*` or `symlink*` in source **or** tests (§8.3 is explicit that fixtures are never generated at test time). `asar.rs` exposes no writer and no extractor, so no caller can acquire a write capability through it. Tauri capabilities in `crates/vertice-app/capabilities/default.json` are untouched.

## 6. Decision 5 — the H1 frontend fix

### 6.1 The new `presenceFor`

```ts
// Records arrive in probe-table order and EVERY slot emits one, so `find`
// over the group returned the first slot's record regardless of status —
// the defect this replaces. Prefer the first Detected record of the group,
// in record order; fall back to the group's first record so a fully
// undetected product still renders a card with real probed paths.
const presenceFor = (slots: readonly ClientInstallSlot[]): ClientPresence | undefined => {
  const group = (report?.clientPresence ?? []).filter((record) => slots.includes(record.slot));
  return group.find((record) => record.status === "detected") ?? group[0];
};
```

`filter` preserves `clientPresence` order, so "first detected in record order" is exactly the probe-table order the core guarantees. The rule is stated over N slots and is proven over N in test (§8.2), not just over the two that exist today — OpenCode becomes a two-slot product in this same change, and Claude Code is likely to become three (H3).

### 6.2 Downstream consumers

The prompt asks which consumers must move. The precise answer is that **none of them need an edit, and that is the point of Option A**:

| Consumer | `ClientsPage.svelte` | Change |
|---|---|---|
| `{@const presence = presenceFor(client.slots)}` | :221 | none — call site is identical |
| `{@const detected = presence?.status === "detected"}` | :222 | **none.** Now reads the selected record, which is the fix |
| `{@const badge = badgeFor(presence)}` | :223 | **none.** `badgeFor` already keys on `presence.slot` (:116) to find the matching freshness check, so it follows the selection automatically. Its `presence.status !== "detected"` guard (:105) is now evaluated against the right record |
| `{@const versions = presence?.installations.map(…)}` | :224 | **none** |
| The detected badge span, the version span | :239-247 | **none** |
| The `clients` table, `openCode` entry | :39 | **`slots: ["openCodeNpm", "openCodeDesktop"]`** — the one other edit in this file |

All four derivations read the **same single selected record**, so the detected badge, the version string and the freshness badge are structurally incapable of disagreeing. That property is the whole reason Option A was chosen over Option C's union: a union would have to reconcile two records' freshness verdicts and installation lists, needing new badge and status vocabulary.

**i18n: zero copy changes, zero new keys.** `scan.clientDetected`, `scan.clientNotDetected`, `scan.clientVersionUnavailable` and every `freshness.*` key already exist and are already used by exactly these expressions. The new slot's label is core-owned English rendered verbatim, never a catalog key (P32D §6). `catalogs.ts` and `locale.test.ts` are byte-identical.

`frontend/src/lib/clientGroups.ts` needs no change either: `CLIENT_ICON.openCode` is keyed on the product, not the slot.

**Known limitation, accepted and recorded.** A machine with *both* an npm and a desktop OpenCode shows only the npm record on the card. The always-visible supported-clients table on the scan route still renders one row per record, so nothing is hidden product-wide. Option C (aggregate the group) remains the better end-state and is deferred, not rejected.

**Observed, out of scope:** `ClientsPage.svelte:38` says `owner: "SST"` while V4 shows the repository is now `anomalyco/opencode`. Not touched here — it is copy, not detection, and changing it would put an unrelated string in this diff. Worth logging alongside the deferred upstream.

## 7. IPC contract surface

No new command, no new event, no capability change. `crates/vertice-app/` changes by exactly one match arm (§4.3) and `capabilities/default.json` is byte-identical. The contract change is inside the existing payload: `ScanReport.clientPresence` gains a fifth record, and `ScanReport.installations` may gain one entry whose `client` is the existing `"openCode"`.

| Binding file | Action |
|---|---|
| `frontend/src/bindings/ClientInstallSlot.ts` | **Regenerated** — four variants become five |
| every other `bindings/*.ts` | **Unchanged** — a diff there means something leaked into `model/` |

Regenerated only by `cargo test -p vertice-core`, never hand-edited, landing in the same commit as the enum. Core and frontend must revert together or `npm run check` fails on the binding — `npm run test` alone would stay green, since vitest does not typecheck.

## 8. Decision 6 — fixtures and testing (`strict_tdd: true` — RED first)

### 8.1 The layering, which is what makes this tractable

A 143 MB archive cannot be committed, and — per T7CD §10.2 and CA-16 — **fixtures are never generated at test time**. The resolution is to put nearly all coverage where no file is needed at all:

| Layer | Where | What it covers | Files on disk |
|---|---|---|---|
| **Pure unit** | `src/asar.rs` `#[cfg(test)] mod tests` | Every prefix cross-check (§2.2 1–6), the ceiling branch, entry location and validation (§2.4), `extract_version` | **none** — built in memory |
| **In-memory end-to-end** | same module | The full byte sequence over a `Vec<u8>` built by a local builder | **none** |
| **File end-to-end** | `tests/client_installations.rs` + committed fixtures | `read_package_version(&Path)`, the seek arithmetic against a real file handle, and the whole slot wiring through `scan_for` | **small committed `app.asar` blobs** |
| **Manual oracle** | §8.4 | That A2 holds and that the extracted version equals OpenCode's own, on the real 143 MB archive (A1 and U1 are already discharged by M1/M2) | — |

The in-memory builder, in `src/asar.rs`'s test module, is the single source of asar bytes in the whole test suite:

```rust
/// Assemble a syntactically valid asar from a header JSON string and a
/// payload. Deliberately the ONLY place in the test suite that writes the
/// 16-byte prefix, so the layout appears exactly twice in the repository:
/// here and in `parse_prefix`.
#[cfg(test)]
fn build_asar(header_json: &str, payload: &[u8]) -> Vec<u8>;
```

**The builder and the reader share one understanding of the format, and nothing in CI ever revisits that.** M1 discharges it *today* — the format is measured, not guessed. What it does not discharge is *tomorrow*: if a future OpenCode release repackages (a different asar version, a v8-snapshot bundle, an unpacked app directory, or simply a moved `package.json`), **the entire suite stays green while the probe is dead on every real machine**. Fixtures cannot detect a change in something they are not a sample of.

There is no cheap mechanical fix — a test against the real 143 MB archive is exactly what the fixture discipline forbids. So this is recorded as an **accepted, monitored risk**, sitting beside the already-accepted `@opencode-aidesktop` rename risk (§11), and written into `internal-docs/pendientes-desarrollo.md` so that the next person who touches OpenCode support finds it rather than rediscovering it. The user-visible degradation is benign in both cases — `Detected` with no version, never a wrong version, never a crash — which is what makes accepting it defensible rather than negligent.

### 8.2 Committed fixture blobs

Under `crates/vertice-core/tests/fixtures/client-installations/opencode-desktop/<case>/AppData/Local/Programs/@opencode-aidesktop/resources/app.asar`.

**They are committed as bytes, not generated.** Generating them into a temp directory at test time was rejected on three grounds: it is a write outside the app data directory (CA-16 would have to be re-argued, as T7CD §10.2 requires for any test-time file creation); temp-directory behaviour differs across the three CI legs and inside the MSIX-redirected shells this project has already been bitten by; and the fixture would then not be reviewable in a diff at all.

Opacity of a committed blob is handled by two devices, both mandatory:

1. **A sidecar `app.asar.layout.txt`** in each fixture's `resources/`, giving the exact byte table (offsets, the four `u32`s in hex and decimal, the header JSON verbatim, the padding length, the payload verbatim). A reviewer reads the sidecar, not the blob.
2. **An integrity test** per fixture: reconstruct the expected bytes with `build_asar` from the sidecar's documented inputs and `assert_eq!` against the committed file. A hand-corrupted, half-line-ending-mangled or truncated blob then fails with a named assertion instead of degrading into "no version" — which is the one failure this suite could otherwise not distinguish from a pass.

**`.gitattributes` gains one line**, mandatory, following the existing precedent on line 5–6:

```
crates/vertice-core/tests/fixtures/client-installations/**/app.asar binary
```

Line 2's `fixtures/** -text` already prevents line-ending normalization, which is the fatal risk (a `u32` word can legitimately contain `0x0D` or `0x0A`); `binary` additionally suppresses diff and merge attempts.

**The happy-path blob, fully specified** — 105 bytes, so the implementer has nothing to compute. The payload carries `name` as well as `version`, because D3 now requires it:

```
header JSON (51 bytes, no whitespace):
  {"files":{"package.json":{"size":37,"offset":"0"}}}
payload (37 bytes):
  {"name":"opencode","version":"0.4.2"}

offset  0: 04 00 00 00   pickle_header_size = 4
offset  4: 3C 00 00 00   header_len         = 60
offset  8: 38 00 00 00   header_payload_len = 56   (= 60 - 4)
offset 12: 33 00 00 00   json_len           = 51
offset 16: the 51 JSON bytes
offset 67: 00            1 padding byte  (56 - 4 - 51 = 1, aligning 51 -> 52)
offset 68: the 37 payload bytes           (= data_start = 8 + 60)
total:     105 bytes
```

Cross-check of the checks in §2.2 against these numbers: `u32@0 == 4` ✓; `56 == 60 - 4` ✓; `51 <= 52` ✓; padding `56 - 4 - 51 = 1 <= 3` ✓; `51 <= 4 MiB` ✓; `data_start = 68 <= 105` ✓; `entry.offset + entry.size = 0 + 37 <= 105 - 68 = 37` ✓.

Note the fixture is deliberately **padded** (padding length 1, not 0). A zero-padding fixture would let the forbidden `data_start = json_start + json_len` formula pass, hiding the §2.2 mistake. Every fixture whose payload is read MUST have a non-zero padding length, and a unit test asserts that the two formulas differ on it.

The fixture set:

| Case | Blob | Expected |
|---|---|---|
| `happy` | the 105 bytes above | `Detected`, 1 installation, version `0.4.2`, `path` = the `@opencode-aidesktop` root, **0 issues** |
| `no-asar` | `resources/.gitkeep`, no archive | `Detected`, 0, 1 `Error` |
| `oversized-header` | **16 bytes only**: `json_len = 5_000_000`, `header_payload_len = 5_000_004`, `header_len = 5_000_008`. Internally consistent, so it reaches the ceiling branch and not `Malformed` | `Detected`, 0, **1 `Warning`** naming 5 000 000 and the ceiling |
| `bad-prefix` | 16 bytes of `0xFF` then junk | `Detected`, 0, 1 `Error` |
| **`tiny-header-len`** | 16 bytes with `u32@0 = 4` and `header_len` in `{0,1,2,3}` — **one fixture per value**, or one fixture plus three unit cases. The small-value boundary at which a bare `header_len - 4` underflows and a bare `as` cast wraps into a huge offset | `Detected`, 0, 1 `Error` (`Malformed`). **Not covered by `bad-prefix`**, which only exercises large `0xFF` words |
| `truncated` | valid prefix declaring a 51-byte header, file ends at byte 40 | `Detected`, 0, 1 `Error` (`UnexpectedEof`) |
| `malformed-header` | valid prefix, header bytes are `not json {{{` padded to length | `Detected`, 0, 1 `Error` |
| `no-package-json-entry` | header `{"files":{"README.md":{"size":2,"offset":"0"}}}` | `Detected`, 0, 1 `Error` |
| `nested-package-json-only` | header where `package.json` exists **only** under `files.node_modules.files`, with a payload carrying version `9.9.9` | `Detected`, 0, 1 `Error` — **and an explicit assertion that `9.9.9` appears nowhere in the report.** The guard against §2.4's rejected recursive search |
| `entry-out-of-range` | root entry with `offset: "9999"` | `Detected`, 0, 1 `Error` (D2) |
| **`shifted-payload`** | Valid root entry at `offset: "0"`, but the payload region begins with a **second, complete, `name`-bearing manifest** (`{"name":"left-pad","version":"9.9.9"}`) placed where the padding-omitting formula would land, and the true root manifest at the correct `data_start`. Reading at `json_start + json_len` yields `9.9.9`; reading at `8 + header_len` yields `0.4.2` | `Detected`, 1 installation, version **`0.4.2`**, and an assertion that **`9.9.9` appears nowhere in the report**. This is the fixture that catches §2.2's forbidden formula, and it is the only one that reproduces the "dense forest of valid manifests" hazard |
| **`no-name-key`** | payload `{"version":"0.4.2"}` — valid, version-bearing, but not manifest-shaped | `Detected`, 0, 1 `Error` (D3). Pins that a bare version blob is refused |
| `no-version-key` | payload `{"name":"opencode"}` | `Detected`, 0, 1 `Error` |
| `empty-version` | payload `{"name":"opencode","version":""}` | `Detected`, 0, 1 `Error` |

Absence needs no new fixture: the existing `nothing` home already lacks `AppData/Local/Programs` entirely and pins `NotDetected` + zero issues.

**Every fixture directory must contain at least one tracked file** — git does not track empty directories, so `resources/` in the `no-asar` case carries a `.gitkeep`, and a dedicated test asserts each fixture path's on-disk existence, exactly as `skill_scanner.rs` already does for `empty-alias`.

`crates/vertice-core/tests/fixtures/scan-orchestrator/` is **untouched**, deliberately: adding a desktop install there would put a binary blob into the tree that backs the CA-16 read-only snapshot and the duration bound, and the per-slot fixtures already prove the end-to-end path. Consequence: the orchestrator's `installations.len()` pins do **not** move.

### 8.3 The pins that move in lockstep

Every `client_presence` / presence-record **count** pin moves 4 → 5; every `installations` count pin is unchanged (§8.2).

| Site | Change |
|---|---|
| `tests/client_installations.rs:51, 182, 415, 939` | `records.len()` 4 → 5; test at :42 renamed `nothing_yields_five_not_detected_records_and_zero_issues` |
| `tests/client_installations.rs:58-67` | the slot-order `vec!` gains `OpenCodeDesktop` between `OpenCodeNpm` and `CodexStandalone` |
| `tests/client_installations.rs:149` | `paths.len(), 4` — **verify before touching**: this pins distinct installation paths in the `isolation` case, which gains no desktop install, so it should stay 4 |
| `src/scan.rs:168` | `client_presence.len()` 4 → 5 |
| `src/scan.rs:111` | `report.installations.len(), 4` — **unchanged** (§8.2) |
| `src/installations.rs:846-901` | the three in-module slot tables (`label`, `client`, `version_source`) gain the new variant |
| `tests/model_contract.rs` | pin `"OpenCode (desktop app)"`; any exhaustive `ClientInstallSlot` match gains the arm |
| `crates/vertice-app/src/freshness/upstream.rs` tests | the `None` arm gains the new slot |
| `openspec/specs/client-installation-detector/spec.md` — requirement body | "exactly four records" → five. **Owned by `sdd-spec`, not written here** |
| `openspec/specs/client-installation-detector/spec.md:5` — **capability prose** | `"four independent probe slots (Claude Code npm, Claude Code desktop, OpenCode npm, Codex standalone)"` → five, naming the OpenCode desktop slot. **A second, distinct site from the requirement body above**, and easy to miss because it is prose rather than a pinned count. Precedent: the archived `add-codex-client-support` change had to rewrite this same line three → four. Missing it ships a merged spec that says five in one paragraph and four in another |
| `frontend/src/lib/pages/ClientsPage.test.ts` | §8.5 |

The count is pinned in at least six places; they move in one commit or the build is red. That is the intended tripwire, not a chore.

### 8.4 The manual oracle — partly discharged, the rest still required

Core tests run against fixtures, never the real machine, so nothing here can be closed by the suite.

**Already discharged (M1, M2 — recorded in §0 and §2.2, no longer a pending step):**

- The four `u32`s of the real `resources\app.asar`: `4`, `1 814 496`, `1 814 492`, `1 814 486`, in a file of `143 971 328` bytes.
- `header_len = 1 814 496`, `json_len = 1 814 486`, `data_start = 1 814 504`, corroborated byte-exactly by the 2-byte alignment padding (§2.2).
- `json_len = 1 814 486 <= 4 194 304` ✓ — the ceiling is not the binding constraint.

**Still required before this change is accepted**, on the affected machine (`C:\Users\raul_`), recorded in the change folder:

1. **A2** — that the header's root `files` map has a top-level `package.json` key. Not yet read.
2. **The extracted version** from `asar::read_package_version`, compared against the version OpenCode's own UI reports. **Equality is the only acceptance signal**, and after §2.2's retraction it is the only thing standing between this design and a silently wrong version. A mismatch is a stop-the-line result, not a fixture to adjust.
3. **The wall-clock cost of that call**, which is the real-world companion to `BENCH-1` (§3.1). Note the two are different measurements and both are wanted: `BENCH-1` isolates `jsonc::parse` on a synthetic ~1.8 MiB header and is reproducible; this one measures the whole call on the real archive with a warm page cache, and is what the user actually experiences.
4. **The whole-scan time** with and without the desktop app present, so the regression is quantified rather than estimated.

This is the same posture as the project's existing real-tool oracles (`opencode debug`, `claude agents`): manual verification, never an automated test.

### 8.5 Frontend tests

`ClientsPage.test.ts` today constructs **only** `claudeCodeNpm` records — it never builds a `claudeCodeBundled` record at all, which is exactly why H1 shipped. RED first, in this order:

1. `claude_code_card_reads_the_bundled_record_when_npm_is_not_detected` — npm `NotDetected` + bundled `Detected` with a version ⇒ the card shows detected, the bundled version, and `badgeFor` evaluated against the **bundled** slot. Fails against `find`.
2. `the_first_detected_record_wins_across_a_group_of_three_slots` — a synthetic three-slot group where slots 1 and 2 are `NotDetected` and slot 3 is `Detected`. Proves the rule for N, not 2.
3. `a_fully_undetected_group_still_renders_the_first_records_probed_paths` — the fallback arm.
4. `both_detected_selects_the_first_in_record_order` — pins the accepted limitation so a future Option C change has to change a test on purpose.
5. `opencode_card_reads_the_desktop_record_when_npm_is_not_detected` — the new group membership.

Run `npm run check` as well as `npm run test`: vitest does not typecheck the regenerated binding.

### 8.6 Rust test order (RED first)

1. `parse_prefix_rejects_a_prefix_whose_payload_length_disagrees_with_the_header_length`
2. `parse_prefix_rejects_a_header_len_below_four_without_underflowing` — parameterised over `{0,1,2,3}`; the §8.2 `tiny-header-len` boundary. A bare `header_len - 4` panics in debug and wraps in release, so this test fails loudly against the notation §2.2 forbids
3. `parse_prefix_refuses_a_header_above_the_ceiling_without_reading_it` — asserts on a **16-byte** input, proving the ceiling fires before any allocation
4. `data_start_is_eight_plus_header_len_not_json_start_plus_json_len` — pins the two formulas as **different** on a padded fixture (§2.2's boxed rule)
5. `a_shifted_payload_never_yields_the_neighbouring_manifests_version` — the `shifted-payload` fixture; the D2 guard against the systematic-offset hazard
6. `a_version_without_a_name_is_refused` — the D3 guard
7. `locate_package_json_ignores_a_nested_node_modules_package_json` — §2.4's rejected-recursion guard, at unit level
8. `read_package_version_returns_the_root_version_from_a_synthetic_archive` (in memory)
9. `opencode_desktop_root_without_a_readable_archive_is_detected_with_no_installations` — the degradation invariant
10. `home_without_the_desktop_root_yields_not_detected_and_zero_issues` — CA-11
11. `oversized_header_degrades_with_a_warning_not_an_error` — §5.2's severity decision
12. `scan_for_emits_five_records_in_probe_table_order`

| Layer | What |
|---|---|
| Unit, no I/O | §8.1 pure layer; `ClientInstallSlot::OpenCodeDesktop`'s label/client/version-source |
| Integration | The §8.2 fixture table, one test per row, via `installations::scan_for(home, HostPlatform::Windows)` — green on all three CI legs (CA-17) |
| Fixture integrity | The §8.2 `build_asar` round-trip equality test per blob |
| Read-only | The existing tree-snapshot equality tests extended over the new fixture homes (CA-16) |
| Contract | No `ClientInstallation` with an empty version; `IssueSeverity` still exactly two variants; `ClientInstallSlot` exhaustive match |

Gates: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --locked`, `cargo deny check bans licenses`, the `msrv` job, bindings-in-sync, and `npm run lint && npm run check && npm run test && npm run build`.

## 9. File changes

| File | Action | Description |
|---|---|---|
| `crates/vertice-core/src/asar.rs` | **Create** | §2 — the reader; sole owner of the byte layout |
| `crates/vertice-core/src/lib.rs` | Modify | `pub mod asar;` |
| `crates/vertice-core/src/model/slot.rs` | Modify | `OpenCodeDesktop` variant + `label()` arm (§4.1) |
| `crates/vertice-core/src/installations.rs` | Modify | `client()`/`version_source()` arms, `VersionSource::AsarPackageJson`, probe entry, `resolve_opencode_desktop_slot`, `resolve_slot` dispatch; module doc "four slots" → "five" (§4.2) |
| `crates/vertice-core/src/jsonc.rs` | **Unchanged** | Reused as-is (§3.2) |
| `crates/vertice-core/src/scan.rs` | Modify | **Test pins only** — no logic change (§8.3) |
| `crates/vertice-app/src/freshness/upstream.rs` | Modify | One arm returning `None` (§4.3) |
| `crates/vertice-app/capabilities/default.json`, `deny.toml`, `Cargo.toml`, `Cargo.lock` | **Unchanged** | No new dependency, no new capability |
| `frontend/src/bindings/ClientInstallSlot.ts` | Regenerated | Five variants; never hand-edited |
| `frontend/src/lib/pages/ClientsPage.svelte` | Modify | `presenceFor` (§6.1) + the `openCode` group's `slots` (§6.2). Two edits, nothing else |
| `frontend/src/lib/i18n/catalogs.ts`, `clientGroups.ts` | **Unchanged** | Zero new keys (§6.2) |
| `crates/vertice-core/tests/fixtures/client-installations/opencode-desktop/**` | **Create** | §8.2 — 14 cases (including `tiny-header-len`, `shifted-payload`, `no-name-key`), each with an `app.asar.layout.txt` sidecar |
| `.gitattributes` | Modify | One `binary` line for the blobs (§8.2) |
| `crates/vertice-core/tests/client_installations.rs`, `model_contract.rs` | Modify | New cases + the 4→5 pins (§8.3) |
| `frontend/src/lib/pages/ClientsPage.test.ts` | Modify | §8.5, RED first |
| `crates/vertice-core/tests/fixtures/scan-orchestrator/**`, `fixtures/roots/reference/**` | **Byte-identical** | §8.2 |
| `internal-docs/pendientes-desarrollo.md` | Modify | The H3 standalone-installer gap; the deferred OpenCode upstream; the stale `owner: "SST"` copy; the `.gitattributes:6` stale path; **the fixture self-consistency blind spot (§8.1)**; and **the measured scan-time cost once `BENCH-1` lands (§3.1)** |

## 10. Slicing and rollback

Three slices, each independently green and independently revertible:

1. **H1 only** — `presenceFor` + `ClientsPage.test.ts`. Frontend-only, no binding change, ships the reported Claude Code fix on its own. Smallest useful increment; if slice 2 or 3 stalls, this one still closes the user-visible bug.
2. **`asar.rs` + its pure and in-memory tests + `BENCH-1`.** No slot, no wiring, no binding — a self-contained module with no caller yet. Deliberately isolated because it is where the correctness and cost risk lives, and because `BENCH-1`'s result (§3.1) can still send this slice back to §3.2 without any of slice 1 or 3 being wasted.
3. **The `OpenCodeDesktop` slot** — model variant, probe, resolver, `upstream_for` arm, fixtures, the 4→5 pins, the regenerated binding, and the OpenCode group membership. The only slice touching the binding, so core and frontend land together.

Final slicing is `sdd-tasks`'s call. Rollback is the proposal's ordered three layers, unchanged: reverting restores the four-variant enum, regenerates `ClientInstallSlot.ts` back to four, restores `find`, drops the `upstream_for` arm, deletes `asar.rs` and the fixtures. A partial rollback (core reverted, binding not) fails at the CI drift gate or at `npm run check`, never silently at runtime. No persisted state, no migration — the scan is recomputed in memory every run.

## 11. Open questions

- [x] Where the asar reader lives, its surface and its failure type — `src/asar.rs`, one public function, `AsarError`. §2.1
- [x] **The exact byte layout — MEASURED, not assumed (M1).** Six `checked_*` cross-checks, plus the boxed rule that `data_start` is `8 + header_len` and never `json_start + json_len`. §2.2
- [x] **U1 — CLOSED.** The real header is 1 814 486 bytes (1.73 MiB), under the ceiling. The answer redirected the design: the ceiling was never the binding constraint; the cost at the real size is. §0, §3.1
- [x] How `package.json` is located — root-level only; recursive search explicitly rejected as silently wrong, now backed by D1/D2/D3 defense in depth. §2.4
- [x] Header parsed through `jsonc.rs`? — **yes**, bounded by a size ceiling; streaming and a new dependency both rejected **pending `BENCH-1`**, which can reopen the question. §3.2
- [x] The header ceiling — **4 MiB**, ~2.3× the measured header; 2 MiB, 16 MiB and 512 KiB each rejected with a reason. §3.3
- [x] **The 20 ms budget — WITHDRAWN**, because nothing enforced it and the document's own arithmetic contradicted it. Replaced by an explicit list of what *is* enforced, plus `BENCH-1`. §3.1, §3.4
- [x] Is an oversized header an issue? — **yes, `Warning`**, now justified as a growth signal rather than as the U1 discharge. §5.2
- [x] The new `presenceFor` and its downstream consumers — one expression changed; `badgeFor`, `detected` and `versions` need no edit by construction. Zero i18n changes. §6
- [x] Slot name, label, position, arms and upstream — `OpenCodeDesktop`, `"OpenCode (desktop app)"`, positioned after `OpenCodeNpm`, `upstream_for → None`. §4
- [x] Fixture strategy — pure/in-memory for almost everything, plus fourteen tiny **committed** blobs with layout sidecars and a builder-equality integrity test; never generated at test time. §8.1–8.2
- [ ] **`BENCH-1` is blocking (§3.1).** The cost of `jsonc::parse` on a ~1.8 MiB header of this shape is unmeasured, and `jsonc.rs`'s parse-then-own-convert design (a `BTreeMap::insert` per key, an allocation per string) could put it an order of magnitude either side of the estimate. A result above ~100 ms reopens §3.2 rather than adjusting a constant. **No time bound is claimed until this lands**
- [ ] **A2 is still an assumption**, and §2.2's fail-closed argument is now stated as *low probability, not provably zero*: the payload is a dense forest of valid, version-bearing dependency manifests, so a systematic offset error's realistic failure is a **wrong version, not a parse error**. D1/D2/D3 (§2.4) plus the `shifted-payload` fixture are the mitigation; §8.4's version-equality oracle is the only proof
- [ ] **Fixture self-consistency has no ongoing guard (§8.1).** `build_asar` and the reader share one understanding, so a future OpenCode repackaging leaves CI green while the probe is dead on every real machine. Accepted and monitored, sibling to the folder-rename risk below; recorded in `internal-docs/pendientes-desarrollo.md`. Degradation stays benign — no version, never a wrong one
- [ ] A real freshness upstream for `OpenCodeDesktop` — deferred; needs proof that the desktop app and the CLI share a release-tag namespace. §4.3
- [ ] Option C (aggregate a product's detected slots into one card) — deferred, not rejected. OpenCode becoming a two-slot product makes it more likely, not less. §6.2
- [ ] The `@opencode-aidesktop` folder name is an electron-builder artifact and could be renamed upstream, silently killing the probe. Accepted: degradation is `NotDetected`, never an error. No mitigation exists short of an oracle
- [ ] Anthropic's native standalone installer (H3) and the macOS/Linux path tables — out of scope, logged separately
