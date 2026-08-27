use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use vertice_core::model::{Prompt, PromptDraft, PromptError, PromptUpdate};

const FILE_NAME: &str = "prompts.json";
const SCHEMA_VERSION: u8 = 1;
static UNIQUE_SUFFIX: AtomicU64 = AtomicU64::new(0);

#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadOutcome {
    Loaded(Vec<Prompt>),
    Unavailable(String),
}

pub trait PromptRepository {
    fn list(&self) -> Result<Vec<Prompt>, PromptError>;
    fn create(&mut self, draft: PromptDraft) -> Result<Prompt, PromptError>;
    fn update(&mut self, update: PromptUpdate) -> Result<Prompt, PromptError>;
    fn delete(&mut self, id: &str) -> Result<(), PromptError>;
}

#[derive(Debug, Clone)]
pub struct JsonPromptRepository {
    path: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PromptDocument {
    schema_version: u8,
    prompts: Vec<Prompt>,
}

#[derive(Debug)]
struct NormalizedDraft {
    title: String,
    body: String,
    tags: Vec<String>,
    best_for_context: Option<String>,
}

impl JsonPromptRepository {
    pub fn new(app_data_dir: PathBuf) -> Self {
        Self {
            path: store_path(&app_data_dir),
        }
    }

    #[cfg(test)]
    pub fn load(&self) -> LoadOutcome {
        match self.read_prompts() {
            Ok(prompts) => LoadOutcome::Loaded(prompts),
            Err(err) => LoadOutcome::Unavailable(err),
        }
    }

    fn read_prompts(&self) -> Result<Vec<Prompt>, String> {
        match fs::read_to_string(&self.path) {
            Ok(contents) => {
                if contents.trim().is_empty() {
                    return Err("prompt store is empty".to_string());
                }
                let document: PromptDocument = serde_json::from_str(&contents)
                    .map_err(|_| "prompt store is not valid JSON".to_string())?;
                if document.schema_version != SCHEMA_VERSION {
                    return Err(format!(
                        "unsupported prompt store schema version {}",
                        document.schema_version
                    ));
                }
                Ok(document.prompts)
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(err) => Err(format!("prompt store is unreadable: {err}")),
        }
    }

    fn save_prompts(&self, prompts: Vec<Prompt>, operation: &str) -> Result<(), PromptError> {
        let document = PromptDocument {
            schema_version: SCHEMA_VERSION,
            prompts,
        };
        let serialized = serde_json::to_string_pretty(&document).map_err(store_error)?;
        let tmp_path = stage(&self.path, operation, &serialized).map_err(store_error)?;
        fs::rename(&tmp_path, &self.path).map_err(store_error)
    }
}

impl PromptRepository for JsonPromptRepository {
    fn list(&self) -> Result<Vec<Prompt>, PromptError> {
        self.read_prompts().map_err(store_unavailable)
    }

    fn create(&mut self, draft: PromptDraft) -> Result<Prompt, PromptError> {
        let normalized = normalize_draft(draft)?;
        let mut prompts = self.read_prompts().map_err(store_unavailable)?;
        let prompt = Prompt {
            id: prompt_id(),
            title: normalized.title,
            body: normalized.body,
            tags: normalized.tags,
            best_for_context: normalized.best_for_context,
            updated_at: timestamp(),
        };
        prompts.push(prompt.clone());
        self.save_prompts(prompts, "create")?;
        Ok(prompt)
    }

    fn update(&mut self, update: PromptUpdate) -> Result<Prompt, PromptError> {
        let normalized = normalize_draft(PromptDraft {
            title: update.title,
            body: update.body,
            tags: update.tags,
            best_for_context: update.best_for_context,
        })?;
        let mut prompts = self.read_prompts().map_err(store_unavailable)?;
        let Some(existing) = prompts.iter_mut().find(|prompt| prompt.id == update.id) else {
            return Err(PromptError::NotFound { id: update.id });
        };
        existing.title = normalized.title;
        existing.body = normalized.body;
        existing.tags = normalized.tags;
        existing.best_for_context = normalized.best_for_context;
        existing.updated_at = timestamp();
        let updated = existing.clone();
        self.save_prompts(prompts, "update")?;
        Ok(updated)
    }

