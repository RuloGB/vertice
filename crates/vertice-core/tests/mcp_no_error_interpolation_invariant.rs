//! Enforces `openspec/changes/add-mcp-scanning/design.md` §7.2 (reasons
//! never interpolate beyond a fixed allow-list) and the
//! `workspace-architecture` delta's seam-containment rule, over the four
//! MCP modules: `mcp.rs`, `mcp_claude.rs`, `mcp_opencode.rs`,
//! `mcp_codex.rs`.
//!
//! **Two calibration corrections, recorded here because an earlier draft of
//! this clause was flagged mis-specified by adversarial review
//! (`design.md` §13.5, E2/E-panic):**
//!
//! 1. The precedent this test's shape is modelled on
//!    (`tests/toml_seam_invariant.rs`) is a whole-file `content.contains`
//!    scan with **no `#[cfg(test)]` exclusion**. This crate writes its unit
//!    tests inline, and the four MCP modules' own test modules freely use
//!    `.expect(...)`/`.unwrap(...)`/`panic!(...)` and interpolate arbitrary
//!    identifiers in assertion messages — a naive copy of that precedent
//!    would flag those idiomatic test assertions and fail CI on day one.
//!    Every check below is run against the source **with every
//!    `#[cfg(test)] mod ... { ... }` block textually removed first.**
//! 2. A literal `{err}` grep is both too narrow (defeated by binding the
//!    error to any other name, `{e}`, `{parse_err}`, `{cause}`) and misses
//!    non-`format!` panic surfaces. This test is therefore **structural,
//!    not literal**: it extracts every interpolated identifier from every
//!    `format!`/`write!` call outside `sanitize_url`/`sanitize_host_port`
//!    (the two functions that legitimately build a *sanitized URL*, not a
//!    `ScanIssue.reason` — `mcp.rs`'s own test module already pins their
//!    output), and rejects any interpolated identifier that is not on the
//!    fixed, three-entry allow-list: the server key, the client label, and
//!    the path.
//!
//! **Residual gap, stated honestly per design §7.2 — this is not claimed to
//! be airtight.** This is sound against direct interpolation inside a
//! `format!`/`write!` call reachable from these four files' non-test
//! bodies. It cannot trace a value laundered through an intermediate
//! variable this simple textual parser does not follow (e.g.
//! `let msg = some_other_module::build(cause); ScanIssue { reason: msg,
//! .. }`), and it does not by itself prove every `ScanIssue` construction
//! site uses one of the checked calls — it closes the specific, realistic
//! defeat (renaming the bound identifier) the literal-`{err}` grep did not,
//! nothing more.
//!
//! This test also enforces the panic-surface hardening from §7.2's
//! "Hardening" paragraph (`.unwrap(`, `.expect(`, `panic!(` MUST NOT appear
//! in these four modules' non-test code — already enforced at compile time
//! by each module's `#![deny(clippy::unwrap_used, clippy::expect_used, ...)]`
//! inner attribute, re-confirmed here textually), and the seam-containment
//! rule (no MCP module imports `jsonc_parser`/`toml_seam` directly; every
//! access goes through `crate::jsonc`/`crate::toml`).

use std::fs;
use std::path::PathBuf;

/// The four MCP modules this invariant polices (`design.md` §7.2, §11).
const TARGET_FILES: [&str; 4] = ["mcp.rs", "mcp_claude.rs", "mcp_opencode.rs", "mcp_codex.rs"];

/// The fixed, three-entry allow-list (`design.md` §7.2): the server key,
/// the client label, and the path. Spelled as every identifier name this
/// codebase's convention could plausibly bind them to, so a future
/// contributor renaming (e.g.) `key` to `server_key` does not need to touch
/// this test — narrower than "anything goes", still not a single literal.
const ALLOWED_INTERPOLATED_IDENTIFIERS: [&str; 6] = [
    "key",
    "server_key",
    "client_label",
    "client",
    "label",
    "path",
];

/// Functions that legitimately build a *sanitized URL string* — not a
/// `ScanIssue.reason` — and are therefore out of this invariant's scope.
/// Their own identifiers (`scheme`, `host`, `port`, `host_port`,
/// `bracketed`) are pinned by `mcp.rs`'s own `sanitize_url` unit tests, not
/// by this file.
const REASON_UNRELATED_FUNCTIONS: [&str; 2] = ["sanitize_url", "sanitize_host_port"];

fn src_dir() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("src");
    path
}

/// Read a target file's full source, or panic with a clear message if it is
/// missing — a missing file here would silently make this whole invariant
/// vacuous.
fn read_target(file_name: &str) -> String {
    let path = src_dir().join(file_name);
    fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
}

