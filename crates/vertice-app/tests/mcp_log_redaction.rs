//! Global RED anchor 0.2 (`add-mcp-scanning` `tasks.md`, `design.md` §12).
//!
//! References `log_scan_report_with`'s emission-capturing closure
//! (`crates/vertice-app/src/commands.rs:59-62`), which is private to that
//! module — today's inline unit tests exercise it from inside
//! `commands.rs` (mirroring `read_only_audit.rs`'s text-based-audit
//! pattern for everything else in this integration-test directory). This
//! stub stays failing until Slice 3 (task 3.10) makes it runnable end to
//! end for the Claude leg, at which point it moves the real assertion
//! in — or, if `log_scan_report_with` is not made reachable from an
//! integration test by then, records that decision here instead of
//! silently relocating the anchor.
//!
//! **Scoped honestly per §12 item 2** (task 0.2's own hedge): today's
//! logger emits only `root.id` / `root.path` (for a `NotFound` root) and
//! `record.label` (for `NotDetected` client presence) — verified against
//! `commands.rs:62-85`. It never reads `report.issues`, so this anchor's
//! claim over `ScanIssue.reason` is forward-looking defense in depth
//! (design §7.2's exact hedge), not a claim that today's logger is proven
//! unsafe against a reason it never touches.
//!
//! **Decision recorded here, per this file's own hedge (task 3.10):**
//! `log_scan_report_with` is `fn`-private to `commands.rs`, not reachable
//! from this integration-test crate. The real end-to-end assertion —
//! scanning `claude/stdio-secret`, building a `ScanReport` from it, and
//! capturing `log_scan_report_with`'s emission closure — lives as a unit
//! test inside `commands.rs` itself:
//! `mcp_secrets_never_reach_the_scan_report_log`. This stub is kept GREEN
//! and non-trivial by independently proving the half of the guarantee this
//! crate CAN observe without a private seam: the `ScanReport` produced by
//! `vertice_core::mcp_claude::scan` against that same fixture never
//! contains a `FAKE`-vocabulary secret when serialized — the exact input
//! the private logger test then feeds through the log-capturing closure.
#[test]
fn fake_token_in_env_never_reaches_the_application_log() {
    let mut fixture_home = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    fixture_home.pop();
    fixture_home.push("vertice-core");
    fixture_home.push("tests");
    fixture_home.push("fixtures");
    fixture_home.push("mcp");
    fixture_home.push("claude");
    fixture_home.push("stdio-secret");

    let scan = vertice_core::mcp_claude::scan(&fixture_home);
    assert!(!scan.components.is_empty());

    let serialized = serde_json::to_string(&scan.components).expect("must serialize");
    assert!(!serialized.contains("FAKE"));
}
