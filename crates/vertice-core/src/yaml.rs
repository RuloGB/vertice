//! YAML deserialization seam.
//!
//! This is the ONLY module in `vertice-core` allowed to import the YAML
//! parsing crate (`serde_norway`) directly. Every other module MUST go
//! through [`from_str`]. Swapping the underlying YAML crate later means
//! changing this file and `Cargo.toml` only — see `tests/yaml_behavior.rs`
//! for the pinned behaviours this seam guarantees, and
//! `openspec/changes/bootstrap-workspace-ci/design.md` for the crate
//! evaluation and decision.

use serde::de::DeserializeOwned;

/// Error returned when YAML input cannot be parsed or deserialized.
#[derive(Debug, thiserror::Error)]
pub enum YamlError {
    #[error("failed to parse YAML: {0}")]
    Parse(#[from] serde_norway::Error),
}

/// Deserialize a value of type `T` from a YAML string.
pub fn from_str<T: DeserializeOwned>(input: &str) -> Result<T, YamlError> {
    serde_norway::from_str(input).map_err(YamlError::from)
}
