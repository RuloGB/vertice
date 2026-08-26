//! Integration tests for the OpenCode MCP adapter (design §10.4, tasks
//! Slice 4). Closes anchors 0.8 and E3's OpenCode leg.

use std::path::PathBuf;

use vertice_core::model::{IssueSeverity, McpTransport};

fn fixture_home(case: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("fixtures");
    path.push("mcp");
    path.push("opencode");
    path.push(case);
    path
}

/// Anchor 0.8: `["npx", "-y", "pkg"]` maps to `command: "npx"`,
/// `arg_count: 2`.
#[test]
fn opencode_array_command_maps_to_command_plus_arg_count() {
    let scan = vertice_core::mcp_opencode::scan(&fixture_home("complete"));

    let component = scan
        .components
        .iter()
        .find(|c| c.name == "docs")
        .expect("docs component must be present");
    let location = component.locations.first().expect("one location");
    match &location.mcp_transport {
        Some(McpTransport::Stdio {
            command, arg_count, ..
        }) => {
            assert_eq!(command, "npx");
            assert_eq!(*arg_count, 2);
        }
        other => panic!("expected Stdio transport, got {other:?}"),
    }
}

#[test]
fn fake_token_in_environment_never_reaches_the_serialized_report() {
    let scan = vertice_core::mcp_opencode::scan(&fixture_home("stdio-secret"));

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

#[test]
fn remote_secret_header_key_survives_value_does_not() {
    let scan = vertice_core::mcp_opencode::scan(&fixture_home("remote-secret"));

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

#[test]
fn malformed_config_yields_one_error_with_a_fixed_reason_and_no_parser_text() {
    let scan = vertice_core::mcp_opencode::scan(&fixture_home("malformed"));

    assert!(scan.components.is_empty());
    assert_eq!(scan.issues.len(), 1);
    assert_eq!(scan.issues[0].severity, IssueSeverity::Error);
    assert_eq!(
        scan.issues[0].reason,
        "could not parse the OpenCode MCP configuration"
    );
}

#[test]
fn malformed_config_with_adjacent_secret_never_leaks_it() {
    let scan = vertice_core::mcp_opencode::scan(&fixture_home("malformed-secret-adjacent"));

    let serialized_components =
        serde_json::to_string(&scan.components).expect("components must serialize");
    let serialized_issues = serde_json::to_string(&scan.issues).expect("issues must serialize");
    assert!(!serialized_components.contains("FAKE"));
    assert!(!serialized_issues.contains("FAKE"));
}

#[test]
fn root_key_wrong_type_yields_zero_components_one_warning() {
    let scan = vertice_core::mcp_opencode::scan(&fixture_home("root-key-wrong-type"));

    assert!(scan.components.is_empty());
    assert_eq!(scan.issues.len(), 1);
    assert_eq!(scan.issues[0].severity, IssueSeverity::Warning);
}

#[test]
fn absent_config_yields_zero_components_and_no_mcp_specific_issue() {
    let scan = vertice_core::mcp_opencode::scan(&fixture_home("absent"));

    assert!(scan.components.is_empty());
    assert!(scan.issues.is_empty());
}

/// The boundary between "no arguments" and "no command": an empty
/// `command` array degrades to `None` plus a `Warning`.
#[test]
fn empty_command_array_degrades_to_none_with_a_warning() {
    let scan = vertice_core::mcp_opencode::scan(&fixture_home("empty-command-array"));

    let component = scan
        .components
        .iter()
        .find(|c| c.name == "github")
        .expect("github component must be present");
    let location = component.locations.first().expect("one location");
    assert_eq!(location.mcp_transport, None);
    assert_eq!(scan.issues.len(), 1);
    assert_eq!(scan.issues[0].severity, IssueSeverity::Warning);
}

/// E3 — OpenCode leg: a wrong-typed `command[0]` with a valid `url` falls
/// back to `Remote`, never `None`.
#[test]
fn entry_unusable_command_valid_url_falls_back_to_remote() {
    let scan = vertice_core::mcp_opencode::scan(&fixture_home("entry-unusable-command-valid-url"));

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

/// An overlay overriding one field must not erase the base's `command` —
/// both locations carry the merged effective transport (design §5.2), using
/// the shipped-and-pinned `opencode.json`/`opencode.jsonc` merge order.
#[test]
fn two_files_partial_override_merges_and_both_locations_share_the_transport() {
    let scan = vertice_core::mcp_opencode::scan(&fixture_home("two-files-partial-override"));

    let component = scan
        .components
        .iter()
        .find(|c| c.name == "github")
        .expect("github component must be present");
    assert_eq!(component.locations.len(), 2);

    for location in &component.locations {
        match &location.mcp_transport {
            Some(McpTransport::Stdio {
                command, env_keys, ..
            }) => {
                assert_eq!(command, "npx");
                assert_eq!(env_keys, &vec!["GITHUB_TOKEN".to_string()]);
            }
            other => panic!("expected Stdio transport, got {other:?}"),
        }
    }

    let serialized = serde_json::to_string(&scan.components).expect("must serialize");
    assert!(!serialized.contains("FAKE"));
}
