//! Skill discovery: walks the three fixed user roots and assembles
//! `Component`/`ScanIssue` values from any discovered `SKILL.md`.
//!
//! RED: signature only, body not yet implemented (task 2.4).

use std::path::Path;

use crate::model::{Component, ScanIssue, SearchRoot};

/// Owned result of one scan. See `crate::skills::scan`.
#[derive(Debug, Clone, PartialEq)]
pub struct SkillScan {
    pub roots: Vec<SearchRoot>,
    pub components: Vec<Component>,
    pub issues: Vec<ScanIssue>,
}

/// Scan the three fixed user skill roots under `home`. See task 2.4.
pub fn scan(_home: &Path) -> SkillScan {
    todo!("implemented in task 2.4")
}
