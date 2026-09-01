//! Integration contract tests for `vertice_core::model`, mirroring T1's
//! `tests/yaml_behavior.rs`. Fixture-free and zero disk I/O, per
//! `design.md` §11: these tests construct values in memory only.

use vertice_core::model::{
    ClientInstallSlot, ClientInstallation, ClientKind, ClientPresenceStatus, Component,
    ComponentId, ComponentKind, Freshness, IssueSeverity, Location, LocationOrigin, Scope,
    SearchRoot, SearchRootId, SearchRootKind, SearchRootStatus, UserSettings,
};
use vertice_core::model::{ScanIssue, ScanReport};

fn sample_search_root(id: &str, kind: SearchRootKind) -> SearchRoot {
    SearchRoot {
        id: SearchRootId(id.to_string()),
        path: std::path::PathBuf::from(format!("/roots/{id}")),
        kind,
        status: SearchRootStatus::Found,
        client: None,
    }
}

/// CA-1: a `Location` with `path: None` is representable and stays
/// distinguishable from one with `path: Some(..)`.
#[test]
fn pathless_and_present_path_locations_are_distinguishable() {
    let root = SearchRootId("root-a".to_string());

    let pathless = Location {
        path: None,
        root: root.clone(),
        origin: LocationOrigin::Embedded,
        mcp_transport: None,
        client: None,
    };
    let with_path = Location {
        path: Some(std::path::PathBuf::from("/roots/root-a/skill/SKILL.md")),
        root,
        origin: LocationOrigin::File,
        mcp_transport: None,
        client: None,
    };

    assert_ne!(pathless, with_path);
    assert_eq!(pathless.path, None);
    assert!(with_path.path.is_some());
}

/// CA-2: one `Component`, N `Location`s, same id derived twice from the
/// same `(kind, name)` pair.
#[test]
fn one_component_holds_multiple_locations_under_one_shared_id() {
    let id_a = ComponentId::derive(ComponentKind::Skill, "issue-creation");
    let id_b = ComponentId::derive(ComponentKind::Skill, "issue-creation");
    assert_eq!(
        id_a, id_b,
        "identity must be deterministic across derivations"
    );

    let component = Component {
        id: id_a,
        name: "issue-creation".to_string(),
        kind: ComponentKind::Skill,
        description: None,
        scope: Scope::User,
        locations: vec![
            Location {
                path: Some(std::path::PathBuf::from("/roots/a/issue-creation/SKILL.md")),
                root: SearchRootId("root-a".to_string()),
                origin: LocationOrigin::File,
                mcp_transport: None,
                client: None,
            },
            Location {
                path: Some(std::path::PathBuf::from("/roots/b/issue-creation/SKILL.md")),
                root: SearchRootId("root-b".to_string()),
                origin: LocationOrigin::File,
                mcp_transport: None,
                client: None,
            },
        ],
        provenance_hint: None,
    };

    assert_eq!(component.locations.len(), 2);
    assert_eq!(component.id, id_b);
}

/// CA-3: `scope` is a required, populated field — never absent or
/// default-omitted.
#[test]
fn scope_is_explicitly_populated_on_construction() {
    let component = Component {
        id: ComponentId::derive(ComponentKind::Agent, "triage"),
        name: "triage".to_string(),
        kind: ComponentKind::Agent,
        description: None,
        scope: Scope::User,
        locations: Vec::new(),
        provenance_hint: None,
    };

    assert_eq!(component.scope, Scope::User);
}

/// An empty `ScanReport` is a legitimate, complete value — not an error
/// signal — and round-trips through `serde_json`.
#[test]
fn empty_scan_report_round_trips_through_json() {
    let report = ScanReport {
        components: Vec::new(),
        installations: Vec::new(),
        roots_scanned: Vec::new(),
        issues: Vec::new(),
        client_presence: None,
        duration_ms: 0,
    };

    let json = serde_json::to_string(&report).expect("empty report must serialize");
    let round_tripped: ScanReport =
        serde_json::from_str(&json).expect("empty report must deserialize");

    assert_eq!(round_tripped.components.len(), 0);
    assert_eq!(round_tripped.installations.len(), 0);
    assert_eq!(round_tripped.issues.len(), 0);
    assert_eq!(round_tripped.duration_ms, 0);
}

