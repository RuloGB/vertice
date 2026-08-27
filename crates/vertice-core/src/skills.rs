//! Skill discovery: walks the three fixed user roots and assembles
//! `Component`/`ScanIssue` values from any discovered `SKILL.md`.
//!
//! `scan` is infallible: it takes an already-resolved `home` and returns an
//! owned [`SkillScan`]. The only failure mode that stops the whole scan —
//! home directory resolution — lives in [`crate::roots::home_dir`], one
//! call earlier, and out of this function entirely (design §7.2).

use std::path::Path;

use walkdir::WalkDir;

use crate::frontmatter::{self, SkillFrontmatter};
use crate::model::{
    ClientKind, Component, ComponentId, ComponentKind, IssueSeverity, Location, LocationOrigin,
    ScanIssue, Scope, SearchRoot, SearchRootId,
};
use crate::roots::{self, ResolvedRoot};

/// Owned result of one scan. Three heterogeneous collections make a tuple
/// unreadable and re-orderable by mistake at every call site. NOT a model
/// type: no `Serialize`/`TS`, the same non-model status
/// `frontmatter::SkillFrontmatter` has — T9 destructures this into a
/// `ScanReport`.
#[derive(Debug, Clone, PartialEq)]
pub struct SkillScan {
    pub roots: Vec<SearchRoot>,
    pub components: Vec<Component>,
    pub issues: Vec<ScanIssue>,
}

/// Scan the three fixed user skill roots under `home`. Read-only:
/// `roots::skill_roots`'s probe, `walkdir`'s directory reads, and
/// `frontmatter::read`'s file reads are the complete disk surface — no
/// write of any kind, anywhere.
pub fn scan(home: &Path) -> SkillScan {
    let mut roots_out = Vec::with_capacity(3);
    let mut components = Vec::new();
    let mut issues = Vec::new();

    for ResolvedRoot { root, scan_paths } in roots::skill_roots(home) {
        for scan_path in &scan_paths {
            walk_one(
                scan_path,
                &root.id,
                root.client,
                &mut components,
                &mut issues,
            );
        }
        roots_out.push(root);
    }

    SkillScan {
        roots: roots_out,
        components,
        issues,
    }
}

/// Walk one scan path (one of a root's `scan_paths`) and accumulate
/// components/issues into the caller's buffers. Never aborts the overall
/// scan: every error class produces at most one `ScanIssue` and control
/// returns to the caller so sibling roots and sibling entries are still
/// visited (design §7).
fn walk_one(
    scan_path: &Path,
    root_id: &SearchRootId,
    client: Option<ClientKind>,
    components: &mut Vec<Component>,
    issues: &mut Vec<ScanIssue>,
) {
    let metadata = match std::fs::symlink_metadata(scan_path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return,
        Err(err) => {
            issues.push(ScanIssue {
                severity: IssueSeverity::Error,
                path: Some(scan_path.to_path_buf()),
                reason: format!("could not inspect search root: {err}"),
            });
            return;
        }
    };

    if !metadata.is_dir() {
        issues.push(ScanIssue {
            severity: IssueSeverity::Error,
            path: Some(scan_path.to_path_buf()),
            reason: "search root is not a directory".to_string(),
        });
        return;
    }

    // Recursive, unbounded depth; symlinks are never followed — written
    // explicitly, never left to the crate default (design §6). Ordering is
    // deterministic for debugging and diffs; correctness does not depend
    // on it.
    for entry in WalkDir::new(scan_path)
        .follow_links(false)
        .sort_by_file_name()
    {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                let path = err
                    .path()
                    .map(Path::to_path_buf)
                    .unwrap_or_else(|| scan_path.to_path_buf());
                issues.push(ScanIssue {
                    severity: IssueSeverity::Error,
                    path: Some(path),
                    reason: format!("could not read directory entry: {err}"),
                });
                continue;
            }
        };

        if entry.file_type().is_dir() || entry.file_name() != "SKILL.md" {
            continue;
        }

        let path = entry.into_path();

        if let Err(lossy) = ensure_utf8_path(&path) {
            issues.push(ScanIssue {
                severity: IssueSeverity::Error,
                path: None,
                reason: format!("skipped a file whose path is not valid UTF-8: {lossy}"),
            });
            continue;
        }

        match frontmatter::read::<SkillFrontmatter>(&path) {
            Ok(fm) => components.push(Component {
                id: ComponentId::derive(ComponentKind::Skill, &fm.name),
                name: fm.name,
                kind: ComponentKind::Skill,
                description: fm.description,
                scope: Scope::User,
                locations: vec![Location {
                    path: Some(path),
                    root: root_id.clone(),
                    origin: LocationOrigin::File,
                    mcp_transport: None,
                    client,
                }],
                provenance_hint: None,
            }),
            Err(issue) => issues.push(escalate(issue)),
        }
    }
}

/// Every failure to parse a discovered `SKILL.md` is escalated to
/// `IssueSeverity::Error`, uniformly — `path`/`reason` untouched. Detection
/// under a skills root is "if there is a `SKILL.md`, it is a skill", so
/// every failure to parse one is a skill missing from the user's inventory
/// (design §5).
fn escalate(issue: ScanIssue) -> ScanIssue {
    ScanIssue {
        severity: IssueSeverity::Error,
        ..issue
    }
}

/// Check whether a discovered path is representable as UTF-8, returning the
/// lossy display string on failure. Split out from the walk loop so the
/// conversion itself is directly unit-testable (design §7.1) — no portable
/// fixture can produce a non-UTF-8 path on disk.
fn ensure_utf8_path(path: &Path) -> Result<(), String> {
    if path.to_str().is_some() {
        Ok(())
    } else {
        Err(path.to_string_lossy().into_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::IssueSeverity;

    /// Every T3 severity class, once escalated, surfaces as `Error` — the
    /// deferred-to-T16 BOM/`NoOpeningFence` case included (design §5).
    #[test]
    fn escalate_maps_every_severity_to_error() {
        for original in [IssueSeverity::Warning, IssueSeverity::Error] {
            let issue = ScanIssue {
                severity: original,
                path: Some(std::path::PathBuf::from("/some/SKILL.md")),
                reason: "no frontmatter block: file does not begin with a --- fence".to_string(),
            };

            let escalated = escalate(issue.clone());

            assert_eq!(escalated.severity, IssueSeverity::Error);
            assert_eq!(escalated.path, issue.path);
            assert_eq!(escalated.reason, issue.reason);
        }
    }

    #[cfg(unix)]
    #[test]
    fn non_utf8_path_component_fails_the_utf8_check() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        let invalid = OsStr::from_bytes(&[0x66, 0x6f, 0x80, 0x6f]);
        let path = Path::new(invalid);

        assert!(ensure_utf8_path(path).is_err());
    }

    #[test]
    fn utf8_path_passes_the_utf8_check() {
        let path = Path::new("/home/user/.claude/skills/demo/SKILL.md");

        assert!(ensure_utf8_path(path).is_ok());
    }
}
