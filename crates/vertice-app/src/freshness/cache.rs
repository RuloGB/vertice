//! The persisted freshness document (design §8, §11): a TTL'd cache of
//! reference-lookup responses, plus the enable/disable setting and the
//! first-run-disclosure-seen flag, all in **one** JSON file. This is the
//! only module in the whole workspace that writes a file (CA-16), and its
//! path is derived exclusively from `tauri::Manager::path().app_data_dir()`
//! — never a literal path, never an env read (asserted by
//! `tests/read_only_audit.rs`).

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// TTL before a cache entry is refetched (design §8).
pub const TTL_SECONDS: u64 = 6 * 60 * 60;

/// Ceiling past which a stale entry is no longer served on fetch failure
/// (design §8).
pub const STALE_CEILING_SECONDS: u64 = 7 * 24 * 60 * 60;

const FILE_NAME: &str = "freshness-cache.json";

/// One upstream identity's last-known-good answer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CacheEntry {
    pub version: String,
    pub fetched_at_unix_s: u64,
}

/// The whole persisted document. `enabled` defaults to `true` (spec: "The
/// Check Is Enabled By Default"); a freshly created or corrupt-and-reset
/// document is therefore never silently disabled.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FreshnessStore {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub disclosure_seen: bool,
    #[serde(default)]
    pub cache: HashMap<String, CacheEntry>,
}

fn default_enabled() -> bool {
    true
}

impl Default for FreshnessStore {
    fn default() -> Self {
        Self {
            enabled: true,
            disclosure_seen: false,
            cache: HashMap::new(),
        }
    }
}

/// The document's path, a child of `app_data_dir` — never constructed from
/// a literal absolute path or an environment read. `app_data_dir` itself is
/// resolved by the caller via `tauri::Manager::path().app_data_dir()`.
pub fn store_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(FILE_NAME)
}

/// Load the store. A missing, corrupt, or unreadable file is treated as an
/// empty (default) store — never a crash, never an error surfaced to the
/// caller (design §8, `component-freshness` spec).
pub fn load(path: &Path) -> FreshnessStore {
    fs::read_to_string(path)
        .ok()
        .and_then(|contents| serde_json::from_str(&contents).ok())
        .unwrap_or_default()
}

/// The only write this whole change introduces: one whole-file `fs::write`
/// of the serialized document. No temp-file-plus-rename (design §8: a torn
/// write is indistinguishable from a corrupt cache, and `load` already
/// treats that as empty).
pub fn save(path: &Path, store: &FreshnessStore) -> std::io::Result<()> {
    let serialized =
        serde_json::to_string(store).expect("FreshnessStore serialization cannot fail");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serialized)
}

/// Seconds since the Unix epoch, clamped to `0` on a clock before 1970
/// (never panics).
pub fn now_unix_s() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

/// Whether `entry` is still within its TTL as of `now`.
pub fn is_fresh(entry: &CacheEntry, now: u64) -> bool {
    now.saturating_sub(entry.fetched_at_unix_s) < TTL_SECONDS
}