/// A non-empty `ScanReport` — including both `ScanIssue` severities and a
/// `ClientInstallation` — round-trips through `serde_json` without loss.
#[test]
fn populated_scan_report_round_trips_through_json() {
    let root = sample_search_root("root-a", SearchRootKind::Skill);

    let report = ScanReport {
        components: vec![Component {
            id: ComponentId::derive(ComponentKind::Skill, "issue-creation"),
            name: "issue-creation".to_string(),
            kind: ComponentKind::Skill,
            description: Some("Creates issues".to_string()),
            scope: Scope::User,
            locations: vec![Location {
                path: Some(std::path::PathBuf::from(
                    "/roots/root-a/issue-creation/SKILL.md",
                )),
                root: root.id.clone(),
                origin: LocationOrigin::File,
                mcp_transport: None,
                client: None,
            }],
            provenance_hint: Some("claude-code".to_string()),
        }],
        installations: vec![ClientInstallation {
            client: ClientKind::ClaudeCode,
            version: "1.2.3".to_string(),
            path: std::path::PathBuf::from("/clients/claude-code"),
        }],
        roots_scanned: vec![root],
        issues: vec![
            ScanIssue {
                severity: IssueSeverity::Warning,
                path: Some(std::path::PathBuf::from("/roots/root-a/broken/SKILL.md")),
                reason: "unparseable frontmatter".to_string(),
            },
            ScanIssue {
                severity: IssueSeverity::Error,
                path: None,
                reason: "root not found".to_string(),
            },
        ],
        client_presence: None,
        duration_ms: 42,
    };

    let json = serde_json::to_string(&report).expect("populated report must serialize");
    let round_tripped: ScanReport =
        serde_json::from_str(&json).expect("populated report must deserialize");

    assert_eq!(round_tripped, report);
}

/// Referential integrity: every `Location::root` used by a `Component` in
/// this report resolves to a `SearchRootId` present in
/// `ScanReport::roots_scanned`. T2 states this contract and tests it on a
/// constructed report; enforcing it at scan time is T9's responsibility.
#[test]
fn location_root_resolves_to_a_scanned_search_root() {
    let root_a = sample_search_root("root-a", SearchRootKind::Skill);
    let root_b = sample_search_root("root-b", SearchRootKind::Agent);

    let report = ScanReport {
        components: vec![Component {
            id: ComponentId::derive(ComponentKind::Skill, "issue-creation"),
            name: "issue-creation".to_string(),
            kind: ComponentKind::Skill,
            description: None,
            scope: Scope::User,
            locations: vec![
                Location {
                    path: Some(std::path::PathBuf::from(
                        "/roots/root-a/issue-creation/SKILL.md",
                    )),
                    root: root_a.id.clone(),
                    origin: LocationOrigin::File,
                    mcp_transport: None,
                    client: None,
                },
                Location {
                    path: Some(std::path::PathBuf::from(
                        "/roots/root-b/issue-creation/SKILL.md",
                    )),
                    root: root_b.id.clone(),
                    origin: LocationOrigin::File,
                    mcp_transport: None,
                    client: None,
                },
            ],
            provenance_hint: None,
        }],
        installations: Vec::new(),
        roots_scanned: vec![root_a, root_b],
        issues: Vec::new(),
        client_presence: None,
        duration_ms: 7,
    };

    let known_root_ids: Vec<&SearchRootId> =
        report.roots_scanned.iter().map(|root| &root.id).collect();

    for component in &report.components {
        for location in &component.locations {
            assert!(
                known_root_ids.contains(&&location.root),
                "Location.root {:?} must resolve to a SearchRoot in roots_scanned",
                location.root
            );
        }
    }
}

/// Spec: `Scope` Is a Closed, Exhaustively Matchable Enum.
///
/// The wildcard-free `match` below is the actual assertion: `Scope` was
/// deliberately left without `#[non_exhaustive]` so that adding a variant
/// breaks compilation at every match site rather than silently falling into
/// a catch-all arm. If a fourth `Scope` is ever added, this test stops
/// compiling — which is the intended failure.
#[test]
fn scope_is_exhaustively_matchable_without_a_wildcard_arm() {
    fn label(scope: Scope) -> &'static str {
        match scope {
            Scope::User => "user",
            Scope::Project => "project",
            Scope::Local => "local",
        }
    }

    assert_eq!(label(Scope::User), "user");
    assert_eq!(label(Scope::Project), "project");
    assert_eq!(label(Scope::Local), "local");
}

