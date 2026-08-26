//! `McpTransport` — connection detail for an MCP `Location`.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Connection detail for one MCP `Location`. Closed, never
/// `#[non_exhaustive]`, and deliberately INCAPABLE of holding a secret:
/// there is no field for an `env` value, a `headers` value, or an
/// individual argument. Redaction is therefore a property of this type,
/// not a rule an adapter author can forget.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../frontend/src/bindings/")]
pub enum McpTransport {
    Stdio {
        /// The executable only. Never an argument, never an env value.
        command: String,
        /// How many arguments were configured. A count cannot carry a value.
        arg_count: usize,
        /// Key NAMES from the environment map, in the map's own sorted
        /// order. Never a value.
        env_keys: Vec<String>,
    },
    Remote {
        /// The sanitized ORIGIN of the configured endpoint —
        /// `scheme://host[:port]`. Userinfo, query, fragment AND path are
        /// removed before construction (design §3.2); this field is never
        /// built from a raw configured string.
        url: String,
        /// Key NAMES from the headers map. Never a value.
        header_keys: Vec<String>,
    },
}
