//! Enforces `openspec/changes/skill-frontmatter-reader/design.md` §11:
//! `yaml.rs` is the only module in `vertice-core` allowed to import
//! `serde_norway` directly. Every other module MUST go through
//! `vertice_core::yaml::from_str`.
//!
//! This is a **textual** check, not a semantic one. It can be fooled by a
//! re-export alias or a macro, and it will false-positive on a doc comment
//! that writes the crate name in path form (e.g. `serde_norway::`). Both are
//! acceptable: the target is accidental breakage, not an adversary, and
//! false positives are loud and cheap to fix. Any module documenting this
//! constraint MUST do so in prose only, without writing `serde_norway::` or
//! `use serde_norway`.
//!
//! Passes vacuously today: `frontmatter.rs` (T3, Work Unit 2 of this change)
//! does not exist yet, so `yaml.rs` is the only `.rs` file under `src/`. It
//! is re-run in Work Unit 2 once `frontmatter.rs` exists as a real sibling
//! module.

use std::fs;
use std::path::{Path, PathBuf};

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir)
        .unwrap_or_else(|err| panic!("failed to read directory {}: {err}", dir.display()));

    for entry in entries {
        let entry = entry.expect("failed to read a directory entry");
        let path = entry.path();

        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

fn is_yaml_module(path: &Path, src_dir: &Path) -> bool {
    path.parent() == Some(src_dir)
        && path.file_name().and_then(|name| name.to_str()) == Some("yaml.rs")
}

#[test]
fn only_yaml_module_imports_serde_norway() {
    let mut src_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    src_dir.push("src");

    let mut rs_files = Vec::new();
    collect_rs_files(&src_dir, &mut rs_files);

    assert!(
        !rs_files.is_empty(),
        "expected at least one .rs file under {}",
        src_dir.display()
    );

    let mut offenders = Vec::new();
    for path in &rs_files {
        if is_yaml_module(path, &src_dir) {
            continue;
        }

        let content = fs::read_to_string(path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));

        if content.contains("use serde_norway") || content.contains("serde_norway::") {
            offenders.push(path.display().to_string());
        }
    }

    assert!(
        offenders.is_empty(),
        "only yaml.rs may import serde_norway directly; found the textual pattern \
         `use serde_norway` or `serde_norway::` in: {offenders:?}. If this is a doc \
         comment naming the crate in path form, reword it in prose only."
    );
}