/// Remove every `#[cfg(test)] mod ... { ... }` block, brace-balanced, so
/// this crate's idiomatic inline unit tests are never scanned (calibration
/// correction 1, module doc comment above).
fn strip_test_modules(content: &str) -> String {
    const MARKER: &str = "#[cfg(test)]";
    let mut result = String::with_capacity(content.len());
    let mut rest = content;

    while let Some(marker_pos) = rest.find(MARKER) {
        result.push_str(&rest[..marker_pos]);
        let after_marker = &rest[marker_pos..];

        match brace_balanced_block_end(after_marker) {
            Some(end) => {
                rest = &after_marker[end..];
            }
            None => {
                // No balanced `mod ... { ... }` found after the marker —
                // should not happen for well-formed Rust. Fail loudly
                // rather than silently under-stripping.
                panic!(
                    "found `#[cfg(test)]` with no balanced brace block after it; \
                     this invariant test's stripping logic needs updating"
                );
            }
        }
    }
    result.push_str(rest);
    result
}

/// Given text starting at (or before) a `{`, return the byte offset one
/// past the matching closing `}` for the FIRST brace found, walking depth.
/// Used both for stripping test modules and for extracting named function
/// bodies.
fn brace_balanced_block_end(text: &str) -> Option<usize> {
    let brace_pos = text.find('{')?;
    let mut depth = 0usize;
    for (i, ch) in text[brace_pos..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(brace_pos + i + 1);
                }
            }
            _ => {}
        }
    }
    None
}

/// Remove every occurrence of `fn <name>(` ... `{ ... }` (brace-balanced)
/// for the given function name, so its body is excluded from the
/// reason-interpolation scan (calibration correction 2: `sanitize_url`/
/// `sanitize_host_port` build a URL, not a `ScanIssue.reason`).
fn strip_named_function(content: &str, fn_name: &str) -> String {
    let marker = format!("fn {fn_name}(");
    let mut result = String::with_capacity(content.len());
    let mut rest = content;

    while let Some(marker_pos) = rest.find(marker.as_str()) {
        result.push_str(&rest[..marker_pos]);
        let after_marker = &rest[marker_pos..];
        match brace_balanced_block_end(after_marker) {
            Some(end) => rest = &after_marker[end..],
            None => panic!(
                "found `fn {fn_name}(` with no balanced brace block after it; \
                 this invariant test's stripping logic needs updating"
            ),
        }
    }
    result.push_str(rest);
    result
}

/// Extract the balanced-parenthesis argument text of every `format!(` and
/// `write!(` call in `content` (test modules and the reason-unrelated
/// functions already stripped out by the caller).
fn find_macro_call_args(content: &str) -> Vec<String> {
    let mut calls = Vec::new();
    for macro_marker in ["format!(", "write!("] {
        let mut rest = content;
        let mut base_offset = 0usize;
        while let Some(rel) = rest.find(macro_marker) {
            let call_start = base_offset + rel + macro_marker.len();
            let after_open = &content[call_start..];
            if let Some(args) = extract_balanced_parens(after_open) {
                calls.push(args);
            }
            let advance = rel + macro_marker.len();
            rest = &rest[advance..];
            base_offset += advance;
        }
    }
    calls
}

/// `text` starts one character past an already-consumed opening `(`.
/// Returns everything up to (not including) the matching closing `)`,
/// tracking string literals so a `)` inside a string does not end the call
/// early.
fn extract_balanced_parens(text: &str) -> Option<String> {
    let mut depth = 1usize;
    let mut in_string = false;
    let mut escaped = false;
    let mut end = None;

    for (i, ch) in text.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    end = Some(i);
                    break;
                }
            }
            _ => {}
        }
    }

    end.map(|end| text[..end].to_string())
}

/// Extract the first string literal from a macro call's argument text
/// (the format string), then every `{ident}` shorthand-capture identifier
/// inside it. `{{`/`}}` escapes and empty/positional `{}`/`{0}` are
/// deliberately not identifiers and are skipped — they cannot smuggle a
/// config-derived value, only a value already passed positionally, which
/// this test does not need to trust because none of these calls pass any
/// positional argument today (every argument in these four modules is a
/// shorthand capture, confirmed by inspection).
fn extract_format_string_identifiers(call_args: &str) -> Vec<String> {
    let Some(format_string) = extract_first_string_literal(call_args) else {
        return Vec::new();
    };

    let mut identifiers = Vec::new();
    let chars: Vec<char> = format_string.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '{' {
            if chars.get(i + 1) == Some(&'{') {
                i += 2;
                continue;
            }
            if let Some(close) = chars[i..].iter().position(|&c| c == '}') {
                let ident: String = chars[i + 1..i + close].iter().collect();
                let ident = ident.trim();
                if !ident.is_empty()
                    && ident
                        .chars()
                        .next()
                        .is_some_and(|c| c.is_alphabetic() || c == '_')
                {
                    // A shorthand capture like `{key}`; a `{key:?}` debug
                    // spec keeps only the identifier before the `:`.
                    let ident = ident.split(':').next().unwrap_or(ident).trim();
                    identifiers.push(ident.to_string());
                }
                i += close + 1;
                continue;
            }
        }
        i += 1;
    }
    identifiers
}

