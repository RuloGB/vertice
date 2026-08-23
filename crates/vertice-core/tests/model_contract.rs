//! Integration contract tests for `vertice_core::model`, mirroring T1's
//! `tests/yaml_behavior.rs`. Fixture-free and zero disk I/O, per
//! `design.md` §11: these tests construct values in memory only.

use vertice_core::model::{
    ClientInstallation, ClientKind, ClientPresenceStatus, Component, ComponentId, ComponentKind,
    IssueSeverity, Location, LocationOrigin, Scope, SearchRoot, SearchRootId, SearchRootKind,
    SearchRootStatus,
};
use vertice_core::model::{ScanIssue, ScanReport};

fn sample_search_root(id: &str, kind: SearchRootKind) -> SearchRoot {
    SearchRoot {
        id: SearchRootId(id.to_string()),
        path: std::path::PathBuf::from(format!("/roots/{id}")),
        kind,
        status: SearchRootStatus::Found,
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
    };
    let with_path = Location {
        path: Some(std::path::PathBuf::from("/roots/root-a/skill/SKILL.md")),
        root,
        origin: LocationOrigin::File,
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
            },
            Location {
                path: Some(std::path::PathBuf::from("/roots/b/issue-creation/SKILL.md")),
                root: SearchRootId("root-b".to_string()),
                origin: LocationOrigin::File,
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
                },
                Location {
                    path: Some(std::path::PathBuf::from(
                        "/roots/root-b/issue-creation/SKILL.md",
                    )),
                    root: root_b.id.clone(),
                    origin: LocationOrigin::File,
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
