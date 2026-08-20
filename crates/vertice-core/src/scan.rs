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
    let installations = crate::installations::scan_for(home, platform);

    let mut roots_scanned = skills.roots;
    roots_scanned.extend(agents.roots);
    roots_scanned.extend(opencode_agents.roots);

    let mut components = skills.components;
    components.extend(agents.components);
    components.extend(opencode_agents.components);

    let mut issues = skills.issues;
    issues.extend(agents.issues);
    issues.extend(opencode_agents.issues);
    issues.extend(installations.issues);
    append_missing_root_issues(&roots_scanned, &mut issues);

    let duration_ms = u32::try_from(started.elapsed().as_millis()).unwrap_or(u32::MAX);

    ScanReport {
        components: crate::consolidate::consolidate(components),
        installations: installations.installations,
        roots_scanned,
        issues,
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
    use std::path::{Path, PathBuf};

    use super::*;
    use crate::installations::HostPlatform;
    use crate::model::{IssueSeverity, SearchRootStatus};

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

        assert_eq!(report.roots_scanned.len(), 6);
        assert_eq!(report.installations.len(), 3);
        assert_eq!(report.components.len(), 10);
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

        assert_eq!(report.roots_scanned.len(), 6);
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
            6
        );
        assert_eq!(
            report
                .issues
                .iter()
                .filter(|issue| issue.reason.ends_with("not detected"))
                .count(),
            3
        );
    }

    #[test]
    fn reference_fixture_is_fast_and_read_only() {
        let home = fixture_home("reference-volume");
        let before = fixture_tree_bytes(&home);

        let report = scan_for(&home, HostPlatform::Windows);

        assert!(report.duration_ms < 2_000);
        assert!(!report.components.is_empty());
        assert_eq!(before, fixture_tree_bytes(&home));
    }

    fn fixture_tree_bytes(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
        let mut out = Vec::new();
        for entry in walkdir::WalkDir::new(root)
            .follow_links(false)
            .sort_by_file_name()
        {
            let entry = entry.expect("fixture tree must be fully readable");
            if entry.file_type().is_file() {
                let bytes = std::fs::read(entry.path()).expect("fixture file must be readable");
                out.push((entry.into_path(), bytes));
            }
        }
        out
    }
}
