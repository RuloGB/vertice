//! Global RED anchors for `add-mcp-scanning` (`design.md` §12, `tasks.md`
//! Slice 0). Each anchor is closed GREEN, in place, by the slice named in
//! its doc comment — this file is the canonical home; the per-client
//! integration suites (`mcp_claude_scanner.rs`, `mcp_opencode_scanner.rs`,
//! `mcp_codex_scanner.rs`) additionally exercise the same behaviours in
//! more detail.
//!
//! Anchors 0.3 and 0.4 are NOT here: they are pure, fixture-free
//! `sanitize_url` properties, so they live directly in
//! `crates/vertice-core/src/mcp.rs`'s own test module (closed in Slice 2,
//! tasks 2.2.1/2.2.3) rather than as an integration-test stub.
//!
//! Anchors 0.9 and 0.11 are closed here in Slice 6, the first point at
//! which all three adapters exist. This file also carries Slice 6's two
//! additional consolidation regression pins (`design.md` §8, `tasks.md`
//! 6.2/6.3/6.6): a fixture with several servers in one config file, and the
//! verified `enabled: bool` shape on a Codex and an OpenCode entry.

use std::path::PathBuf;

fn fixture_home(client: &str, case: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("fixtures");
    path.push("mcp");
    path.push(client);
    path.push(case);
    path
}

/// Anchor 0.1. Closed GREEN in Slice 3 (task 3.9): the `FAKE` guard over
/// the serialized report for `claude/stdio-secret` (design §10.2).
#[test]
fn fake_token_in_env_never_reaches_the_serialized_report() {
    let scan = vertice_core::mcp_claude::scan(&fixture_home("claude", "stdio-secret"));

    let serialized = serde_json::to_string(&scan.components).expect("must serialize");
    assert!(!serialized.contains("FAKE"));
}

/// Anchor 0.5. Closed GREEN in Slice 3 (task 3.6/3.8): an unparseable URL
/// yields `mcp_transport: None` plus one `Warning`, and the raw URL string
/// never appears in the `ScanIssue.reason` (design §3.3, §7.2).
#[test]
fn unparseable_url_yields_no_transport_and_a_warning_without_echoing_the_url() {
    let scan = vertice_core::mcp_claude::scan(&fixture_home("claude", "remote-unparseable-url"));

    let component = scan
        .components
        .first()
        .expect("one component must be present");
    assert_eq!(component.locations.first().unwrap().mcp_transport, None);
    assert_eq!(scan.issues.len(), 1);
    assert!(!scan.issues[0].reason.contains("mcp.example.test"));
}

/// Anchor 0.6. Closed GREEN in Slice 3 (task 3.5): `arg_count` reflects the
/// configured argument count, with no argument value anywhere on
/// `McpTransport` (design §2, §6.3).
#[test]
fn token_bearing_argument_yields_only_a_count() {
    let scan = vertice_core::mcp_claude::scan(&fixture_home("claude", "stdio-secret"));

    let component = scan
        .components
        .first()
        .expect("one component must be present");
    match &component.locations.first().unwrap().mcp_transport {
        Some(vertice_core::model::McpTransport::Stdio { arg_count, .. }) => {
            assert_eq!(*arg_count, 1);
        }
        other => panic!("expected Stdio transport, got {other:?}"),
    }
}

/// Anchor 0.7. Closed GREEN partially in Slice 3 (Claude half) and fully in
/// Slice 5 (Codex half): transport discrimination never reads a `type`
/// field (design §6.3).
#[test]
fn entry_without_a_type_field_is_discriminated_structurally() {
    let claude_scan = vertice_core::mcp_claude::scan(&fixture_home("claude", "settings-json-only"));
    let claude_component = claude_scan
        .components
        .first()
        .expect("one Claude component must be present");
    assert!(matches!(
        claude_component.locations.first().unwrap().mcp_transport,
        Some(vertice_core::model::McpTransport::Stdio { .. })
    ));

    let codex_scan = vertice_core::mcp_codex::scan(&fixture_home("codex", "remote-secret"));
    let codex_component = codex_scan
        .components
        .first()
        .expect("one Codex component must be present");
    assert!(matches!(
        codex_component.locations.first().unwrap().mcp_transport,
        Some(vertice_core::model::McpTransport::Remote { .. })
    ));
}