    fn delete(&mut self, id: &str) -> Result<(), PromptError> {
        let mut prompts = self.read_prompts().map_err(store_unavailable)?;
        let initial_len = prompts.len();
        prompts.retain(|prompt| prompt.id != id);
        if prompts.len() == initial_len {
            return Err(PromptError::NotFound { id: id.to_string() });
        }
        self.save_prompts(prompts, "delete")
    }
}

pub fn store_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(FILE_NAME)
}

fn normalize_draft(draft: PromptDraft) -> Result<NormalizedDraft, PromptError> {
    let title = required_field("title", draft.title)?;
    let body = required_field("body", draft.body)?;
    let tags = draft
        .tags
        .into_iter()
        .map(|tag| tag.trim().to_string())
        .filter(|tag| !tag.is_empty())
        .collect();
    let best_for_context = draft
        .best_for_context
        .map(|context| context.trim().to_string())
        .filter(|context| !context.is_empty());
    Ok(NormalizedDraft {
        title,
        body,
        tags,
        best_for_context,
    })
}

fn required_field(field: &str, value: String) -> Result<String, PromptError> {
    let trimmed = value.trim().to_string();
    if trimmed.is_empty() {
        return Err(PromptError::InvalidInput {
            field: field.to_string(),
        });
    }
    Ok(trimmed)
}

fn timestamp() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn stage(path: &Path, operation: &str, contents: &str) -> io::Result<PathBuf> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp_path = path.with_file_name(format!(
        "{FILE_NAME}.tmp-{operation}-{}-{}",
        std::process::id(),
        unique_suffix()
    ));
    fs::write(&tmp_path, contents)?;
    Ok(tmp_path)
}

fn store_unavailable(reason: String) -> PromptError {
    PromptError::StoreUnavailable { reason }
}

fn store_error(error: impl std::fmt::Display) -> PromptError {
    PromptError::StoreUnavailable {
        reason: error.to_string(),
    }
}

fn prompt_id() -> String {
    format!("prompt-{}", unique_suffix())
}

