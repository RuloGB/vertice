//! The durable `settings.json` document (`add-locale-persistence` design
//! §2): `locale`, `enabled`, `disclosure_seen`, written through
//! temp-file-plus-rename. Distinct from `freshness::cache`'s disposable
//! whole-file write — the two documents deliberately have different write
//! semantics. This module's path is derived exclusively from
//! `tauri::Manager::path().app_data_dir()` — never a literal path, never an
//! env read (asserted by `tests/read_only_audit.rs`).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use vertice_core::model::UserSettings;

const FILE_NAME: &str = "settings.json";
const TMP_FILE_NAME: &str = "settings.json.tmp";

/// The three-way classification `load` produces, evaluated in this order —
/// `Missing` before `Unreadable` before `Loaded` (design §2, user-settings
/// spec "The Load Outcome Is A Three-Way Classification").
pub enum LoadOutcome {
    Missing,
    Loaded(UserSettings),
    Unreadable,
}

/// The document's path, a child of `app_data_dir` — never constructed from
/// a literal absolute path or an environment read. `app_data_dir` itself is
/// resolved by the caller via `tauri::Manager::path().app_data_dir()`.
pub fn store_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(FILE_NAME)
}

/// Classify `path`'s contents, never panicking and never surfacing an
/// error: `Err(NotFound)` -> `Missing`; any other read error, an empty or
/// whitespace-only file, or a parse failure -> `Unreadable`; a
/// successfully-parsed document -> `Loaded`.
pub fn load(path: &Path) -> LoadOutcome {
    match fs::read_to_string(path) {
        Ok(contents) => {
            if contents.trim().is_empty() {
                return LoadOutcome::Unreadable;
            }
            match serde_json::from_str::<UserSettings>(&contents) {
                Ok(settings) => LoadOutcome::Loaded(settings),
                Err(_) => LoadOutcome::Unreadable,
            }
        }
        Err(err) if err.kind() == io::ErrorKind::NotFound => LoadOutcome::Missing,
        Err(_) => LoadOutcome::Unreadable,
    }
}

/// Pure, total mapping from a load outcome to the settled `UserSettings`
/// (design §2). `enabled`'s fallback is asymmetric by settled decision:
/// `true` on a genuine first run (`Missing`), `false` when the document
/// exists but cannot be trusted (`Unreadable`) — so a read failure never
/// silently resumes outbound requests the user had turned off. `locale` and
/// `disclosure_seen` fall back identically for both failure outcomes.
pub fn resolve(outcome: LoadOutcome) -> UserSettings {
    match outcome {
        LoadOutcome::Missing => UserSettings {
            locale: None,
            enabled: true,
            disclosure_seen: false,
        },
        LoadOutcome::Loaded(settings) => settings,
        LoadOutcome::Unreadable => UserSettings {
            locale: None,
            enabled: false,
            disclosure_seen: false,
        },
    }
}

/// Write `contents` to a temp file beside `path`, creating the parent
/// directory first if it does not yet exist. Does not commit — the caller
/// must call `commit` to make the write visible.
fn stage(path: &Path, contents: &str) -> io::Result<PathBuf> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let tmp_path = parent.join(TMP_FILE_NAME);
    fs::write(&tmp_path, contents)?;
    Ok(tmp_path)
}

/// Atomically make a staged temp file the document at `path`.
fn commit(tmp_path: &Path, path: &Path) -> io::Result<()> {
    fs::rename(tmp_path, path)
}

