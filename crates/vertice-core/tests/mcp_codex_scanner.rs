//! Integration tests for the Codex MCP adapter (design §10.4, tasks
//! Slice 5). Closes anchor 0.7's Codex half, E3's Codex leg, and the final
//! `ROOT_ORDER` wiring.

use std::path::PathBuf;

use vertice_core::model::{IssueSeverity, McpTransport};

fn fixture_home(case: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("fixtures");
    path.push("mcp");
    path.push("codex");
    path.push(case);
    path
}

#[test]
fn fake_token_in_env_never_reaches_the_serialized_report() {
    let scan = vertice_core::mcp_codex::scan(&fixture_home("stdio-secret"));

    let serialized = serde_json::to_string(&scan.components).expect("must serialize");
    assert!(!serialized.contains("FAKE"));

    let component = scan
        .components
        .iter()
        .find(|c| c.name == "github")
        .expect("github component must be present");
    let location = component.locations.first().expect("one location");
    match &location.mcp_transport {
        Some(McpTransport::Stdio {
            command,
            arg_count,
            env_keys,
        }) => {
            assert_eq!(command, "npx");
            assert_eq!(*arg_count, 1);
            assert_eq!(env_keys, &vec!["GITHUB_TOKEN".to_string()]);
        }
        other => panic!("expected Stdio transport, got {other:?}"),
    }
}

/// Closes anchor 0.7's Codex half: a `{ url }`-only entry with no
/// `command` and no `type` anywhere yields `Remote` — the structural
/// discriminator, from the opposite direction of the Claude leg.
#[test]
fn remote_only_entry_with_no_type_is_discriminated_structurally() {
    let scan = vertice_core::mcp_codex::scan(&fixture_home("remote-secret"));

    let component = scan
        .components
        .iter()
        .find(|c| c.name == "github")
        .expect("github component must be present");
    let location = component.locations.first().expect("one location");
    match &location.mcp_transport {
        Some(McpTransport::Remote { url, header_keys }) => {
            assert_eq!(url, "https://mcp.example.test");
            assert_eq!(header_keys, &vec!["Authorization".to_string()]);
        }
        other => panic!("expected Remote transport, got {other:?}"),
    }

    let serialized = serde_json::to_string(&scan.components).expect("must serialize");
    assert!(!serialized.contains("FAKE"));
}

/// Malformed TOML yields one fixed-reason `Error`, no `toml`-crate
/// `Display` text embedded — the direct regression for §7.2's TOML-side
/// hazard.
#[test]
fn malformed_config_yields_one_error_with_a_fixed_reason_and_no_parser_text() {
    let scan = vertice_core::mcp_codex::scan(&fixture_home("malformed"));

    assert!(scan.components.is_empty());
    assert_eq!(scan.issues.len(), 1);
    assert_eq!(scan.issues[0].severity, IssueSeverity::Error);
    assert_eq!(
        scan.issues[0].reason,
        "could not parse the Codex MCP configuration"
    );
}

#[test]
fn malformed_config_with_adjacent_secret_never_leaks_it() {
    let scan = vertice_core::mcp_codex::scan(&fixture_home("malformed-secret-adjacent"));

    let serialized_components =
        serde_json::to_string(&scan.components).expect("components must serialize");
    let serialized_issues = serde_json::to_string(&scan.issues).expect("issues must serialize");
    assert!(!serialized_components.contains("FAKE"));
    assert!(!serialized_issues.contains("FAKE"));
}

#[test]
fn root_key_wrong_type_confirmed_mcp_servers_absent_case() {
    let scan = vertice_core::mcp_codex::scan(&fixture_home("root-key-wrong-type"));

    assert!(scan.components.is_empty());
    assert_eq!(scan.issues.len(), 1);
    assert_eq!(scan.issues[0].severity, IssueSeverity::Warning);
}

/// One wrong-typed scalar field degrades via `Lenient`, does NOT escalate
/// to a file-level `Error` — the sibling entry survives.
#[test]
fn entry_field_wrong_type_does_not_escalate_and_sibling_survives() {
    let scan = vertice_core::mcp_codex::scan(&fixture_home("entry-field-wrong-type"));

    assert_eq!(scan.components.len(), 2);

    let broken = scan
        .components
        .iter()
        .find(|c| c.name == "broken-field")
        .expect("broken-field component must be present");
    assert_eq!(broken.locations.first().unwrap().mcp_transport, None);

    let github = scan
        .components
        .iter()
        .find(|c| c.name == "github")
        .expect("github component must be present");
    assert!(matches!(
        github.locations.first().unwrap().mcp_transport,
        Some(McpTransport::Stdio { .. })
    ));
}

/// `args: []` (M5's real observed shape) yields `arg_count: 0` and a valid
/// `Stdio`, not a degraded entry.
#[test]
fn empty_args_yields_zero_and_a_valid_stdio() {
    let scan = vertice_core::mcp_codex::scan(&fixture_home("empty-args"));

    let component = scan
        .components
        .iter()
        .find(|c| c.name == "github")
        .expect("github component must be present");
    let location = component.locations.first().expect("one location");
    match &location.mcp_transport {
        Some(McpTransport::Stdio { arg_count, .. }) => assert_eq!(*arg_count, 0),
        other => panic!("expected Stdio transport, got {other:?}"),
    }
    assert!(scan.issues.is_empty());
}

/// `args: ["--flag", 42]` yields `arg_count: 2`, a valid `Stdio`, and NO
/// `Warning` — a non-string element is counted, never inspected.
#[test]
fn args_non_string_element_is_counted_not_inspected() {
    let scan = vertice_core::mcp_codex::scan(&fixture_home("args-non-string-element"));

    let component = scan
        .components
        .iter()
        .find(|c| c.name == "github")
        .expect("github component must be present");
    let location = component.locations.first().expect("one location");
    match &location.mcp_transport {
        Some(McpTransport::Stdio { arg_count, .. }) => assert_eq!(*arg_count, 2),
        other => panic!("expected Stdio transport, got {other:?}"),
    }
    assert!(scan.issues.is_empty());
}

#[test]
fn absent_config_yields_zero_components_and_no_mcp_specific_issue() {
    let scan = vertice_core::mcp_codex::scan(&fixture_home("absent"));

    assert!(scan.components.is_empty());
    assert!(scan.issues.is_empty());
}

/// E3 — Codex leg: a wrong-typed `command` with a valid `url` falls back to
/// `Remote`, never `None`. Closes E3 fully (Claude Slice 3, OpenCode
/// Slice 4, Codex here).
#[test]
fn entry_unusable_command_valid_url_falls_back_to_remote() {
    let scan = vertice_core::mcp_codex::scan(&fixture_home("entry-unusable-command-valid-url"));

    let component = scan
        .components
        .iter()
        .find(|c| c.name == "github")
        .expect("github component must be present");
    let location = component.locations.first().expect("one location");
    assert!(matches!(
        location.mcp_transport,
        Some(McpTransport::Remote { .. })
    ));
    assert_eq!(scan.issues.len(), 1);
    assert_eq!(scan.issues[0].severity, IssueSeverity::Warning);
}

#[test]
fn complete_fixture_yields_one_stdio_component() {
    let scan = vertice_core::mcp_codex::scan(&fixture_home("complete"));

    assert_eq!(scan.components.len(), 1);
    assert!(scan.issues.is_empty());
}