fn unique_suffix() -> String {
    let sequence = UNIQUE_SUFFIX.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!("{nanos}-{sequence}")
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Barrier, Mutex};
    use std::thread;

    use vertice_core::model::{PromptDraft, PromptError, PromptUpdate};

    use super::{store_path, JsonPromptRepository, LoadOutcome, PromptRepository};

    fn temp_app_data_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "vertice-prompt-store-test-{label}-{}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).expect("temp test dir must be creatable");
        dir
    }

    fn draft(title: &str, body: &str) -> PromptDraft {
        PromptDraft {
            title: title.to_string(),
            body: body.to_string(),
            tags: vec!["  TDD  ".to_string(), "".to_string(), "rust".to_string()],
            best_for_context: Some("  Code Review  ".to_string()),
        }
    }

    #[test]
    fn missing_file_loads_empty_without_creating_document() {
        let app_data_dir = temp_app_data_dir("missing");
        let path = store_path(&app_data_dir);

        let outcome = JsonPromptRepository::new(app_data_dir).load();

        assert!(matches!(outcome, LoadOutcome::Loaded(prompts) if prompts.is_empty()));
        assert!(!path.exists());
    }

    #[test]
    fn create_rejects_empty_title_or_body_and_preserves_existing_bytes() {
        let app_data_dir = temp_app_data_dir("reject-create");
        let mut repo = JsonPromptRepository::new(app_data_dir.clone());
        let prompt = repo.create(draft("Good", "Body")).expect("seed prompt");
        let before = fs::read(store_path(&app_data_dir)).expect("seed bytes");

        let empty_title = repo
            .create(draft("   ", "Body"))
            .expect_err("title rejected");
        let empty_body = repo
            .create(draft("Title", "   "))
            .expect_err("body rejected");

        assert_eq!(
            empty_title,
            PromptError::InvalidInput {
                field: "title".to_string()
            }
        );
        assert_eq!(
            empty_body,
            PromptError::InvalidInput {
                field: "body".to_string()
            }
        );
        assert_eq!(fs::read(store_path(&app_data_dir)).unwrap(), before);
        assert_eq!(repo.list().unwrap(), vec![prompt]);
    }

    #[test]
    fn create_and_update_trim_optional_fields_and_preserve_identity() {
        let app_data_dir = temp_app_data_dir("normalize");
        let mut repo = JsonPromptRepository::new(app_data_dir);

        let created = repo.create(draft("  Title  ", "  Body  ")).expect("create");
        assert_eq!(created.title, "Title");
        assert_eq!(created.body, "Body");
        assert_eq!(created.tags, vec!["TDD".to_string(), "rust".to_string()]);
        assert_eq!(created.best_for_context, Some("Code Review".to_string()));

        let updated = repo
            .update(PromptUpdate {
                id: created.id.clone(),
                title: "  Better  ".to_string(),
                body: "  New body  ".to_string(),
                tags: vec![" ux ".to_string(), " ".to_string()],
                best_for_context: Some("   ".to_string()),
            })
            .expect("update");

        assert_eq!(updated.id, created.id);
        assert_eq!(updated.title, "Better");
        assert_eq!(updated.body, "New body");
        assert_eq!(updated.tags, vec!["ux".to_string()]);
        assert_eq!(updated.best_for_context, None);
        assert_ne!(updated.updated_at, "");
    }

    #[test]
    fn corrupt_empty_unreadable_and_future_schema_fail_closed_and_preserve_bytes() {
        let app_data_dir = temp_app_data_dir("fail-closed");
        let path = store_path(&app_data_dir);
        for contents in ["", "not json", r#"{"schemaVersion":99,"prompts":[]}"#] {
            fs::write(&path, contents).expect("write malformed fixture");
            let before = fs::read(&path).expect("fixture bytes");
            let mut repo = JsonPromptRepository::new(app_data_dir.clone());

            assert!(matches!(repo.load(), LoadOutcome::Unavailable(_)));
            assert!(matches!(
                repo.create(draft("Safe", "Body")),
                Err(PromptError::StoreUnavailable { .. })
            ));
            assert_eq!(fs::read(&path).unwrap(), before);
        }
    }

    #[test]
    fn update_delete_missing_identity_preserve_bytes() {
        let app_data_dir = temp_app_data_dir("not-found");
        let mut repo = JsonPromptRepository::new(app_data_dir.clone());
        repo.create(draft("One", "Body")).expect("seed");
        let before = fs::read(store_path(&app_data_dir)).unwrap();

        let update_err = repo
            .update(PromptUpdate {
                id: "missing".to_string(),
                title: "Two".to_string(),
                body: "Body".to_string(),
                tags: vec![],
                best_for_context: None,
            })
            .expect_err("missing update");
        let delete_err = repo.delete("missing").expect_err("missing delete");

        assert_eq!(
            update_err,
            PromptError::NotFound {
                id: "missing".to_string()
            }
        );
        assert_eq!(
            delete_err,
            PromptError::NotFound {
                id: "missing".to_string()
            }
        );
        assert_eq!(fs::read(store_path(&app_data_dir)).unwrap(), before);
    }

    #[test]
    fn concurrent_whole_mutations_serialize_without_lost_updates_or_temp_collisions() {
        let app_data_dir = temp_app_data_dir("concurrent");
        let repo = Arc::new(Mutex::new(JsonPromptRepository::new(app_data_dir.clone())));
        let barrier = Arc::new(Barrier::new(8));
        let mut handles = Vec::new();

        for index in 0..8 {
            let repo = Arc::clone(&repo);
            let barrier = Arc::clone(&barrier);
            handles.push(thread::spawn(move || {
                barrier.wait();
                repo.lock()
                    .unwrap()
                    .create(draft(&format!("Prompt {index}"), "Body"))
                    .unwrap();
            }));
        }
        for handle in handles {
            handle.join().expect("thread joins");
        }

        let prompts = repo.lock().unwrap().list().expect("list prompts");
        assert_eq!(prompts.len(), 8);
        assert_eq!(leftover_temp_files(&app_data_dir), Vec::<String>::new());
    }

    #[test]
    fn store_path_is_a_child_of_app_data_dir() {
        let app_data_dir = temp_app_data_dir("path");
        let path = store_path(&app_data_dir);

        assert_eq!(path.parent(), Some(app_data_dir.as_path()));
        assert_eq!(path.file_name().unwrap(), "prompts.json");
    }

    fn leftover_temp_files(app_data_dir: &Path) -> Vec<String> {
        let mut files: Vec<String> = fs::read_dir(app_data_dir)
            .unwrap()
            .flatten()
            .filter_map(|entry| entry.file_name().to_str().map(str::to_string))
            .filter(|name| name.contains(".tmp-"))
            .collect();
        files.sort();
        files
    }
}