fn extract_first_string_literal(text: &str) -> Option<String> {
    let start = text.find('"')?;
    let mut result = String::new();
    let mut escaped = false;
    for ch in text[start + 1..].chars() {
        if escaped {
            result.push(ch);
            escaped = false;
            continue;
        }
        match ch {
            '\\' => escaped = true,
            '"' => return Some(result),
            other => result.push(other),
        }
    }
    None
}

#[test]
fn no_scan_issue_reason_interpolates_beyond_the_fixed_allow_list() {
    let mut offenders = Vec::new();

    for file_name in TARGET_FILES {
        let raw = read_target(file_name);
        let mut scoped = strip_test_modules(&raw);
        for reason_unrelated_fn in REASON_UNRELATED_FUNCTIONS {
            scoped = strip_named_function(&scoped, reason_unrelated_fn);
        }

        for call_args in find_macro_call_args(&scoped) {
            for identifier in extract_format_string_identifiers(&call_args) {
                if !ALLOWED_INTERPOLATED_IDENTIFIERS.contains(&identifier.as_str()) {
                    offenders.push(format!("{file_name}: interpolates `{{{identifier}}}`"));
                }
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "a `format!`/`write!` call in an MCP module interpolates an identifier \
         outside the fixed allow-list ({ALLOWED_INTERPOLATED_IDENTIFIERS:?}); design §7.2 \
         requires every `ScanIssue.reason` to stay free of parser errors, URLs, \
         arguments, env values, header values, or file content: {offenders:?}"
    );
}

#[test]
fn no_mcp_module_calls_unwrap_expect_or_panic_outside_its_own_tests() {
    let mut offenders = Vec::new();

    for file_name in TARGET_FILES {
        let raw = read_target(file_name);
        let scoped = strip_test_modules(&raw);

        for pattern in [".unwrap(", ".expect(", "panic!("] {
            if scoped.contains(pattern) {
                offenders.push(format!("{file_name}: contains `{pattern}`"));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "design §7.2's panic-surface hardening: MCP modules MUST NOT call \
         `.unwrap()`, `.expect()`, or `panic!()` over any value bound during the \
         redact phase (already enforced at compile time by each module's \
         `#![deny(clippy::unwrap_used, clippy::expect_used, ...)]`; this is the \
         textual re-confirmation): {offenders:?}"
    );
}

#[test]
fn no_mcp_module_imports_the_jsonc_or_toml_crate_directly() {
    let mut offenders = Vec::new();

    for file_name in TARGET_FILES {
        let raw = read_target(file_name);
        let scoped = strip_test_modules(&raw);

        for pattern in [
            "use jsonc_parser",
            "jsonc_parser::",
            "use toml_seam",
            "toml_seam::",
        ] {
            if scoped.contains(pattern) {
                offenders.push(format!("{file_name}: contains `{pattern}`"));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "`workspace-architecture`'s seam-containment rule: only `jsonc.rs`/`toml.rs` \
         may import `jsonc_parser`/`toml_seam` directly; every MCP module MUST go \
         through `crate::jsonc`/`crate::toml`: {offenders:?}"
    );
}

/// A meta-check on this test itself: confirms `strip_test_modules` actually
/// removes something from at least one target file, so a future rename of
/// the `#[cfg(test)] mod tests` convention does not silently turn the two
/// tests above into vacuously-passing no-ops.
#[test]
fn strip_test_modules_removes_a_non_empty_test_module_from_every_target_file() {
    for file_name in TARGET_FILES {
        let raw = read_target(file_name);
        let scoped = strip_test_modules(&raw);
        assert!(
            scoped.len() < raw.len(),
            "{file_name}: stripping `#[cfg(test)]` modules did not shrink the \
             source; this file's test module may not follow the expected \
             `#[cfg(test)] mod ... {{ ... }}` shape any more"
        );
    }
}

/// A meta-check confirming `strip_named_function` actually removes
/// `sanitize_url`'s body from `mcp.rs`, so the exclusion in the first test
/// above is not silently vacuous either.
#[test]
fn strip_named_function_removes_sanitize_url_from_mcp_rs() {
    let raw = read_target("mcp.rs");
    let scoped = strip_test_modules(&raw);
    assert!(scoped.contains("fn sanitize_url("));

    let stripped = strip_named_function(&scoped, "sanitize_url");
    assert!(!stripped.contains("format!(\"{scheme}://{host_port}\")"));
}
