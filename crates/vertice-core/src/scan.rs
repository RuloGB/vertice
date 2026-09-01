//! Scan orchestration: compose the registered adapters into one report.

use std::collections::HashSet;
use std::path::Path;
use std::time::Instant;

use crate::installations::HostPlatform;
use crate::model::{IssueSeverity, ScanError, ScanIssue, ScanReport, SearchRoot, SearchRootStatus};

/// Run the registered user-root adapters for the current user.
///
/// The only failure that prevents a scan from beginning is resolving the
/// current home directory. Recoverable adapter and item diagnostics are
/// accumulated in the returned report.
pub fn scan() -> Result<ScanReport, ScanError> {
    let home = crate::roots::home_dir()?;

    Ok(scan_for(&home, HostPlatform::current()))
}

/// Compose every infallible adapter against one resolved home and platform.
/// Kept private so fixture tests can use deterministic paths without exposing
/// a second production API.
fn scan_for(home: &Path, platform: HostPlatform) -> ScanReport {
    let started = Instant::now();
    let skills = crate::skills::scan(home);
    let agents = crate::agents::scan(home);
    let opencode_agents = crate::opencode_agents::scan(home);
    let codex_agents = crate::codex_agents::scan(home);
    let claude_mcp = crate::mcp_claude::scan(home);
    let opencode_mcp = crate::mcp_opencode::scan(home);
    let codex_mcp = crate::mcp_codex::scan(home);
    let installations = crate::installations::scan_for(home, platform);

    let mut roots_scanned = skills.roots;
    roots_scanned.extend(agents.roots);
    roots_scanned.extend(opencode_agents.roots);
    roots_scanned.extend(codex_agents.roots);
    roots_scanned.extend(claude_mcp.roots);
    roots_scanned.extend(opencode_mcp.roots);
    roots_scanned.extend(codex_mcp.roots);

    let mut components = skills.components;
    components.extend(agents.components);
    components.extend(opencode_agents.components);
    components.extend(codex_agents.components);
    components.extend(claude_mcp.components);
    components.extend(opencode_mcp.components);
    components.extend(codex_mcp.components);

    let mut issues = skills.issues;
    issues.extend(agents.issues);
    issues.extend(opencode_agents.issues);
    issues.extend(codex_agents.issues);
    issues.extend(claude_mcp.issues);
    issues.extend(opencode_mcp.issues);
    issues.extend(codex_mcp.issues);
    issues.extend(installations.issues);
    append_missing_root_issues(&roots_scanned, &mut issues);

    let duration_ms = u32::try_from(started.elapsed().as_millis()).unwrap_or(u32::MAX);

    ScanReport {
        components: crate::consolidate::consolidate(components),
        installations: installations.installations,
        roots_scanned,
        issues,
        client_presence: installations.presence,
        duration_ms,
    }
}

