//! Frontmatter and `SKILL.md` reader.
//!
//! Turns a single path into a typed value or a [`ScanIssue`]. This is the
//! first module in `vertice-core` that touches the filesystem: it is a
//! sibling of `model/`, not a member of it, so `model/`'s zero-I/O invariant
//! is unaffected. This module MUST NOT import the YAML parsing crate
//! directly — every deserialization goes through [`crate::yaml::from_str`],
//! the crate's single shared YAML seam.
//!
//! Five ordered steps turn a `&Path` into `Result<T, ScanIssue>`: read
//! bytes, validate UTF-8, split the `---`-fenced block, deserialize it, and
//! map every failure arm to a `ScanIssue`. No arm panics.

use std::path::Path;

use serde::de::DeserializeOwned;
use serde::Deserialize;

use crate::model::{IssueSeverity, ScanIssue};

/// Frontmatter contract for a `SKILL.md`-shaped file. `name` is required;
/// `description` mirrors `Component.description`'s optionality. Deliberately
/// `Deserialize`-only: this is a reader artifact consumed and discarded by a
/// caller assembling a `Component`, never a value that crosses IPC.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct SkillFrontmatter {
    pub name: String,
    pub description: Option<String>,
}

/// Why [`split`] could not isolate a frontmatter block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FenceError {
    /// The file is empty or whitespace-only.
    Empty,
    /// The first line is not an exact `---` fence.
    NoOpeningFence,
    /// An opening `---` fence was found but no closing fence appeared
    /// before end of file.
    Unterminated,
}

/// Isolate the `---`-fenced block from `source`, line-based and regex-free.
///
/// Touches no byte index and no string slice by offset, so an out-of-bounds
/// panic is structurally impossible rather than merely avoided.
fn split(source: &str) -> Result<String, FenceError> {
    if source.trim().is_empty() {
        return Err(FenceError::Empty);
    }

    let mut lines = source.lines();
    match lines.next() {
        Some(first) if first.trim_end() == "---" => {}
        _ => return Err(FenceError::NoOpeningFence),
    }

    let mut block = String::new();
    for line in lines {
        if line.trim_end() == "---" {
            return Ok(block);
        }
        block.push_str(line);
        block.push('\n');
    }

    Err(FenceError::Unterminated)
}

/// Read and parse a single `SKILL.md`-shaped file at `path` into `T`.
///
/// `T` is any `DeserializeOwned` type, so a future caller (T5) supplies its
/// own frontmatter shape without modifying this function. Every failure
/// class — I/O, non-UTF-8 content, an absent or unterminated fence, or a
/// YAML/type/missing-field error — is converted into a `ScanIssue` with
/// `path: Some(path.to_path_buf())`. Never panics.
pub fn read<T: DeserializeOwned>(path: &Path) -> Result<T, ScanIssue> {
    let bytes = std::fs::read(path).map_err(|err| ScanIssue {
        severity: IssueSeverity::Warning,
        path: Some(path.to_path_buf()),
        reason: format!("could not read file: {err}"),
    })?;

    let content = std::str::from_utf8(&bytes).map_err(|err| ScanIssue {
        severity: IssueSeverity::Warning,
        path: Some(path.to_path_buf()),
        reason: format!(
            "file content is not valid UTF-8 (valid up to byte {})",
            err.valid_up_to()
        ),
    })?;

    let block = split(content).map_err(|err| {
        let (severity, reason) = match err {
            FenceError::Empty => (IssueSeverity::Warning, "file is empty".to_string()),
            FenceError::NoOpeningFence => (
                IssueSeverity::Warning,
                "no frontmatter block: file does not begin with a --- fence".to_string(),
            ),
            FenceError::Unterminated => (
                IssueSeverity::Error,
                "unterminated frontmatter block: opening --- fence with no closing fence \
                 before end of file"
                    .to_string(),
            ),
        };
        ScanIssue {
            severity,
            path: Some(path.to_path_buf()),
            reason,
        }
    })?;

    crate::yaml::from_str(&block).map_err(|err| ScanIssue {
        severity: IssueSeverity::Error,
        path: Some(path.to_path_buf()),
        reason: format!("frontmatter is not valid YAML: {err}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opening_and_closing_fence_yields_the_block_between_them() {
        let source = "---\nname: a\n---\nbody\n";

        let block = split(source).expect("well-formed fence should split");

        assert_eq!(block, "name: a\n");
    }

    #[test]
    fn empty_source_is_a_distinct_error_from_no_opening_fence() {
        assert_eq!(split(""), Err(FenceError::Empty));
        assert_eq!(split("   \n\t\n"), Err(FenceError::Empty));
    }

    #[test]
    fn missing_opening_fence_is_reported() {
        let source = "# Not frontmatter\nbody\n";

        assert_eq!(split(source), Err(FenceError::NoOpeningFence));
    }

    #[test]
    fn indented_fence_is_not_an_opening_fence() {
        // A frontmatter fence must sit at column 0. Only trailing whitespace
        // is tolerated, so an indented `---` is content, not a delimiter.
        assert_eq!(
            split(
                "  ---
name: a
---
"
            ),
            Err(FenceError::NoOpeningFence)
        );
        assert_eq!(
            split(
                "	---
name: a
---
"
            ),
            Err(FenceError::NoOpeningFence)
        );
    }

    #[test]
    fn opening_fence_tolerates_trailing_whitespace() {
        let source = "---  
name: a
---
";

        let block = split(source).expect("trailing whitespace after the fence is tolerated");

        assert_eq!(
            block,
            "name: a
"
        );
    }

    #[test]
    fn fence_not_on_the_first_line_is_treated_as_no_opening_fence() {
        let source = "\n---\nname: a\n---\n";

        assert_eq!(split(source), Err(FenceError::NoOpeningFence));
    }

    #[test]
    fn unterminated_opening_fence_reaches_eof() {
        let source = "---\nname: a\nno closing fence here\n";

        assert_eq!(split(source), Err(FenceError::Unterminated));
    }

    #[test]
    fn crlf_fence_lines_still_match() {
        let source = "---\r\nname: a\r\n---\r\nbody\r\n";

        let block = split(source).expect("CRLF fence should still split");

        assert_eq!(block, "name: a\n");
    }

    #[test]
    fn empty_block_between_fences_yields_an_empty_string() {
        let source = "---\n---\nbody\n";

        let block = split(source).expect("adjacent fences should yield an empty block");

        assert_eq!(block, "");
    }
}
