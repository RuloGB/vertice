//! Enforces `openspec/changes/2026-08-23-add-codex-client-support/design.md`
//! §10.4, item 2: no file under `src/` may reference `version.json` or
//! `latest_version`. `~/.codex/version.json` is an update-availability
//! cache, not a version source (design §3.1), and with no oracle for Codex
//! (design §0, V4), this textual check is the only mechanical guarantee that
//! it never becomes one.

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

#[test]
fn no_source_file_references_version_json_or_latest_version() {
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
        let content = fs::read_to_string(path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));

        if content.contains("version.json") || content.contains("latest_version") {
            offenders.push(path.display().to_string());
        }
    }

    assert!(
        offenders.is_empty(),
        "version.json is an update-availability cache, never a version source; found the \
         forbidden pattern in: {offenders:?}"
    );
}