/// Stage then commit: `settings.json` is only ever replaced by a fully
/// written temp file renamed into place, so a write interrupted before the
/// rename leaves the previously-persisted document intact (design §2,
/// user-settings spec "An Explicit User Choice Survives A Full Application
/// Restart").
pub fn save(path: &Path, settings: &UserSettings) -> io::Result<()> {
    let serialized =
        serde_json::to_string(settings).expect("UserSettings serialization cannot fail");
    let tmp_path = stage(path, &serialized)?;
    commit(&tmp_path, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use vertice_core::model::UserSettings;

    fn temp_app_data_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "vertice-settings-store-test-{label}-{}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).expect("temp test dir must be creatable");
        dir
    }

    /// Mirrors `freshness::cache`'s "not yet created" helper — path built
    /// but deliberately not pre-created.
    fn temp_app_data_dir_not_created(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "vertice-settings-store-test-not-created-{label}-{}",
            std::process::id()
        ))
    }

    #[test]
    fn store_path_is_a_child_of_the_stubbed_app_data_dir() {
        let app_data_dir = temp_app_data_dir("path");
        let path = store_path(&app_data_dir);

        assert_eq!(path.parent(), Some(app_data_dir.as_path()));
        assert_eq!(path.file_name().unwrap(), "settings.json");
    }

    #[test]
    fn never_created_file_loads_as_missing_and_resolves_enabled_true() {
        let app_data_dir = temp_app_data_dir("never-created");
        let path = store_path(&app_data_dir);

        let outcome = load(&path);
        assert!(matches!(outcome, LoadOutcome::Missing));

        let settings = resolve(outcome);
        assert_eq!(
            settings,
            UserSettings {
                locale: None,
                enabled: true,
                disclosure_seen: false,
            }
        );
    }

    #[test]
    fn corrupt_file_resolves_enabled_false() {
        let app_data_dir = temp_app_data_dir("corrupt");
        let path = store_path(&app_data_dir);
        fs::write(&path, b"{ not json").expect("test write must succeed");

        let outcome = load(&path);
        assert!(matches!(outcome, LoadOutcome::Unreadable));

        let settings = resolve(outcome);
        assert!(!settings.enabled);
        assert!(!settings.disclosure_seen);
        assert_eq!(settings.locale, None);
    }

    #[test]
    fn empty_file_resolves_enabled_false() {
        let app_data_dir = temp_app_data_dir("empty");
        let path = store_path(&app_data_dir);
        fs::write(&path, b"").expect("test write must succeed");

        let outcome = load(&path);
        assert!(matches!(outcome, LoadOutcome::Unreadable));
        assert!(!resolve(outcome).enabled);
    }

    #[test]
    fn whitespace_only_file_resolves_enabled_false() {
        let app_data_dir = temp_app_data_dir("whitespace");
        let path = store_path(&app_data_dir);
        fs::write(&path, b"   \n").expect("test write must succeed");

        let outcome = load(&path);
        assert!(matches!(outcome, LoadOutcome::Unreadable));
        assert!(!resolve(outcome).enabled);
    }

    #[test]
    fn resolve_is_conservative_for_every_unreadable_producer() {
        let settings = resolve(LoadOutcome::Unreadable);

        assert!(!settings.enabled);
        assert!(!settings.disclosure_seen);
        assert_eq!(settings.locale, None);
    }

    #[test]
    fn valid_document_round_trips_all_three_fields() {
        let app_data_dir = temp_app_data_dir("roundtrip");
        let path = store_path(&app_data_dir);
        let settings = UserSettings {
            locale: Some("es".to_string()),
            enabled: false,
            disclosure_seen: true,
        };

        save(&path, &settings).expect("save must succeed against a writable temp dir");
        let outcome = load(&path);
        assert!(matches!(outcome, LoadOutcome::Loaded(_)));
        assert_eq!(resolve(outcome), settings);
    }

    #[test]
    fn save_creates_the_app_data_directory_when_it_does_not_yet_exist() {
        let app_data_dir = temp_app_data_dir_not_created("save-creates-dir");
        assert!(!app_data_dir.exists());
        let path = store_path(&app_data_dir);
        let settings = UserSettings {
            locale: None,
            enabled: true,
            disclosure_seen: false,
        };

        save(&path, &settings).expect("save must create the parent directory and succeed");

        assert_eq!(resolve(load(&path)), settings);
    }

    #[test]
    fn save_leaves_no_temp_file_behind() {
        let app_data_dir = temp_app_data_dir("no-temp-file");
        let path = store_path(&app_data_dir);
        let settings = UserSettings {
            locale: None,
            enabled: true,
            disclosure_seen: false,
        };

        save(&path, &settings).expect("save must succeed");

        let entries: Vec<_> = fs::read_dir(&app_data_dir)
            .expect("dir must be readable")
            .collect();
        assert_eq!(entries.len(), 1);
        let entry = entries.into_iter().next().unwrap().expect("entry readable");
        assert_eq!(entry.file_name(), "settings.json");
    }

    #[test]
    fn an_interrupted_write_leaves_the_previous_document_intact() {
        let app_data_dir = temp_app_data_dir("interrupted-write");
        let path = store_path(&app_data_dir);
        let original = UserSettings {
            locale: Some("en".to_string()),
            enabled: true,
            disclosure_seen: false,
        };
        save(&path, &original).expect("initial save must succeed");

        let interrupted = UserSettings {
            locale: Some("es".to_string()),
            enabled: false,
            disclosure_seen: true,
        };
        let serialized =
            serde_json::to_string(&interrupted).expect("UserSettings serialization cannot fail");
        // Stage only — never commit — simulating an interruption before the
        // atomic rename.
        stage(&path, &serialized).expect("stage must succeed");

        assert_eq!(resolve(load(&path)), original);
    }
}