/// Spec (`domain-model`, `add-mcp-scanning`): `ComponentKind` Is a Closed,
/// Exhaustively Matchable Enum admitting three variants.
///
/// The wildcard-free `match` below is the actual assertion: a fourth
/// `ComponentKind` variant would break compilation here instead of silently
/// falling into a catch-all arm.
#[test]
fn component_kind_is_exhaustively_matchable_without_a_wildcard_arm() {
    fn label(kind: ComponentKind) -> &'static str {
        match kind {
            ComponentKind::Skill => "skill",
            ComponentKind::Agent => "agent",
            ComponentKind::Mcp => "mcp",
        }
    }

    assert_eq!(label(ComponentKind::Skill), "skill");
    assert_eq!(label(ComponentKind::Agent), "agent");
    assert_eq!(label(ComponentKind::Mcp), "mcp");
}

/// Spec (`domain-model`, `add-mcp-scanning`): `SearchRootKind` Is a Closed,
/// Exhaustively Matchable Enum admitting three variants, mirroring
/// `ComponentKind`.
#[test]
fn search_root_kind_is_exhaustively_matchable_without_a_wildcard_arm() {
    fn label(kind: SearchRootKind) -> &'static str {
        match kind {
            SearchRootKind::Skill => "skill",
            SearchRootKind::Agent => "agent",
            SearchRootKind::Mcp => "mcp",
        }
    }

    assert_eq!(label(SearchRootKind::Skill), "skill");
    assert_eq!(label(SearchRootKind::Agent), "agent");
    assert_eq!(label(SearchRootKind::Mcp), "mcp");
}

/// Spec: `ClientKind` Is A Closed Enumeration Admitting Three Named Clients.
///
/// The wildcard-free `match` below is the actual assertion: a fourth
/// `ClientKind` variant would break compilation here instead of silently
/// falling into a catch-all arm (domain-model spec, "ClientKind is
/// exhaustively matchable").
#[test]
fn client_kind_is_exhaustively_matchable_without_a_wildcard_arm() {
    fn label(kind: ClientKind) -> &'static str {
        match kind {
            ClientKind::ClaudeCode => "claudeCode",
            ClientKind::OpenCode => "openCode",
            ClientKind::Codex => "codex",
        }
    }

    assert_eq!(label(ClientKind::ClaudeCode), "claudeCode");
    assert_eq!(label(ClientKind::OpenCode), "openCode");
    assert_eq!(label(ClientKind::Codex), "codex");
}

/// Spec: `ClientPresenceStatus` Is a Closed, Exhaustively Matchable Enum.
///
/// Same construction as `Scope` above, and for the same reason: the status
/// carries the whole "did we find this client" answer, so a future variant
/// must break compilation at every match site instead of silently landing
/// in a catch-all arm and being rendered as if it were one of these two.
#[test]
fn client_presence_status_is_exhaustively_matchable_without_a_wildcard_arm() {
    fn label(status: ClientPresenceStatus) -> &'static str {
        match status {
            ClientPresenceStatus::Detected => "detected",
            ClientPresenceStatus::NotDetected => "notDetected",
        }
    }

    assert_eq!(label(ClientPresenceStatus::Detected), "detected");
    assert_eq!(label(ClientPresenceStatus::NotDetected), "notDetected");
}

/// Spec: a component with no provenance hint is representable without a
/// sentinel — `None`, `Some("")` and `Some("claude-code")` are three
/// distinct states, both in Rust and across the serialized contract.
///
/// This is what makes `Option<String>` more than a stylistic preference:
/// with a plain `String`, the first two states would collapse into one.
#[test]
fn provenance_hint_absent_empty_and_present_are_three_distinct_states() {
    fn component_with_hint(hint: Option<&str>) -> Component {
        Component {
            id: ComponentId::derive(ComponentKind::Skill, "issue-creation"),
            name: "issue-creation".to_string(),
            kind: ComponentKind::Skill,
            description: None,
            scope: Scope::User,
            locations: Vec::new(),
            provenance_hint: hint.map(str::to_string),
        }
    }

    let absent = component_with_hint(None);
    let empty = component_with_hint(Some(""));
    let present = component_with_hint(Some("claude-code"));

    assert_ne!(absent, empty, "None must not collapse into Some(\"\")");
    assert_ne!(absent, present);
    assert_ne!(empty, present);

    // The distinction must survive the IPC contract, not just Rust equality.
    let absent_json = serde_json::to_value(&absent).expect("absent serializes");
    let empty_json = serde_json::to_value(&empty).expect("empty serializes");

    assert_eq!(absent_json["provenanceHint"], serde_json::Value::Null);
    assert_eq!(empty_json["provenanceHint"], serde_json::json!(""));
}

