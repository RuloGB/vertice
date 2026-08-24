//! Home directory resolution and the four fixed user skill roots.
//!
//! `home_dir` is the ONLY ambient-environment read in the crate. Every
//! other function — here and in [`crate::skills`] — takes `home` as a
//! parameter, which is what makes fixtures possible: no test ever reads the
//! author's machine, and no environment variable is set or read by any
//! test. See `design.md` §3/§4.

use std::path::{Path, PathBuf};

use crate::model::{ScanError, SearchRoot, SearchRootId, SearchRootKind, SearchRootStatus};

/// A resolved root together with every path that MUST be scanned for it.
/// One entry per element except the OpenCode root, which carries two: the
/// canonical plural path and the singular alias, scanned as one logical
/// root under one id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRoot {
    pub root: SearchRoot,
    pub scan_paths: Vec<PathBuf>,
}

/// Resolve the current user's home directory. The sole ambient-environment
/// read in the crate — every other function in `roots`/`skills` takes
/// `home` as a parameter instead.
///
/// Fails when `std::env::home_dir()` returns `None`, or when the resolved
/// path is not representable as UTF-8 (a non-UTF-8 home makes every
/// `SearchRoot::path` derived from it un-serializable — design §7.2).
pub fn home_dir() -> Result<PathBuf, ScanError> {
    resolve_home(std::env::home_dir())
}

/// Testable core of [`home_dir`], split out so the failure paths can be
/// exercised without touching the real environment (design §7.2 / task
/// 2.8).
fn resolve_home(raw: Option<PathBuf>) -> Result<PathBuf, ScanError> {
    let home = raw.ok_or_else(|| ScanError::Internal {
        reason: "could not resolve the current user's home directory".to_string(),
    })?;

    if home.to_str().is_none() {
        return Err(ScanError::Internal {
            reason: "home directory path is not valid UTF-8".to_string(),
        });
    }

    Ok(home)
}

/// Resolve the four fixed user skill roots under `home`. Always returns
/// exactly four entries — the array length is the CA-6/CA-14 guarantee,
/// expressed in the type rather than asserted in prose. Root ids are
/// hardcoded, never derived from `home`, so fixture assertions stay
/// machine-independent.
///
/// No OS config-directory convention is consulted anywhere: every root is
/// `home` plus a hardcoded relative suffix, built with per-segment
/// `PathBuf::push` (design §9). `codex-skills` is the fourth and last entry
/// (design §6.1) — it lands at index 3 of `consolidate::ROOT_ORDER`'s eight,
/// not last overall (design §0's correction to the proposal).
pub fn skill_roots(home: &Path) -> [ResolvedRoot; 4] {
    [
        resolve_single(
            home,
            "claude-skills",
            SearchRootKind::Skill,
            [".claude", "skills"],
        ),
        resolve_single(
            home,
            "agents-skills",
            SearchRootKind::Skill,
            [".agents", "skills"],
        ),
        resolve_opencode(home),
        resolve_single(
            home,
            "codex-skills",
            SearchRootKind::Skill,
            [".codex", "skills"],
        ),
    ]
}

/// Resolve the Claude Code agent roots under `home`. Two entries: the
/// on-disk root that is walked, and the embedded pseudo-root that is only
/// probed (design §3). Root ids are hardcoded, never derived from `home`.
pub fn agent_roots(home: &Path) -> [ResolvedRoot; 2] {
    let agents = resolve_single(
        home,
        "claude-agents",
        SearchRootKind::Agent,
        [".claude", "agents"],
    );

    let mut embedded_path = home.to_path_buf();
    embedded_path.push(".claude");
    let embedded_status = probe(&embedded_path);

    let embedded = ResolvedRoot {
        root: SearchRoot {
            id: SearchRootId("claude-embedded-agents".to_string()),
            path: embedded_path,
            kind: SearchRootKind::Agent,
            status: embedded_status,
        },
        // Probed, never walked (design §3).
        scan_paths: vec![],
    };

    [agents, embedded]
}

/// Resolve a root with exactly one scan path (Claude Code, Agents).
fn resolve_single(home: &Path, id: &str, kind: SearchRootKind, suffix: [&str; 2]) -> ResolvedRoot {
    let mut path = home.to_path_buf();
    for segment in suffix {
        path.push(segment);
    }

    let status = probe(&path);

    ResolvedRoot {
        root: SearchRoot {
            id: SearchRootId(id.to_string()),
            path: path.clone(),
            kind,
            status,
        },
        scan_paths: vec![path],
    }
}