/// Whether `entry` may still be served as a stale fallback (fetch failed,
/// but the entry is not yet past the 7-day ceiling).
pub fn is_within_stale_ceiling(entry: &CacheEntry, now: u64) -> bool {
    now.saturating_sub(entry.fetched_at_unix_s) <= STALE_CEILING_SECONDS
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static UNIQUE: AtomicU64 = AtomicU64::new(0);

    /// A fresh, empty directory under the OS temp dir, standing in for a
    /// stubbed `app_data_dir()` (design §14) — no real Tauri app is
    /// constructed in these unit tests.
    fn temp_app_data_dir(label: &str) -> PathBuf {
        let unique = UNIQUE.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "vertice-freshness-cache-test-{label}-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).expect("temp test dir must be creatable");
        dir
    }

    #[test]
    fn store_path_is_a_child_of_the_stubbed_app_data_dir() {
        let app_data_dir = temp_app_data_dir("path");
        let path = store_path(&app_data_dir);

        assert_eq!(path.parent(), Some(app_data_dir.as_path()));
        assert_eq!(path.file_name().unwrap(), FILE_NAME);
    }

    #[test]
    fn missing_file_loads_as_the_default_enabled_empty_store() {
        let app_data_dir = temp_app_data_dir("missing");
        let path = store_path(&app_data_dir);

        let store = load(&path);

        assert_eq!(store, FreshnessStore::default());
        assert!(store.enabled);
        assert!(store.cache.is_empty());
    }

    #[test]
    fn corrupt_file_is_treated_as_the_default_empty_store() {
        let app_data_dir = temp_app_data_dir("corrupt");
        let path = store_path(&app_data_dir);
        fs::write(&path, b"not valid json at all, no braces here")
            .expect("test write must succeed");

        let store = load(&path);

        assert_eq!(store, FreshnessStore::default());
    }

    #[test]
    fn save_then_load_round_trips_the_whole_document() {
        let app_data_dir = temp_app_data_dir("roundtrip");
        let path = store_path(&app_data_dir);
        let mut store = FreshnessStore {
            enabled: false,
            disclosure_seen: true,
            ..FreshnessStore::default()
        };
        store.cache.insert(
            "npm:opencode-ai".to_string(),
            CacheEntry {
                version: "1.18.21".to_string(),
                fetched_at_unix_s: 1_000,
            },
        );

        save(&path, &store).expect("save must succeed against a writable temp dir");
        let reloaded = load(&path);

        assert_eq!(reloaded, store);
    }

    /// Regression: `save()` must create its own parent directory. Unlike
    /// `temp_app_data_dir`, this helper builds the path but deliberately
    /// does NOT create it — standing in for a machine where
    /// `app_data_dir()` has never existed (design §9, §14 A1).
    fn temp_app_data_dir_not_created(label: &str) -> PathBuf {
        let unique = UNIQUE.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "vertice-freshness-cache-test-not-created-{label}-{}-{unique}",
            std::process::id()
        ))
    }

    #[test]
    fn save_creates_the_app_data_directory_when_it_does_not_yet_exist() {
        let app_data_dir = temp_app_data_dir_not_created("save-creates-dir");
        assert!(!app_data_dir.exists());
        let path = store_path(&app_data_dir);
        let store = FreshnessStore::default();

        save(&path, &store).expect("save must create the parent directory and succeed");

        let reloaded = load(&path);
        assert_eq!(reloaded, store);
    }

    #[test]
    fn save_is_a_single_whole_file_write() {
        let app_data_dir = temp_app_data_dir("wholefile");
        let path = store_path(&app_data_dir);
        let store = FreshnessStore::default();

        save(&path, &store).expect("save must succeed");

        assert!(path.is_file());
        assert!(
            fs::read_dir(&app_data_dir)
                .expect("dir must be readable")
                .count()
                == 1
        );
    }

    #[test]
    fn ttl_is_respected() {
        let now = 100_000;
        let fresh = CacheEntry {
            version: "1.0.0".to_string(),
            fetched_at_unix_s: now - (TTL_SECONDS - 1),
        };
        let expired = CacheEntry {
            version: "1.0.0".to_string(),
            fetched_at_unix_s: now - (TTL_SECONDS + 1),
        };

        assert!(is_fresh(&fresh, now));
        assert!(!is_fresh(&expired, now));
    }

    #[test]
    fn expired_entry_serves_stale_within_seven_days_not_beyond() {
        let now = 1_000_000;
        let just_expired = CacheEntry {
            version: "1.0.0".to_string(),
            fetched_at_unix_s: now - (TTL_SECONDS + 1),
        };
        let within_ceiling = CacheEntry {
            version: "1.0.0".to_string(),
            fetched_at_unix_s: now - (STALE_CEILING_SECONDS - 1),
        };
        let past_ceiling = CacheEntry {
            version: "1.0.0".to_string(),
            fetched_at_unix_s: now - (STALE_CEILING_SECONDS + 1),
        };

        assert!(!is_fresh(&just_expired, now));
        assert!(is_within_stale_ceiling(&just_expired, now));
        assert!(is_within_stale_ceiling(&within_ceiling, now));
        assert!(!is_within_stale_ceiling(&past_ceiling, now));
    }
}
