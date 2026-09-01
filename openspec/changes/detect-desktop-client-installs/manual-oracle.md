# Phase 10 — Manual oracle results

Recorded 2026-09-01, against the affected machine (`C:\Users\raul_`) and its real
OpenCode desktop installation.

Archive under test: `C:\Users\raul_\AppData\Local\Programs\@opencode-aidesktop\resources\app.asar`
(143,971,328 bytes).

## 10.1 — A2: the root `files` map carries a top-level `package.json` key — DISCHARGED

The archive prefix reads, as four little-endian `u32` values:

| Offset | Value | Relation |
| --- | --- | --- |
| 0 | 4 | canonical pickle size |
| 4 | 1814496 | `header_len` |
| 8 | 1814492 | `header_len - 4` |
| 12 | 1814486 | `json_len` |

The header JSON parses and `files["package.json"]` resolves at the root of the tree, so
A2 holds on a real archive. The layout invariant is corroborated byte-exactly: the JSON
ends at `16 + 1814486 = 1814502`, while `data_start = 8 + header_len = 1814504`. The
two-byte gap is the format's 4-byte alignment padding, which is precisely why
`data_start` MUST NOT be computed as `json_start + json_len`.

## 10.2 — Version equality — PARTIALLY DISCHARGED

Reading the root `package.json` entry at `data_start + entry.offset` and parsing exactly
`entry.size` bytes yields:

```
name    = @opencode-ai/desktop
version = 1.18.25
```

This is unambiguously the application's own manifest, not a bundled dependency's: the
`name` matches the install folder `@opencode-aidesktop`, which electron-builder derives
by concatenating the npm scope and the package name. The systematic-offset hazard raised
in design §2.2 therefore did not materialise on this archive.

It also confirms the D3 design decision: D3 requires a non-empty `name` but deliberately
does NOT assert `name == "opencode"`. Had it asserted equality, this real installation
would fail, because the desktop package is named `@opencode-ai/desktop`.

**Still open:** the extracted string has not yet been compared against the version
OpenCode's own UI reports. Equality against the UI is the acceptance signal for this
gate, and a mismatch is a stop-the-line result, not a fixture to adjust.

## 10.3 — Wall-clock cost of the real call — NOT MEASURED

Not run. `BENCH-1`'s synthetic measurement (30.9 ms average / 43.6 ms worst-of-20 against
a 1,814,766-byte synthetic header) stands as the only figure so far. The real-archive
companion measurement is still wanted.

## 10.4 — Whole-scan time with and without the desktop app present — NOT MEASURED

Not run. The scan-time regression remains estimated from `BENCH-1`, not measured
end-to-end on the affected machine.

## 10.5 — Recording

This file is that record. Phase 10 is **not** fully discharged: 10.1 is closed, 10.2 is
closed except for the UI comparison, and 10.3/10.4 are open. This gate MUST NOT be marked
done by any CI run.
