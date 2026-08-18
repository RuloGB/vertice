//! Home directory resolution and the three fixed user skill roots.
//!
//! `home_dir` is the ONLY ambient-environment read in the crate. Every
//! other function — here and in [`crate::skills`] — takes `home` as a
//! parameter, which is what makes fixtures possible: no test ever reads the
//! author's machine, and no environment variable is set or read by any
//! test. See `design.md` §3/§4.
//!
//! RED: signatures only, bodies not yet implemented (task 2.1/2.2).

use std::path::{Path, PathBuf};

use crate::model::{ScanError, SearchRoot};

/// A resolved root together with every path that MUST be scanned for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRoot {
    pub root: SearchRoot,
    pub scan_paths: Vec<PathBuf>,
}

/// Resolve the current user's home directory. See task 2.2.
pub fn home_dir() -> Result<PathBuf, ScanError> {
    todo!("implemented in task 2.2")
}

/// Resolve the three fixed user skill roots under `home`. See task 2.2.
pub fn skill_roots(_home: &Path) -> [ResolvedRoot; 3] {
    todo!("implemented in task 2.2")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::SearchRootId;

    #[test]
    fn opencode_alias_paths_share_one_root_id() {
        let home = PathBuf::from("/home/example");

        let resolved = &skill_roots(&home)[2];

        assert_eq!(
            resolved.root.id,
            SearchRootId("opencode-skills".to_string())
        );
        assert_eq!(resolved.scan_paths.len(), 2);
    }

    #[test]
    fn root_ids_are_stable_and_never_path_derived() {
        let first = skill_roots(&PathBuf::from("/home/alice"));
        let second = skill_roots(&PathBuf::from("/home/bob"));

        let ids = |roots: &[ResolvedRoot; 3]| -> Vec<SearchRootId> {
            roots.iter().map(|r| r.root.id.clone()).collect()
        };

        assert_eq!(ids(&first), ids(&second));
    }

    #[test]
    fn skill_roots_always_returns_exactly_three_entries() {
        let roots = skill_roots(&PathBuf::from("/definitely/does/not/exist"));

        assert_eq!(roots.len(), 3);
    }
}
