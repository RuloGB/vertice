use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
struct AuditReport {
    reviewed_files: usize,
    covered_classes: Vec<&'static str>,
    covered_patterns: Vec<&'static str>,
    findings: Vec<String>,
    static_proof_is_limited: bool,
}

#[test]
fn core_source_audit_covers_all_filesystem_mutation_classes() {
    let report = audit_core_mutation_surface();

    assert!(
        report.reviewed_files >= 8,
        "audit must cover the core source tree"
    );
    assert!(report.covered_classes.contains(&"write"));
    assert!(report.covered_classes.contains(&"truncate"));
    assert!(report.covered_classes.contains(&"create"));
    assert!(report.covered_classes.contains(&"delete"));
    assert!(report.covered_classes.contains(&"rename"));
    assert!(report.covered_classes.contains(&"link"));
    assert!(report.covered_classes.contains(&"permissions"));
    assert!(report.covered_classes.contains(&"write_trait"));
    assert!(report.covered_patterns.contains(&"std::fs::write"));
    assert!(report.covered_patterns.contains(&"fs::write"));
    assert!(report.static_proof_is_limited);
    assert!(
        report.findings.is_empty(),
        "unexpected mutation APIs: {:?}",
        report.findings
    );
}

fn audit_core_mutation_surface() -> AuditReport {
    let source_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let source_files = rust_source_files(&source_root);
    let forbidden_patterns = forbidden_mutation_patterns();
    let mut findings = Vec::new();

    for path in &source_files {
        let source = fs::read_to_string(path).expect("core source file must be readable");
        for (line_index, line) in source.lines().enumerate() {
            let code = strip_line_comment(line);
            for pattern in forbidden_patterns {
                if code.contains(pattern.needle) {
                    findings.push(format!(
                        "{}:{} contains forbidden {} mutation pattern `{}`",
                        path.strip_prefix(&source_root)
                            .expect("audited file must be under source root")
                            .display(),
                        line_index + 1,
                        pattern.class,
                        pattern.needle
                    ));
                }
            }
        }
    }

    AuditReport {
        reviewed_files: source_files.len(),
        covered_classes: sorted_covered_classes(forbidden_patterns),
        covered_patterns: sorted_covered_patterns(forbidden_patterns),
        findings,
        static_proof_is_limited: true,
    }
}

#[derive(Debug)]
struct ForbiddenPattern {
    class: &'static str,
    needle: &'static str,
}

fn forbidden_mutation_patterns() -> &'static [ForbiddenPattern] {
    &[
        ForbiddenPattern {
            class: "create",
            needle: "create_dir",
        },
        ForbiddenPattern {
            class: "create",
            needle: "create_new",
        },
        ForbiddenPattern {
            class: "create",
            needle: "File::create",
        },
        ForbiddenPattern {
            class: "create",
            needle: "fs::copy",
        },
        ForbiddenPattern {
            class: "create",
            needle: "std::fs::copy",
        },
        ForbiddenPattern {
            class: "delete",
            needle: "remove_dir",
        },
        ForbiddenPattern {
            class: "delete",
            needle: "remove_file",
        },
        ForbiddenPattern {
            class: "link",
            needle: "hard_link",
        },
        ForbiddenPattern {
            class: "link",
            needle: "symlink_dir",
        },
        ForbiddenPattern {
            class: "link",
            needle: "symlink_file",
        },
        ForbiddenPattern {
            class: "permissions",
            needle: "set_permissions",
        },
        ForbiddenPattern {
            class: "rename",
            needle: "std::fs::rename",
        },
        ForbiddenPattern {
            class: "rename",
            needle: "fs::rename",
        },
        ForbiddenPattern {
            class: "truncate",
            needle: ".set_len(",
        },
        ForbiddenPattern {
            class: "truncate",
            needle: ".truncate(",
        },
        ForbiddenPattern {
            class: "write",
            needle: ".append(",
        },
        ForbiddenPattern {
            class: "write",
            needle: ".write(",
        },
        ForbiddenPattern {
            class: "write",
            needle: ".write_all(",
        },
        ForbiddenPattern {
            class: "write",
            needle: "fs::write",
        },
        ForbiddenPattern {
            class: "write",
            needle: "OpenOptions",
        },
        ForbiddenPattern {
            class: "write",
            needle: "std::fs::write",
        },
        ForbiddenPattern {
            class: "write_trait",
            needle: "BufWriter",
        },
        ForbiddenPattern {
            class: "write_trait",
            needle: "std::io::Write",
        },
    ]
}

fn rust_source_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_rust_source_files(root, &mut files);
    files.sort();
    files
}

fn collect_rust_source_files(path: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(path).expect("source directory must be readable") {
        let entry = entry.expect("source entry must be readable");
        let path = entry.path();
        if path.is_dir() {
            collect_rust_source_files(&path, files);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            files.push(path);
        }
    }
}

fn strip_line_comment(line: &str) -> &str {
    line.split_once("//").map_or(line, |(code, _)| code)
}

fn sorted_covered_classes(patterns: &[ForbiddenPattern]) -> Vec<&'static str> {
    let mut classes = patterns
        .iter()
        .map(|pattern| pattern.class)
        .collect::<Vec<_>>();
    classes.sort_unstable();
    classes.dedup();
    classes
}

fn sorted_covered_patterns(patterns: &[ForbiddenPattern]) -> Vec<&'static str> {
    let mut needles = patterns
        .iter()
        .map(|pattern| pattern.needle)
        .collect::<Vec<_>>();
    needles.sort_unstable();
    needles.dedup();
    needles
}