/// Spec (`client-installation-detector`): the slot discriminator is
/// exhaustively matchable — the promoted `ClientInstallSlot` mirrors the
/// `Scope`/`ClientPresenceStatus` pattern. A fifth variant would break
/// compilation here instead of silently falling into a catch-all arm.
#[test]
fn client_install_slot_is_exhaustively_matchable_without_a_wildcard_arm() {
    fn label(slot: ClientInstallSlot) -> &'static str {
        match slot {
            ClientInstallSlot::ClaudeCodeNpm => "Claude Code CLI (npm)",
            ClientInstallSlot::ClaudeCodeBundled => "Claude Code (bundled in Claude Desktop)",
            ClientInstallSlot::OpenCodeNpm => "OpenCode (npm)",
            ClientInstallSlot::OpenCodeDesktop => "OpenCode (desktop app)",
            ClientInstallSlot::CodexStandalone => "Codex CLI (standalone)",
        }
    }

    assert_eq!(
        label(ClientInstallSlot::ClaudeCodeNpm),
        "Claude Code CLI (npm)"
    );
    assert_eq!(
        label(ClientInstallSlot::ClaudeCodeBundled),
        "Claude Code (bundled in Claude Desktop)"
    );
    assert_eq!(label(ClientInstallSlot::OpenCodeNpm), "OpenCode (npm)");
    assert_eq!(
        label(ClientInstallSlot::OpenCodeDesktop),
        "OpenCode (desktop app)"
    );
    assert_eq!(
        label(ClientInstallSlot::CodexStandalone),
        "Codex CLI (standalone)"
    );
}

/// Spec (`component-freshness`): `Freshness` is a closed three-valued
/// verdict — no `#[non_exhaustive]`, no fourth "ahead"/"prerelease" state.
/// The wildcard-free match is the actual assertion.
#[test]
fn freshness_is_exhaustively_matchable_without_a_wildcard_arm() {
    fn label(freshness: &Freshness) -> &'static str {
        match freshness {
            Freshness::UpToDate => "upToDate",
            Freshness::Outdated { .. } => "outdated",
            Freshness::Unknown { .. } => "unknown",
        }
    }

    assert_eq!(label(&Freshness::UpToDate), "upToDate");
    assert_eq!(
        label(&Freshness::Outdated {
            latest: "1.0.0".to_string()
        }),
        "outdated"
    );
    assert_eq!(
        label(&Freshness::Unknown {
            reason: "unparseable".to_string()
        }),
        "unknown"
    );
}

/// domain-model spec, "SearchRoot Carries Its Owning Client": a
/// client-specific root with `Some(ClaudeCode)` survives a JSON
/// round-trip at the integration-contract level, preserving the typed
/// ownership value across the serde boundary.
#[test]
fn search_root_with_client_round_trips_through_json_contract() {
    let root = SearchRoot {
        id: SearchRootId("claude-skills".to_string()),
        path: std::path::PathBuf::from("/home/user/.claude/skills"),
        kind: SearchRootKind::Skill,
        status: SearchRootStatus::Found,
        client: Some(ClientKind::ClaudeCode),
    };

    let json = serde_json::to_string(&root).expect("must serialize");
    let round_tripped: SearchRoot = serde_json::from_str(&json).expect("must deserialize");

    assert_eq!(round_tripped, root);
    assert_eq!(round_tripped.client, Some(ClientKind::ClaudeCode));
}

/// domain-model spec, "Shared root carries no single owner": the
/// `agents-skills` shape serializes `client` as JSON `null` and
/// round-trips to `None` at the integration-contract level.
#[test]
fn shared_search_root_serializes_client_as_json_null_contract() {
    let root = SearchRoot {
        id: SearchRootId("agents-skills".to_string()),
        path: std::path::PathBuf::from("/home/user/.agents/skills"),
        kind: SearchRootKind::Skill,
        status: SearchRootStatus::Found,
        client: None,
    };

    let json = serde_json::to_value(&root).expect("must serialize");
    assert_eq!(json["client"], serde_json::Value::Null);

    let round_tripped: SearchRoot = serde_json::from_value(json).expect("must deserialize");
    assert_eq!(round_tripped.client, None);
}

/// `add-locale-persistence`: the durable, whole-document user settings type
/// replaces the former `FreshnessSettings` (superseded — `enabled` and
/// `disclosure_seen` now live in this single durable document alongside the
/// frontend's persisted `locale` choice). Same plain-data, camelCase-
/// serialized, `TS`-derived pattern as every other type in this module.
#[test]
fn user_settings_round_trips_camel_cased() {
    let settings = UserSettings {
        locale: Some("es".to_string()),
        enabled: false,
        disclosure_seen: true,
    };

    let json = serde_json::to_value(&settings).expect("UserSettings must serialize");
    assert_eq!(json["locale"], "es");
    assert_eq!(json["enabled"], false);
    assert_eq!(json["disclosureSeen"], true);

    let round_tripped: UserSettings =
        serde_json::from_value(json).expect("UserSettings must deserialize its own output");
    assert_eq!(round_tripped, settings);
}