fn append_missing_root_issues(roots: &[SearchRoot], issues: &mut Vec<ScanIssue>) {
    let mut reported = HashSet::new();
    for root in roots {
        if root.status == SearchRootStatus::NotFound && reported.insert(root.id.0.clone()) {
            issues.push(ScanIssue {
                severity: IssueSeverity::Warning,
                path: None,
                reason: format!("search root {} was not found", root.id.0),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::Read;
    use std::path::{Path, PathBuf};
    use std::time::SystemTime;

    use super::*;
    use crate::installations::HostPlatform;
    use crate::model::{ClientPresenceStatus, IssueSeverity, SearchRootStatus};

    fn fixture_home(case: &str) -> PathBuf {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests");
        path.push("fixtures");
        path.push("scan-orchestrator");
        path.push(case);
        path
    }

    #[test]
    fn complete_fixture_consolidates_all_adapter_output_into_one_report() {
        let report = scan_for(&fixture_home("complete"), HostPlatform::Windows);

        assert_eq!(report.roots_scanned.len(), 11);
        assert_eq!(report.installations.len(), 4);
        assert_eq!(report.components.len(), 15);
        let shared = report
            .components
            .iter()
            .find(|component| component.name == "shared")
            .expect("shared skill must be reported");
        assert_eq!(shared.locations.len(), 2);
        assert!(report.issues.is_empty());
    }

    #[test]
    fn corrupt_skill_is_reported_without_losing_sibling_adapter_results() {
        let home = fixture_home("corrupt-skill");
        let report = scan_for(&home, HostPlatform::Windows);

        let corrupt_path = home
            .join(".claude")
            .join("skills")
            .join("broken")
            .join("SKILL.md");
        assert!(report.issues.iter().any(|issue| {
            issue.severity == IssueSeverity::Error && issue.path.as_ref() == Some(&corrupt_path)
        }));
        assert!(report
            .components
            .iter()
            .any(|component| component.name == "valid-agent"));
        assert!(report
            .components
            .iter()
            .any(|component| component.name == "open-agent"));
        assert_eq!(report.installations.len(), 1);
    }

    #[test]
    fn missing_roots_and_clients_are_visible_diagnostics() {
        let report = scan_for(&fixture_home("missing-root-client"), HostPlatform::Windows);

        assert_eq!(report.roots_scanned.len(), 11);
        assert!(report
            .roots_scanned
            .iter()
            .all(|root| root.status == SearchRootStatus::NotFound));
        assert_eq!(
            report
                .issues
                .iter()
                .filter(|issue| issue.path.is_none() && issue.severity == IssueSeverity::Warning)
                .count(),
            11
        );

        let client_presence = report
            .client_presence
            .as_ref()
            .expect("Windows always has a probe table");
        assert_eq!(client_presence.len(), 5, "one record per defined slot");
        assert!(
            client_presence
                .iter()
                .all(|record| record.status == ClientPresenceStatus::NotDetected),
            "every slot is absent on this fixture"
        );
        assert!(
            report.installations.is_empty(),
            "no installations resolve when every slot is absent"
        );
    }

    /// scan-orchestration: a same-named skill from a Codex root and a
    /// Claude Code root consolidates into one `Component` with two
    /// `Location`s — no client discriminator, consolidation unmodified
    /// (design §2, §6.2).
    #[test]
    fn codex_and_claude_same_named_skill_consolidates_into_one_component() {
        let report = scan_for(
            &fixture_home("codex-claude-same-skill"),
            HostPlatform::Windows,
        );

        let shared: Vec<_> = report
            .components
            .iter()
            .filter(|c| c.name == "shared")
            .collect();
        assert_eq!(shared.len(), 1, "one Component for the shared identity");
        assert_eq!(shared[0].locations.len(), 2, "one Location per root");
    }

    /// scan-orchestration: an MCP server named `github`, configured in all
    /// three clients with a different command each, consolidates into one
    /// `Component { kind: Mcp }` carrying three `Location`s, each retaining
    /// its own `McpTransport` (`specs/scan-orchestration/spec.md:43-47`,
    /// design §5.3, §8). Mirrors `mcp_redaction.rs`'s anchor 0.9, exercised
    /// here through the real orchestrator instead of the three adapters
    /// called directly.
    #[test]
    fn mcp_same_name_three_clients_consolidates_into_one_component_three_transports() {
        let report = scan_for(
            &fixture_home("mcp-same-name-three-clients"),
            HostPlatform::Windows,
        );

        let github: Vec<_> = report
            .components
            .iter()
            .filter(|c| c.name == "github")
            .collect();
        assert_eq!(github.len(), 1, "one Component for the shared identity");
        assert_eq!(github[0].locations.len(), 3, "one Location per client");
        assert_eq!(github[0].kind, crate::model::ComponentKind::Mcp);
    }

    /// scan-orchestration: a malformed Codex agent `.toml` file does not
    /// abort the scan — one `Error` `ScanIssue` for it, every other
    /// adapter's valid results still present (design §7.1).
    #[test]
    fn malformed_codex_agent_does_not_abort_the_scan() {
        let report = scan_for(&fixture_home("corrupt-codex-agent"), HostPlatform::Windows);

        let broken_path = fixture_home("corrupt-codex-agent")
            .join(".codex")
            .join("agents")
            .join("broken.toml");
        assert!(report.issues.iter().any(|issue| {
            issue.severity == IssueSeverity::Error && issue.path.as_ref() == Some(&broken_path)
        }));
        assert!(report
            .components
            .iter()
            .any(|component| component.name == "codex-good"));
        assert!(report
            .components
            .iter()
            .any(|component| component.name == "valid-agent"));
    }

    #[test]
    fn reference_fixture_is_fast_and_read_only() {
        let home = fixture_home("reference-volume");
        let before = fixture_tree_snapshot(&home);

        let report = scan_for(&home, HostPlatform::Windows);

        assert!(report.duration_ms < 2_000);
        assert!(!report.components.is_empty());
        assert_eq!(before, fixture_tree_snapshot(&home));
    }

    #[test]
    fn reference_fixture_snapshot_tracks_files_directories_and_metadata() {
        let home = fixture_home("reference-volume");

        let snapshot = fixture_tree_snapshot(&home);

        assert!(snapshot
            .iter()
            .any(|entry| matches!(entry.kind, FixtureEntryKind::Directory)));
        assert!(snapshot
            .iter()
            .any(|entry| matches!(entry.kind, FixtureEntryKind::File)));
        assert!(snapshot.iter().all(|entry| entry.path.is_relative()));
        assert_snapshot_captures_platform_permission_evidence(&home, &snapshot);
        assert!(
            snapshot
                .iter()
                .filter(|entry| entry.file_hash.is_some())
                .count()
                >= 2
        );
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct FixtureEntrySnapshot {
        path: PathBuf,
        kind: FixtureEntryKind,
        file_len: Option<u64>,
        file_hash: Option<u64>,
        permissions: PermissionEvidence,
        modified: SystemTime,
        symlink_target: Option<PathBuf>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum FixtureEntryKind {
        Directory,
        File,
        Symlink,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum PermissionEvidence {
        #[cfg(unix)]
        UnixMode(u32),
        #[cfg(windows)]
        WindowsAttributes(u32),
        #[cfg(not(any(unix, windows)))]
        Readonly(bool),
    }

    #[cfg(unix)]
    fn assert_snapshot_captures_platform_permission_evidence(
        root: &Path,
        snapshot: &[FixtureEntrySnapshot],
    ) {
        use std::os::unix::fs::PermissionsExt;

        let entry = snapshot
            .iter()
            .find(|entry| matches!(entry.kind, FixtureEntryKind::File))
            .expect("fixture snapshot must include at least one file");
        let metadata = fs::symlink_metadata(root.join(&entry.path))
            .expect("fixture metadata must be readable for permission evidence");
        let PermissionEvidence::UnixMode(mode) = entry.permissions;

        assert_eq!(mode, metadata.permissions().mode());
        assert_ne!(
            mode & 0o400,
            0,
            "fixture file must retain owner-read evidence"
        );
    }

    #[cfg(windows)]
    fn assert_snapshot_captures_platform_permission_evidence(
        root: &Path,
        snapshot: &[FixtureEntrySnapshot],
    ) {
        use std::os::windows::fs::MetadataExt;

        let entry = snapshot
            .iter()
            .find(|entry| matches!(entry.kind, FixtureEntryKind::Directory))
            .expect("fixture snapshot must include at least one directory");
        let metadata = fs::symlink_metadata(root.join(&entry.path))
            .expect("fixture metadata must be readable for permission evidence");
        let PermissionEvidence::WindowsAttributes(attributes) = entry.permissions;

        assert_eq!(attributes, metadata.file_attributes());
        assert_ne!(
            attributes & 0x10,
            0,
            "directory evidence must retain FILE_ATTRIBUTE_DIRECTORY"
        );
    }

    #[cfg(not(any(unix, windows)))]
    fn assert_snapshot_captures_platform_permission_evidence(
        root: &Path,
        snapshot: &[FixtureEntrySnapshot],
    ) {
        let entry = snapshot
            .iter()
            .find(|entry| matches!(entry.kind, FixtureEntryKind::File))
            .expect("fixture snapshot must include at least one file");
        let metadata = fs::symlink_metadata(root.join(&entry.path))
            .expect("fixture metadata must be readable for permission evidence");
        let PermissionEvidence::Readonly(readonly) = entry.permissions;

        assert_eq!(readonly, metadata.permissions().readonly());
    }
    fn fixture_tree_snapshot(root: &Path) -> Vec<FixtureEntrySnapshot> {
        let mut entries = Vec::new();
        for entry in walkdir::WalkDir::new(root)
            .follow_links(false)
            .sort_by_file_name()
        {
            let entry = entry.expect("fixture tree must be fully readable");
            let path = entry.path();
            let metadata = fs::symlink_metadata(path).expect("fixture metadata must be readable");
            let file_type = metadata.file_type();
            let kind = if file_type.is_symlink() {
                FixtureEntryKind::Symlink
            } else if file_type.is_dir() {
                FixtureEntryKind::Directory
            } else {
                FixtureEntryKind::File
            };
            let relative_path = path
                .strip_prefix(root)
                .expect("fixture entry must be inside the fixture root")
                .to_path_buf();
            let (file_len, file_hash) = if file_type.is_file() {
                (Some(metadata.len()), Some(stable_file_hash(path)))
            } else {
                (None, None)
            };
            let symlink_target = if file_type.is_symlink() {
                Some(fs::read_link(path).expect("fixture symlink target must be readable"))
            } else {
                None
            };

            entries.push(FixtureEntrySnapshot {
                path: relative_path,
                kind,
                file_len,
                file_hash,
                permissions: permission_evidence(&metadata),
                modified: metadata
                    .modified()
                    .expect("fixture modified timestamp must be available for CA-16 evidence"),
                symlink_target,
            });
        }
        entries
    }

    #[cfg(unix)]
    fn permission_evidence(metadata: &fs::Metadata) -> PermissionEvidence {
        use std::os::unix::fs::PermissionsExt;

        PermissionEvidence::UnixMode(metadata.permissions().mode())
    }

    #[cfg(windows)]
    fn permission_evidence(metadata: &fs::Metadata) -> PermissionEvidence {
        use std::os::windows::fs::MetadataExt;

        PermissionEvidence::WindowsAttributes(metadata.file_attributes())
    }

    #[cfg(not(any(unix, windows)))]
    fn permission_evidence(metadata: &fs::Metadata) -> PermissionEvidence {
        PermissionEvidence::Readonly(metadata.permissions().readonly())
    }

    fn stable_file_hash(path: &Path) -> u64 {
        const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
        const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

        let mut file = fs::File::open(path).expect("fixture file must be readable");
        let mut hash = FNV_OFFSET;
        let mut buffer = [0; 8 * 1024];
        loop {
            let read = file
                .read(&mut buffer)
                .expect("fixture file bytes must be readable");
            if read == 0 {
                return hash;
            }
            for byte in &buffer[..read] {
                hash ^= u64::from(*byte);
                hash = hash.wrapping_mul(FNV_PRIME);
            }
        }
    }
}