/// Anchor 0.8. Closed GREEN in Slice 4 (task 4.4): `["npx", "-y", "pkg"]`
/// maps to `command: "npx"`, `arg_count: 2` (design §6.3's `command`/`args`
/// asymmetry table).
#[test]
fn opencode_array_command_maps_to_command_plus_arg_count() {
    let scan = vertice_core::mcp_opencode::scan(&fixture_home("opencode", "complete"));

    let component = scan
        .components
        .first()
        .expect("one component must be present");
    match &component.locations.first().unwrap().mcp_transport {
        Some(vertice_core::model::McpTransport::Stdio {
            command, arg_count, ..
        }) => {
            assert_eq!(command, "npx");
            assert_eq!(*arg_count, 2);
        }
        other => panic!("expected Stdio transport, got {other:?}"),
    }
}

/// Anchor 0.9 (`tasks.md`). Closed GREEN in Slice 6 (task 6.4), the first
/// point at which all three adapters exist: one `Component` with three
/// `Location`s, ordered `claude-mcp -> opencode-mcp -> codex-mcp`, each
/// carrying its own transport (design §5.3, §8). `merge_into` concatenates
/// locations and never deduplicates — nothing is discarded at merge time.
#[test]
fn same_server_name_in_three_clients_yields_one_component_with_three_transports() {
    let home = fixture_home("shared", "same-name-three-clients");

    let mut components = Vec::new();
    components.extend(vertice_core::mcp_claude::scan(&home).components);
    components.extend(vertice_core::mcp_opencode::scan(&home).components);
    components.extend(vertice_core::mcp_codex::scan(&home).components);
    let input_len = components.len();

    let consolidated = vertice_core::consolidate::consolidate(components);
    assert_eq!(
        consolidated
            .iter()
            .map(|c| c.locations.len())
            .sum::<usize>(),
        input_len,
        "total_location_count_is_conserved must hold across MCP adapters too"
    );

    let github: Vec<_> = consolidated.iter().filter(|c| c.name == "github").collect();
    assert_eq!(github.len(), 1, "one Component for the shared identity");
    let component = github.first().expect("one Component for github");
    assert_eq!(component.locations.len(), 3);

    let root_order: Vec<&str> = component
        .locations
        .iter()
        .map(|loc| loc.root.0.as_str())
        .collect();
    assert_eq!(
        root_order,
        vec!["claude-mcp", "opencode-mcp", "codex-mcp"],
        "MCP family precedence is claude-mcp < opencode-mcp < codex-mcp (design §5.3)"
    );

    let commands: Vec<String> = component
        .locations
        .iter()
        .map(|loc| match &loc.mcp_transport {
            Some(vertice_core::model::McpTransport::Stdio { command, .. }) => command.clone(),
            other => panic!("expected Stdio transport, got {other:?}"),
        })
        .collect();
    assert_eq!(
        commands,
        vec![
            "claude-github-cli",
            "opencode-github-cli",
            "codex-github-cli"
        ],
        "each location must retain its own transport, never a shared/collapsed one"
    );
}

/// `tasks.md` 6.2: several servers declared in one config file must
/// consolidate into a deterministic, total component order (design §8.4).
#[test]
fn several_servers_in_one_file_consolidate_in_deterministic_order() {
    let home = fixture_home("shared", "several-servers-one-file");

    let scan = vertice_core::mcp_claude::scan(&home);
    assert_eq!(scan.components.len(), 3);

    let consolidated = vertice_core::consolidate::consolidate(scan.components);
    let names: Vec<&str> = consolidated.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, vec!["alpha", "beta", "gamma"]);

    // Run again to prove the order is deterministic, not incidentally stable.
    let scan_again = vertice_core::mcp_claude::scan(&home);
    let consolidated_again = vertice_core::consolidate::consolidate(scan_again.components);
    let names_again: Vec<&str> = consolidated_again.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, names_again);
}

/// `tasks.md` 6.3/6.6: the verified `enabled: bool` shape (M7 for Codex, M9
/// for OpenCode) is never read — a disabled entry is still emitted, and
/// `provenance_hint` stays `None` (proposal decision 6, design §6.3).
#[test]
fn disabled_flagged_entries_are_still_emitted_with_no_provenance_hint() {
    let home = fixture_home("shared", "disabled-flagged");

    let opencode_scan = vertice_core::mcp_opencode::scan(&home);
    let opencode_component = opencode_scan
        .components
        .iter()
        .find(|c| c.name == "opencode-disabled")
        .expect("the disabled OpenCode entry must still be emitted");
    assert_eq!(opencode_component.provenance_hint, None);
    assert!(matches!(
        opencode_component.locations.first().unwrap().mcp_transport,
        Some(vertice_core::model::McpTransport::Stdio { .. })
    ));

    let codex_scan = vertice_core::mcp_codex::scan(&home);
    let codex_component = codex_scan
        .components
        .iter()
        .find(|c| c.name == "codex-disabled")
        .expect("the disabled Codex entry must still be emitted");
    assert_eq!(codex_component.provenance_hint, None);
    assert!(matches!(
        codex_component.locations.first().unwrap().mcp_transport,
        Some(vertice_core::model::McpTransport::Stdio { .. })
    ));
}

