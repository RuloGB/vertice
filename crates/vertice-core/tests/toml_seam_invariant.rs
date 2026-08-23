//! Enforces `openspec/changes/2026-08-23-add-codex-client-support/design.md`
//! §5.2: `toml.rs` is the only module in `vertice-core` allowed to name the
//! `toml_seam` dependency alias directly. Every other module MUST go through
//! `vertice_core::toml::from_str`. A line-for-line analogue of
//! `tests/yaml_seam_invariant.rs`.
//!
//! This is a **textual** check, not a semantic one. It can be fooled by a
//! re-export alias or a macro, and it will false-positive on a doc comment
//! that writes the alias name in path form (e.g. `toml_seam::`). Both are
//! acceptable: the target is accidental breakage, not an adversary, and
//! false positives are loud and cheap to fix. Any module documenting this
//! constraint MUST do so in prose only, without writing `toml_seam::` or
//! `use toml_seam`.

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

fn is_toml_module(path: &Path, src_dir: &Path) -> bool {
    path.parent() == Some(src_dir)
        && path.file_name().and_then(|name| name.to_str()) == Some("toml.rs")
}

#[test]
fn only_toml_module_imports_toml_seam() {
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
        if is_toml_module(path, &src_dir) {
            continue;
        }

        let content = fs::read_to_string(path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));

        if content.contains("use toml_seam") || content.contains("toml_seam::") {
            offenders.push(path.display().to_string());
        }
    }

    assert!(
        offenders.is_empty(),
        "only toml.rs may import toml_seam directly; found the textual pattern \
         `use toml_seam` or `toml_seam::` in: {offenders:?}. If this is a doc \
         comment naming the alias in path form, reword it in prose only."
    );
}