/// domain-model spec, "Location Carries the Producing Root Client": a
/// skill location produced from the `claude-skills` fixture root carries
/// `Some(ClientKind::ClaudeCode)` (CA-17: versioned fixtures only).
#[test]
fn skill_location_carries_its_roots_client() {
    let mut home = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    home.push("tests");
    home.push("fixtures");
    home.push("scan-orchestrator");
    home.push("complete");

    let scan = vertice_core::skills::scan(&home);
    let claude_root = scan
        .roots
        .iter()
        .find(|r| r.id.0 == "claude-skills")
        .expect("claude-skills root must be present");
    assert_eq!(claude_root.client, Some(ClientKind::ClaudeCode));

    let claude_location = scan
        .components
        .iter()
        .flat_map(|c| &c.locations)
        .find(|loc| loc.root == claude_root.id)
        .expect("at least one location must come from claude-skills");
    assert_eq!(claude_location.client, Some(ClientKind::ClaudeCode));
}

/// domain-model spec, "Location Carries the Producing Root Client": the
/// `complete` fixture's `shared` component has one location from
/// `claude-skills` (`Some(ClaudeCode)`) and one from `agents-skills`
/// (`None`) — the load-bearing Some+None pair, already on disk.
#[test]
fn shared_skill_locations_carry_no_client_for_shared_root() {
    let mut home = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    home.push("tests");
    home.push("fixtures");
    home.push("scan-orchestrator");
    home.push("complete");

    let scan = vertice_core::skills::scan(&home);
    let consolidated = vertice_core::consolidate::consolidate(scan.components);
    let shared = consolidated
        .iter()
        .find(|c| c.name == "shared")
        .expect("shared skill must be reported");
    assert_eq!(shared.locations.len(), 2);

    let clients: Vec<Option<ClientKind>> = shared.locations.iter().map(|loc| loc.client).collect();
    assert!(
        clients.contains(&Some(ClientKind::ClaudeCode)),
        "one location must come from claude-skills"
    );
    assert!(
        clients.contains(&None),
        "one location must come from agents-skills (shared, no owner)"
    );
}

/// domain-model spec, "Location Client Has Referential Integrity": over
/// the full `complete` fixture report, every location's `client` equals
/// the `client` of the `SearchRoot` that produced it.
#[test]
fn every_location_client_matches_its_root_client() {
    let mut home = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    home.push("tests");
    home.push("fixtures");
    home.push("scan-orchestrator");
    home.push("complete");

    let skills = vertice_core::skills::scan(&home);
    let agents = vertice_core::agents::scan(&home);
    let opencode_agents = vertice_core::opencode_agents::scan(&home);
    let codex_agents = vertice_core::codex_agents::scan(&home);
    let claude_mcp = vertice_core::mcp_claude::scan(&home);
    let opencode_mcp = vertice_core::mcp_opencode::scan(&home);
    let codex_mcp = vertice_core::mcp_codex::scan(&home);

    let mut roots: Vec<SearchRoot> = Vec::new();
    roots.extend(skills.roots);
    roots.extend(agents.roots);
    roots.extend(opencode_agents.roots);
    roots.extend(codex_agents.roots);
    roots.extend(claude_mcp.roots);
    roots.extend(opencode_mcp.roots);
    roots.extend(codex_mcp.roots);

    let mut components: Vec<Component> = Vec::new();
    components.extend(skills.components);
    components.extend(agents.components);
    components.extend(opencode_agents.components);
    components.extend(codex_agents.components);
    components.extend(claude_mcp.components);
    components.extend(opencode_mcp.components);
    components.extend(codex_mcp.components);
    let components = vertice_core::consolidate::consolidate(components);

    let root_client_map: std::collections::HashMap<&str, Option<ClientKind>> = roots
        .iter()
        .map(|root| (root.id.0.as_str(), root.client))
        .collect();

    for component in &components {
        for location in &component.locations {
            let root_client = root_client_map
                .get(location.root.0.as_str())
                .unwrap_or_else(|| {
                    panic!(
                        "location root {} not found in roots_scanned",
                        location.root.0
                    )
                });
            assert_eq!(
                location.client, *root_client,
                "location client must match its root's client for root {}",
                location.root.0
            );
        }
    }
}