/// Resolve the OpenCode root. `~/.config/opencode/skills/` (plural) is the
/// canonical path carried by the `SearchRoot`; `~/.config/opencode/skill/`
/// (singular) is a second scan path under the same id. `status` is `Found`
/// if either directory exists.
fn resolve_opencode(home: &Path) -> ResolvedRoot {
    let mut plural = home.to_path_buf();
    plural.push(".config");
    plural.push("opencode");
    plural.push("skills");

    let mut singular = home.to_path_buf();
    singular.push(".config");
    singular.push("opencode");
    singular.push("skill");

    let status = match (probe(&plural), probe(&singular)) {
        (SearchRootStatus::Found, _) | (_, SearchRootStatus::Found) => SearchRootStatus::Found,
        _ => SearchRootStatus::NotFound,
    };

    ResolvedRoot {
        root: SearchRoot {
            id: SearchRootId("opencode-skills".to_string()),
            path: plural.clone(),
            kind: SearchRootKind::Skill,
            status,
        },
        scan_paths: vec![plural, singular],
    }
}

/// Resolve the OpenCode agent config root under `home`. `SearchRoot.path`
/// carries `~/.config/opencode/opencode.json` (the merge base, design §3);
/// `scan_paths` carries the base then the `.jsonc` overlay, in merge order.
/// `status` is `Found` if EITHER file exists. Root id is hardcoded, never
/// path-derived. Structurally a sibling of `resolve_opencode` above: same
/// two-`push` path construction, same `match (probe(a), probe(b))` status
/// fold, same `scan_paths` vector — `probe` is reused unchanged.
pub fn opencode_agent_root(home: &Path) -> ResolvedRoot {
    let mut base = home.to_path_buf();
    base.push(".config");
    base.push("opencode");
    base.push("opencode.json");

    let mut overlay = home.to_path_buf();
    overlay.push(".config");
    overlay.push("opencode");
    overlay.push("opencode.jsonc");

    let status = match (probe(&base), probe(&overlay)) {
        (SearchRootStatus::Found, _) | (_, SearchRootStatus::Found) => SearchRootStatus::Found,
        _ => SearchRootStatus::NotFound,
    };

    ResolvedRoot {
        root: SearchRoot {
            id: SearchRootId("opencode-agents".to_string()),
            path: base.clone(),
            kind: SearchRootKind::Agent,
            status,
        },
        scan_paths: vec![base, overlay],
    }
}

/// Resolve the Codex agent root under `home`: a single on-disk directory,
/// walked exactly like `claude-agents`. Mirrors `opencode_agent_root`'s
/// public single-root shape, but built from `resolve_single` because it is
/// one plain directory with one scan path — no alias, no merge order
/// (design §6.1). Emits a `SearchRoot` with `kind: Agent`.
pub fn codex_agent_root(home: &Path) -> ResolvedRoot {
    resolve_single(
        home,
        "codex-agents",
        SearchRootKind::Agent,
        [".codex", "agents"],
    )
}