/// Anchor 0.10. Closed GREEN per-client starting Slice 3 (task 3.2/3.8):
/// exactly one fixed-reason `Error` with no parser-library `Display` text
/// embedded (design §7.1, §7.2).
#[test]
fn malformed_config_yields_one_error_with_a_fixed_reason_and_no_parser_text() {
    let scan = vertice_core::mcp_claude::scan(&fixture_home("claude", "malformed"));

    assert!(scan.components.is_empty());
    assert_eq!(scan.issues.len(), 1);
    assert_eq!(
        scan.issues[0].severity,
        vertice_core::model::IssueSeverity::Error
    );
    assert_eq!(
        scan.issues[0].reason,
        "could not parse the Claude Code MCP configuration"
    );
}

/// Anchor 0.11 (`tasks.md`, CA-11). Closed GREEN in Slice 6 (task 6.5):
/// zero MCP components and zero MCP-adapter-authored issues from all three
/// adapters against `shared/no-mcp-anywhere` — a `home` with none of the
/// three MCP config files present. Absence is `SearchRootStatus::NotFound`,
/// never a fault (design §7.1); the orchestrator-level path-less `NotFound`
/// `Warning` per root is separately exercised by
/// `scan-orchestrator/missing-root-client` in `src/scan.rs`'s own test
/// module, which has access to the private `scan_for`.
#[test]
fn home_without_any_mcp_configuration_yields_no_components_and_no_errors() {
    let home = fixture_home("shared", "no-mcp-anywhere");

    let claude_scan = vertice_core::mcp_claude::scan(&home);
    assert!(claude_scan.components.is_empty());
    assert!(claude_scan.issues.is_empty());
    assert!(claude_scan
        .roots
        .iter()
        .all(|root| root.status == vertice_core::model::SearchRootStatus::NotFound));

    let opencode_scan = vertice_core::mcp_opencode::scan(&home);
    assert!(opencode_scan.components.is_empty());
    assert!(opencode_scan.issues.is_empty());
    assert!(opencode_scan
        .roots
        .iter()
        .all(|root| root.status == vertice_core::model::SearchRootStatus::NotFound));

    let codex_scan = vertice_core::mcp_codex::scan(&home);
    assert!(codex_scan.components.is_empty());
    assert!(codex_scan.issues.is_empty());
    assert!(codex_scan
        .roots
        .iter()
        .all(|root| root.status == vertice_core::model::SearchRootStatus::NotFound));
}

/// `tasks.md` 7.6: the final, whole-tree run of the `FAKE` guard (design
/// §10.2) — every secret-bearing fixture, across all three clients, scanned
/// together and serialized together, not just the Claude-only Slice-3
/// subset a single fixture's own test already covers. A leak in the
/// interaction between two clients' output (as opposed to one client in
/// isolation) would only show up here.
#[test]
fn fake_guard_holds_across_the_full_secret_bearing_fixture_tree() {
    let secret_bearing: [(&str, &str); 8] = [
        ("claude", "stdio-secret"),
        ("claude", "remote-secret"),
        ("claude", "remote-dirty-url"),
        ("claude", "remote-userinfo-ambiguous-url"),
        ("claude", "malformed-secret-adjacent"),
        ("opencode", "stdio-secret"),
        ("opencode", "remote-secret"),
        ("opencode", "malformed-secret-adjacent"),
    ];

    let mut components = Vec::new();
    let mut issues = Vec::new();

    for (client, case) in secret_bearing {
        let home = fixture_home(client, case);
        let scan = match client {
            "claude" => vertice_core::mcp_claude::scan(&home),
            "opencode" => vertice_core::mcp_opencode::scan(&home),
            _ => unreachable!("only claude/opencode carry secret-bearing cases in this table"),
        };
        components.extend(scan.components);
        issues.extend(scan.issues);
    }

    for case in ["stdio-secret", "remote-secret", "malformed-secret-adjacent"] {
        let home = fixture_home("codex", case);
        let scan = vertice_core::mcp_codex::scan(&home);
        components.extend(scan.components);
        issues.extend(scan.issues);
    }

    let serialized_components =
        serde_json::to_string(&components).expect("components must serialize");
    assert!(!serialized_components.contains("FAKE"));

    let serialized_issues = serde_json::to_string(&issues).expect("issues must serialize");
    assert!(!serialized_issues.contains("FAKE"));
}
