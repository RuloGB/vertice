//! Integration tests for the Claude Code MCP adapter (design §10.4, tasks
//! Slice 3). Closes anchors 0.1, 0.2 (via `mcp_log_redaction.rs`), 0.5,
//! 0.6, 0.7 (Claude half), 0.7a (Claude), 0.10 (Claude), and E4.

use std::path::PathBuf;

use vertice_core::model::{IssueSeverity, McpTransport};

fn fixture_home(case: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("fixtures");
    path.push("mcp");
    path.push("claude");
    path.push(case);
    path
}

/// Anchor 0.1 / §10.2's `FAKE` guard: no fake secret value survives into
/// the serialized report.
#[test]
fn fake_token_in_env_never_reaches_the_serialized_report() {
    let scan = vertice_core::mcp_claude::scan(&fixture_home("stdio-secret"));

    let serialized = serde_json::to_string(&scan.components).expect("components must serialize");
    assert!(!serialized.contains("FAKE"));
}

/// Anchor 0.6: an argument carrying a token yields only a count.
#[test]
fn token_bearing_argument_yields_only_a_count() {
    let scan = vertice_core::mcp_claude::scan(&fixture_home("stdio-secret"));

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
    let scan = vertice_core::mcp_claude::scan(&fixture_home("remote-secret"));

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
fn dirty_url_is_reduced_and_carries_no_fake_fragment() {
    let scan = vertice_core::mcp_claude::scan(&fixture_home("remote-dirty-url"));

    let component = scan
        .components
        .iter()
        .find(|c| c.name == "github")
        .expect("github component must be present");
    let location = component.locations.first().expect("one location");
    match &location.mcp_transport {
        Some(McpTransport::Remote { url, .. }) => {
            assert_eq!(url, "https://mcp.example.test:8443");
        }
        other => panic!("expected Remote transport, got {other:?}"),
    }

    let serialized = serde_json::to_string(&scan.components).expect("must serialize");
    assert!(!serialized.contains("FAKE"));
}

/// Closes 0.4 for the Claude leg: a userinfo containing a path delimiter
/// yields `None`, never a truncated authority fragment.
#[test]
fn userinfo_ambiguous_url_yields_no_transport_and_no_fake_fragment() {
    let scan = vertice_core::mcp_claude::scan(&fixture_home("remote-userinfo-ambiguous-url"));

    let component = scan
        .components
        .iter()
        .find(|c| c.name == "github")
        .expect("github component must be present");
    let location = component.locations.first().expect("one location");
    assert_eq!(location.mcp_transport, None);

    let serialized = serde_json::to_string(&scan.components).expect("must serialize");
    assert!(!serialized.contains("FAKE"));
}

/// Anchor 0.5: an unparseable URL yields `None` plus a `Warning`, and the
/// raw URL string never appears in the reason.
#[test]
fn unparseable_url_yields_no_transport_and_a_warning_without_echoing_the_url() {
    let scan = vertice_core::mcp_claude::scan(&fixture_home("remote-unparseable-url"));

    let component = scan
        .components
        .iter()
        .find(|c| c.name == "github")
        .expect("github component must be present");
    let location = component.locations.first().expect("one location");
    assert_eq!(location.mcp_transport, None);

    assert_eq!(scan.issues.len(), 1);
    assert_eq!(scan.issues[0].severity, IssueSeverity::Warning);
    assert!(!scan.issues[0].reason.contains("mcp.example.test"));
}

/// Anchor 0.10 (Claude half): malformed JSONC yields one fixed-reason
/// `Error`, no parser text embedded.
#[test]
fn malformed_config_yields_one_error_with_a_fixed_reason_and_no_parser_text() {
    let scan = vertice_core::mcp_claude::scan(&fixture_home("malformed"));

    assert!(scan.components.is_empty());
    assert_eq!(scan.issues.len(), 1);
    assert_eq!(scan.issues[0].severity, IssueSeverity::Error);
    assert_eq!(
        scan.issues[0].reason,
        "could not parse the Claude Code MCP configuration"
    );
}

/// The `FAKE` guard empirically proves §7.2's no-interpolation rule on the
/// JSONC path: a malformed `\u` escape adjacent to a `FAKE` token never
/// reaches the report.
#[test]
fn malformed_config_with_adjacent_secret_never_leaks_it() {
    let scan = vertice_core::mcp_claude::scan(&fixture_home("malformed-secret-adjacent"));

    let serialized_components =
        serde_json::to_string(&scan.components).expect("components must serialize");
    let serialized_issues = serde_json::to_string(&scan.issues).expect("issues must serialize");
    assert!(!serialized_components.contains("FAKE"));
    assert!(!serialized_issues.contains("FAKE"));
}

#[test]
fn root_key_wrong_type_yields_zero_components_one_warning() {
    let scan = vertice_core::mcp_claude::scan(&fixture_home("root-key-wrong-type"));

    assert!(scan.components.is_empty());
    assert_eq!(scan.issues.len(), 1);
    assert_eq!(scan.issues[0].severity, IssueSeverity::Warning);
}

#[test]
fn entry_wrong_type_emits_component_with_none_transport_and_a_warning() {
    let scan = vertice_core::mcp_claude::scan(&fixture_home("entry-wrong-type"));

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

/// Closes 0.7a for the Claude leg: an unusable `command` with a valid `url`
/// falls back to `Remote`, never `None`.
#[test]
fn entry_unusable_command_valid_url_falls_back_to_remote() {
    let scan = vertice_core::mcp_claude::scan(&fixture_home("entry-unusable-command-valid-url"));

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

/// Closes 0.7's Claude half: a `~/.claude/settings.json` entry with no
/// `type` key still yields `Stdio` — the structural discriminator.
#[test]
fn entry_without_a_type_field_is_discriminated_structurally() {
    let scan = vertice_core::mcp_claude::scan(&fixture_home("settings-json-only"));

    let component = scan
        .components
        .iter()
        .find(|c| c.name == "github")
        .expect("github component must be present");
    let location = component.locations.first().expect("one location");
    assert!(matches!(
        location.mcp_transport,
        Some(McpTransport::Stdio { .. })
    ));
}

/// An overlay overriding one field must not erase the base's `command` —
/// both locations carry the merged effective transport (design §5.2).
#[test]
fn two_files_partial_override_merges_and_both_locations_share_the_transport() {
    let scan = vertice_core::mcp_claude::scan(&fixture_home("two-files-partial-override"));

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

/// E4: `claude/empty-root-key` — a present-but-empty root key yields zero
/// components and zero issues.
#[test]
fn empty_mcp_servers_object_yields_zero_components_and_no_issue() {
    let scan = vertice_core::mcp_claude::scan(&fixture_home("empty-root-key"));

    assert!(scan.components.is_empty());
    assert!(scan.issues.is_empty());
}

#[test]
fn absent_config_yields_zero_components_and_no_mcp_specific_issue() {
    let scan = vertice_core::mcp_claude::scan(&fixture_home("absent"));

    assert!(scan.components.is_empty());
    assert!(scan.issues.is_empty());
}

/// An empty server key is emitted, never dropped (design §6.4).
#[test]
fn blank_key_is_emitted_with_no_issue() {
    let scan = vertice_core::mcp_claude::scan(&fixture_home("blank-key"));

    assert_eq!(scan.components.len(), 1);
    assert_eq!(scan.components[0].name, "");
    assert_eq!(scan.components[0].id.as_str(), "mcp:");
    assert!(scan.issues.is_empty());
}