/// Probe whether `path` exists on disk. Returns `NotFound` only for
/// `ErrorKind::NotFound`; any other outcome (the path existing, or a
/// probe error of another kind) is `Found` — `NotFound` is a positive claim
/// about the machine and the safest default otherwise is "we found
/// something, or something went wrong inspecting it" (design §7).
fn probe(path: &Path) -> SearchRootStatus {
    match std::fs::symlink_metadata(path) {
        Ok(_) => SearchRootStatus::Found,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => SearchRootStatus::NotFound,
        Err(_) => SearchRootStatus::Found,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Alias grouping: `.config/opencode/skill/` and
    /// `.config/opencode/skills/` resolve to one `SearchRoot` id, carried as
    /// two entries in `scan_paths` under that same id.
    #[test]
    fn opencode_alias_paths_share_one_root_id() {
        let home = PathBuf::from("/home/example");

        let resolved = resolve_opencode(&home);

        assert_eq!(
            resolved.root.id,
            SearchRootId("opencode-skills".to_string())
        );
        assert_eq!(resolved.scan_paths.len(), 2);
        assert!(resolved.scan_paths.iter().any(|p| p.ends_with("skills")));
        assert!(resolved.scan_paths.iter().any(|p| p.ends_with("skill")));
    }

    /// Requirement: OpenCode Agent Root Resolves Under The Home Directory
    /// To Two Config Files (T6, design §3/§5.1). `scan_paths` order is the
    /// single source of merge order and MUST be asserted, not assumed.
    #[test]
    fn opencode_agent_root_resolves_to_two_config_files_in_merge_order() {
        let home = PathBuf::from("/home/example");

        let resolved = opencode_agent_root(&home);

        assert_eq!(
            resolved.root.id,
            SearchRootId("opencode-agents".to_string())
        );
        assert_eq!(resolved.root.kind, SearchRootKind::Agent);
        assert!(resolved.root.path.ends_with("opencode.json"));

        assert_eq!(resolved.scan_paths.len(), 2);
        assert!(resolved.scan_paths[0].ends_with("opencode.json"));
        assert!(resolved.scan_paths[1].ends_with("opencode.jsonc"));
    }

    /// `opencode_agent_root`'s id is hardcoded, never path-derived, exactly
    /// like the other roots in this module.
    #[test]
    fn opencode_agent_root_id_is_stable_and_never_path_derived() {
        let first = opencode_agent_root(&PathBuf::from("/home/alice"));
        let second = opencode_agent_root(&PathBuf::from("/home/bob"));

        assert_eq!(first.root.id, second.root.id);
        assert_eq!(first.root.id, SearchRootId("opencode-agents".to_string()));
    }

    /// Root ids are hardcoded, never path-derived — the same four ids
    /// appear regardless of `home`.
    #[test]
    fn root_ids_are_stable_and_never_path_derived() {
        let first = skill_roots(&PathBuf::from("/home/alice"));
        let second = skill_roots(&PathBuf::from("/home/bob"));

        let ids = |roots: &[ResolvedRoot; 4]| -> Vec<SearchRootId> {
            roots.iter().map(|r| r.root.id.clone()).collect()
        };

        assert_eq!(ids(&first), ids(&second));
        assert_eq!(
            ids(&first),
            vec![
                SearchRootId("claude-skills".to_string()),
                SearchRootId("agents-skills".to_string()),
                SearchRootId("opencode-skills".to_string()),
                SearchRootId("codex-skills".to_string()),
            ]
        );
    }

    /// `skill_roots` returns exactly four roots for any `home`, whether or
    /// not any of them exist on disk.
    #[test]
    fn skill_roots_always_returns_exactly_four_entries() {
        let roots = skill_roots(&PathBuf::from("/definitely/does/not/exist"));

        assert_eq!(roots.len(), 4);
        assert!(roots
            .iter()
            .all(|r| r.root.status == SearchRootStatus::NotFound));
    }

    /// `codex_agent_root`'s id is hardcoded, never path-derived, exactly
    /// like `opencode_agent_root`'s.
    #[test]
    fn codex_agent_root_id_is_stable_and_never_path_derived() {
        let first = codex_agent_root(&PathBuf::from("/home/alice"));
        let second = codex_agent_root(&PathBuf::from("/home/bob"));

        assert_eq!(first.root.id, second.root.id);
        assert_eq!(first.root.id, SearchRootId("codex-agents".to_string()));
        assert_eq!(first.root.kind, SearchRootKind::Agent);
    }

    /// `agent_roots` returns exactly two entries: the walked on-disk root
    /// and the probed-only embedded pseudo-root, both with hardcoded,
    /// never-path-derived ids (design §3, task 1.1).
    #[test]
    fn agent_roots_returns_exactly_two_entries_with_stable_ids() {
        let home = PathBuf::from("/home/example");

        let [agents, embedded] = agent_roots(&home);

        assert_eq!(agents.root.id, SearchRootId("claude-agents".to_string()));
        let mut expected_agents_path = home.clone();
        expected_agents_path.push(".claude");
        expected_agents_path.push("agents");
        assert_eq!(agents.root.path, expected_agents_path);
        assert_eq!(agents.root.kind, SearchRootKind::Agent);
        assert_eq!(agents.scan_paths, vec![expected_agents_path]);

        assert_eq!(
            embedded.root.id,
            SearchRootId("claude-embedded-agents".to_string())
        );
        let mut expected_embedded_path = home.clone();
        expected_embedded_path.push(".claude");
        assert_eq!(embedded.root.path, expected_embedded_path);
        assert_eq!(embedded.root.kind, SearchRootKind::Agent);
        assert!(
            embedded.scan_paths.is_empty(),
            "the embedded pseudo-root is probed but never walked"
        );
    }

    /// Root ids returned by `agent_roots` never change with `home`.
    #[test]
    fn agent_root_ids_are_stable_and_never_path_derived() {
        let first = agent_roots(&PathBuf::from("/home/alice"));
        let second = agent_roots(&PathBuf::from("/home/bob"));

        let ids = |roots: &[ResolvedRoot; 2]| -> Vec<SearchRootId> {
            roots.iter().map(|r| r.root.id.clone()).collect()
        };

        assert_eq!(ids(&first), ids(&second));
        assert_eq!(
            ids(&first),
            vec![
                SearchRootId("claude-agents".to_string()),
                SearchRootId("claude-embedded-agents".to_string()),
            ]
        );
    }

    /// Task 2.7 (`.gitkeep` tripwire, status half): the empty-alias fixture
    /// resolves to `status: Found` via the singular scan path.
    #[test]
    fn empty_alias_root_status_is_found() {
        let mut home = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        home.push("tests");
        home.push("fixtures");
        home.push("roots");
        home.push("empty-alias");

        let resolved = resolve_opencode(&home);

        assert_eq!(resolved.root.status, SearchRootStatus::Found);
    }

    /// Task 2.8: `home_dir` resolution failure is testable without touching
    /// the real environment.
    #[test]
    fn resolve_home_fails_when_raw_is_none() {
        let err = resolve_home(None).expect_err("None must fail");

        assert!(matches!(err, ScanError::Internal { .. }));
    }

    #[cfg(unix)]
    #[test]
    fn resolve_home_fails_on_non_utf8_path() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        let raw = OsString::from_vec(vec![0x66, 0x6f, 0x80, 0x6f]);
        let err = resolve_home(Some(PathBuf::from(raw))).expect_err("non-UTF-8 home must fail");

        assert!(matches!(err, ScanError::Internal { .. }));
    }
}
