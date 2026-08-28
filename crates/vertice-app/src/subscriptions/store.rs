use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{DateTime, SecondsFormat, Utc};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use vertice_core::model::{
    BillingCycle, Subscription, SubscriptionDraft, SubscriptionError, SubscriptionUpdate,
};

const FILE_NAME: &str = "subscriptions.json";
const LOCK_FILE_NAME: &str = "subscriptions.lock";
const SCHEMA_VERSION: u8 = 1;
static UNIQUE_SUFFIX: AtomicU64 = AtomicU64::new(0);

pub trait SubscriptionRepository {
    fn list(&self) -> Result<Vec<Subscription>, SubscriptionError>;
    fn create(&mut self, draft: SubscriptionDraft) -> Result<Subscription, SubscriptionError>;
    fn update(&mut self, update: SubscriptionUpdate) -> Result<Subscription, SubscriptionError>;
    fn delete(&mut self, id: &str) -> Result<(), SubscriptionError>;
}

#[derive(Debug, Clone)]
pub struct JsonSubscriptionRepository {
    path: PathBuf,
    #[cfg(test)]
    forced_durability_warning: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SubscriptionDocument {
    schema_version: u8,
    subscriptions: Vec<Subscription>,
}

#[derive(Debug)]
enum StoreReadError {
    Corrupt(String),
    Unavailable(String),
}

impl StoreReadError {
    fn into_subscription_error(self) -> SubscriptionError {
        match self {
            Self::Corrupt(reason) => SubscriptionError::StoreCorrupt { reason },
            Self::Unavailable(reason) => SubscriptionError::StoreUnavailable { reason },
        }
    }
}

#[derive(Debug)]
struct NormalizedDraft {
    provider: String,
    plan: String,
    amount: f64,
    currency: vertice_core::model::Currency,
    cycle: BillingCycle,
    renewal_day: u8,
    renewal_month: Option<u8>,
}

impl JsonSubscriptionRepository {
    pub fn new(app_data_dir: PathBuf) -> Self {
        Self {
            path: store_path(&app_data_dir),
            #[cfg(test)]
            forced_durability_warning: false,
        }
    }
    #[cfg(test)]
    pub(crate) fn new_with_forced_durability_warning(app_data_dir: PathBuf) -> Self {
        Self {
            path: store_path(&app_data_dir),
            forced_durability_warning: true,
        }
    }
    fn read_subscriptions(&self) -> Result<Vec<Subscription>, StoreReadError> {
        match fs::read_to_string(&self.path) {
            Ok(contents) => {
                if contents.trim().is_empty() {
                    return Err(StoreReadError::Corrupt(
                        "subscription store is empty".into(),
                    ));
                }
                let document: SubscriptionDocument =
                    serde_json::from_str(&contents).map_err(|_| {
                        StoreReadError::Corrupt("subscription store is not valid JSON".into())
                    })?;
                if document.schema_version != SCHEMA_VERSION {
                    return Err(StoreReadError::Corrupt(format!(
                        "unsupported subscription store schema version {}",
                        document.schema_version
                    )));
                }
                for record in &document.subscriptions {
                    validate_record(record).map_err(StoreReadError::Corrupt)?;
                }
                Ok(document.subscriptions)
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(error) => Err(StoreReadError::Unavailable(format!(
                "subscription store is unreadable: {error}"
            ))),
        }
    }
    fn with_exclusive_lock<T>(
        &self,
        operation: impl FnOnce() -> Result<T, SubscriptionError>,
    ) -> Result<T, SubscriptionError> {
        let parent = self.path.parent().ok_or_else(|| {
            store_unavailable("subscription store has no parent directory".into())
        })?;
        fs::create_dir_all(parent).map_err(store_error)?;
        let lock_path = parent.join(LOCK_FILE_NAME);
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(lock_path)
            .map_err(store_error)?;
        lock.lock_exclusive().map_err(store_error)?;
        operation()
    }

    fn save_subscriptions(
        &self,
        subscriptions: Vec<Subscription>,
        operation: &str,
    ) -> Result<(), SubscriptionError> {
        let serialized = serde_json::to_string_pretty(&SubscriptionDocument {
            schema_version: SCHEMA_VERSION,
            subscriptions: subscriptions.clone(),
        })
        .map_err(store_error)?;
        let temporary = stage_and_sync(&self.path, operation, &serialized).map_err(store_error)?;
        if let Err(error) = fs::rename(&temporary, &self.path) {
            let _ = fs::remove_file(&temporary);
            return Err(store_error(error));
        }
        reconcile_parent_sync(
            self.parent_sync_result(),
            || self.read_after_parent_sync(),
            &subscriptions,
        )
    }

    fn parent_sync_result(&self) -> io::Result<()> {
        #[cfg(test)]
        if self.forced_durability_warning {
            return Err(io::Error::other("forced parent directory sync failure"));
        }
        sync_parent_directory(&self.path)
    }

    fn read_after_parent_sync(&self) -> Result<Vec<Subscription>, StoreReadError> {
        #[cfg(test)]
        if self.forced_durability_warning {
            return Err(StoreReadError::Unavailable(
                "forced post-rename reconciliation failure".into(),
            ));
        }
        self.read_subscriptions()
    }
}

impl SubscriptionRepository for JsonSubscriptionRepository {
    fn list(&self) -> Result<Vec<Subscription>, SubscriptionError> {
        self.with_exclusive_lock(|| {
            self.read_subscriptions()
                .map_err(StoreReadError::into_subscription_error)
        })
    }
    fn create(&mut self, draft: SubscriptionDraft) -> Result<Subscription, SubscriptionError> {
        self.with_exclusive_lock(|| {
            let normalized = normalize_draft(draft)?;
            let mut subscriptions = self
                .read_subscriptions()
                .map_err(StoreReadError::into_subscription_error)?;
            let subscription = Subscription {
                id: subscription_id(),
                provider: normalized.provider,
                plan: normalized.plan,
                amount: normalized.amount,
                currency: normalized.currency,
                cycle: normalized.cycle,
                renewal_day: normalized.renewal_day,
                renewal_month: normalized.renewal_month,
                updated_at: timestamp(),
            };
            subscriptions.push(subscription.clone());
            self.save_subscriptions(subscriptions, "create")?;
            Ok(subscription)
        })
    }
    fn update(&mut self, update: SubscriptionUpdate) -> Result<Subscription, SubscriptionError> {
        self.with_exclusive_lock(|| {
            let id = update.id;
            let normalized = normalize_draft(SubscriptionDraft {
                provider: update.provider,
                plan: update.plan,
                amount: update.amount,
                currency: update.currency,
                cycle: update.cycle,
                renewal_day: update.renewal_day,
                renewal_month: update.renewal_month,
            })?;
            let mut subscriptions = self
                .read_subscriptions()
                .map_err(StoreReadError::into_subscription_error)?;
            let existing = subscriptions
                .iter_mut()
                .find(|item| item.id == id)
                .ok_or_else(|| SubscriptionError::NotFound { id: id.clone() })?;
            existing.provider = normalized.provider;
            existing.plan = normalized.plan;
            existing.amount = normalized.amount;
            existing.currency = normalized.currency;
            existing.cycle = normalized.cycle;
            existing.renewal_day = normalized.renewal_day;
            existing.renewal_month = normalized.renewal_month;
            existing.updated_at = next_timestamp(&existing.updated_at)?;
            let result = existing.clone();
            self.save_subscriptions(subscriptions, "update")?;
            Ok(result)
        })
    }
    fn delete(&mut self, id: &str) -> Result<(), SubscriptionError> {
        self.with_exclusive_lock(|| {
            let mut subscriptions = self
                .read_subscriptions()
                .map_err(StoreReadError::into_subscription_error)?;
            let length = subscriptions.len();
            subscriptions.retain(|item| item.id != id);
            if subscriptions.len() == length {
                return Err(SubscriptionError::NotFound { id: id.into() });
            }
            self.save_subscriptions(subscriptions, "delete")
        })
    }
}

pub fn store_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(FILE_NAME)
}
fn normalize_draft(draft: SubscriptionDraft) -> Result<NormalizedDraft, SubscriptionError> {
    let provider = required_field("provider", draft.provider)?;
    let plan = required_field("plan", draft.plan)?;
    if !draft.amount.is_finite() || draft.amount <= 0.0 {
        return Err(invalid("amount"));
    }
    if !(1..=31).contains(&draft.renewal_day) {
        return Err(invalid("renewalDay"));
    }
    match (draft.cycle, draft.renewal_month) {
        (BillingCycle::Yearly, Some(month)) if (1..=12).contains(&month) => {}
        (BillingCycle::Yearly, _) => return Err(invalid("renewalMonth")),
        _ => {}
    }
    Ok(NormalizedDraft {
        provider,
        plan,
        amount: draft.amount,
        currency: draft.currency,
        cycle: draft.cycle,
        renewal_day: draft.renewal_day,
        renewal_month: draft.renewal_month,
    })
}
fn validate_record(record: &Subscription) -> Result<(), String> {
    if !record.id.starts_with("sub-") || record.id.len() <= 4 {
        return Err("subscription store contains an invalid id".into());
    }
    DateTime::parse_from_rfc3339(&record.updated_at)
        .map_err(|_| "subscription store contains an invalid timestamp".to_string())?;
    normalize_draft(SubscriptionDraft {
        provider: record.provider.clone(),
        plan: record.plan.clone(),
        amount: record.amount,
        currency: record.currency,
        cycle: record.cycle,
        renewal_day: record.renewal_day,
        renewal_month: record.renewal_month,
    })
    .map_err(|error| format!("subscription store contains invalid data: {error:?}"))?;
    Ok(())
}
fn required_field(field: &str, value: String) -> Result<String, SubscriptionError> {
    let trimmed = value.trim().to_string();
    if trimmed.is_empty() {
        Err(invalid(field))
    } else {
        Ok(trimmed)
    }
}
fn invalid(field: &str) -> SubscriptionError {
    SubscriptionError::InvalidInput {
        field: field.into(),
    }
}
fn timestamp() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true)
}
fn next_timestamp(previous: &str) -> Result<String, SubscriptionError> {
    next_timestamp_at(previous, Utc::now())
}

fn next_timestamp_at(previous: &str, now: DateTime<Utc>) -> Result<String, SubscriptionError> {
    let previous = DateTime::parse_from_rfc3339(previous)
        .map_err(|_| SubscriptionError::StoreCorrupt {
            reason: "subscription store contains an invalid timestamp".into(),
        })?
        .with_timezone(&Utc);
    Ok(
        std::cmp::max(now, previous + chrono::Duration::nanoseconds(1))
            .to_rfc3339_opts(SecondsFormat::Nanos, true),
    )
}
fn stage_and_sync(path: &Path, operation: &str, contents: &str) -> io::Result<PathBuf> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = path.with_file_name(format!(
        "{FILE_NAME}.tmp-{operation}-{}-{}",
        std::process::id(),
        unique_suffix()
    ));
    let mut staged = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    if let Err(error) = staged
        .write_all(contents.as_bytes())
        .and_then(|_| staged.sync_all())
    {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    Ok(temporary)
}
fn sync_parent_directory(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        let parent = path.parent().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "subscription store has no parent directory",
            )
        })?;
        fs::File::open(parent)?.sync_all()
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Ok(())
    }
}
fn reconcile_parent_sync(
    parent_sync: io::Result<()>,
    read_persisted: impl FnOnce() -> Result<Vec<Subscription>, StoreReadError>,
    expected: &[Subscription],
) -> Result<(), SubscriptionError> {
    match parent_sync {
        Ok(()) => Ok(()),
        Err(_error) if read_persisted().is_ok_and(|persisted| persisted == expected) => Ok(()),
        Err(error) => Err(SubscriptionError::CommittedWithDurabilityWarning {
            reason: error.to_string(),
        }),
    }
}
fn subscription_id() -> String {
    format!("sub-{}", unique_suffix())
}
fn unique_suffix() -> String {
    let sequence = UNIQUE_SUFFIX.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_nanos())
        .unwrap_or_default();
    format!("{nanos}-{sequence}")
}
fn store_unavailable(reason: String) -> SubscriptionError {
    SubscriptionError::StoreUnavailable { reason }
}
fn store_error(error: impl std::fmt::Display) -> SubscriptionError {
    store_unavailable(error.to_string())
}
#[cfg(test)]
mod tests {
    use super::{
        reconcile_parent_sync, stage_and_sync, JsonSubscriptionRepository, StoreReadError,
        SubscriptionRepository,
    };
    use chrono::{DateTime, Utc};
    use std::fs;
    use std::io;
    use std::path::PathBuf;
    use std::sync::{Arc, Barrier};
    use std::thread;
    use vertice_core::model::{
        BillingCycle, Currency, SubscriptionDraft, SubscriptionError, SubscriptionUpdate,
    };

    fn dir(label: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "vertice-subscription-{label}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn staged_write_is_readable_before_atomic_rename() {
        let target = dir("staged-write").join("subscriptions.json");

        let staged = stage_and_sync(&target, "test", "{\"schemaVersion\":1}").unwrap();

        assert_eq!(
            fs::read_to_string(&staged).unwrap(),
            "{\"schemaVersion\":1}"
        );
        fs::remove_file(staged).unwrap();
    }

    #[test]
    fn parent_sync_failure_is_reconciled_or_reported_as_committed_warning() {
        let mut repository = JsonSubscriptionRepository::new(dir("parent-sync"));
        let expected = vec![repository.create(draft()).unwrap()];
        let sync_error = || Err(io::Error::other("directory sync failed"));

        assert!(reconcile_parent_sync(sync_error(), || Ok(expected.clone()), &expected).is_ok());
        assert!(matches!(
            reconcile_parent_sync(
                sync_error(),
                || Err(StoreReadError::Unavailable("read failed".into())),
                &expected
            ),
            Err(SubscriptionError::CommittedWithDurabilityWarning { .. })
        ));
    }
    fn draft() -> SubscriptionDraft {
        SubscriptionDraft {
            provider: " OpenAI ".into(),
            plan: " Plus ".into(),
            amount: 20.0,
            currency: Currency::Usd,
            cycle: BillingCycle::Monthly,
            renewal_day: 12,
            renewal_month: None,
        }
    }

    #[test]
    fn missing_store_is_empty_and_create_survives_restart() {
        let path = dir("restart");
        let mut repo = JsonSubscriptionRepository::new(path.clone());
        assert!(repo.list().unwrap().is_empty());
        let created = repo.create(draft()).unwrap();
        assert!(created.id.starts_with("sub-"));
        assert_eq!(
            JsonSubscriptionRepository::new(path).list().unwrap(),
            vec![created]
        );
    }
    #[test]
    fn invalid_input_and_not_found_never_mutate_existing_bytes() {
        let path = dir("invalid");
        let mut repo = JsonSubscriptionRepository::new(path.clone());
        let existing = repo.create(draft()).unwrap();
        let file = super::store_path(&path);
        let before = fs::read(&file).unwrap();
        let error = repo
            .create(SubscriptionDraft {
                amount: 0.0,
                ..draft()
            })
            .unwrap_err();
        assert_eq!(
            error,
            SubscriptionError::InvalidInput {
                field: "amount".into()
            }
        );
        assert_eq!(
            repo.delete("missing").unwrap_err(),
            SubscriptionError::NotFound {
                id: "missing".into()
            }
        );
        assert_eq!(fs::read(file).unwrap(), before);
        assert_eq!(repo.list().unwrap(), vec![existing]);
    }

    #[test]
    fn accepts_the_last_day_of_the_month_as_a_renewal_day() {
        let path = dir("last-day");
        let mut repo = JsonSubscriptionRepository::new(path);

        let created = repo
            .create(SubscriptionDraft {
                renewal_day: 31,
                ..draft()
            })
            .unwrap();

        assert_eq!(created.renewal_day, 31);
    }
    #[test]
    fn corrupt_or_semantically_invalid_json_fails_closed_without_rewrite() {
        let path = dir("corrupt");
        let file = super::store_path(&path);
        for contents in [
            "not json",
            r#"{"schemaVersion":1,"subscriptions":[{"id":"sub-1","provider":"x","plan":"p","amount":1,"currency":"eur","cycle":"monthly","renewalDay":0,"renewalMonth":null,"updatedAt":"2026-01-01T00:00:00Z"}]}"#,
            r#"{"schemaVersion":1,"subscriptions":[{"id":"sub-1","provider":"x","plan":"p","amount":1,"currency":"eur","cycle":"yearly","renewalDay":1,"renewalMonth":null,"updatedAt":"2026-01-01T00:00:00Z"}]}"#,
        ] {
            fs::write(&file, contents).unwrap();
            let before = fs::read(&file).unwrap();
            let mut repo = JsonSubscriptionRepository::new(path.clone());
            assert!(matches!(
                repo.list(),
                Err(SubscriptionError::StoreCorrupt { .. })
            ));
            assert!(matches!(
                repo.create(draft()),
                Err(SubscriptionError::StoreCorrupt { .. })
            ));
            assert_eq!(fs::read(&file).unwrap(), before);
        }
    }
    #[test]
    fn repeated_clock_bumps_timestamp_by_exactly_one_nanosecond() {
        let previous = "2026-01-01T00:00:00.000000123Z";
        let repeated_now = DateTime::parse_from_rfc3339(previous)
            .unwrap()
            .with_timezone(&Utc);
        let next = super::next_timestamp_at(previous, repeated_now).unwrap();
        assert_eq!(next, "2026-01-01T00:00:00.000000124Z");
    }

    #[test]
    fn update_is_monotonic_and_concurrent_creates_do_not_lose_records() {
        let path = dir("concurrency");
        let mut repo = JsonSubscriptionRepository::new(path.clone());
        let first = repo.create(draft()).unwrap();
        let updated = repo
            .update(SubscriptionUpdate {
                id: first.id.clone(),
                provider: "Anthropic".into(),
                plan: "Plus".into(),
                amount: 20.0,
                currency: Currency::Usd,
                cycle: BillingCycle::Monthly,
                renewal_day: 12,
                renewal_month: None,
            })
            .unwrap();
        assert_eq!(updated.id, first.id);
        assert!(
            DateTime::parse_from_rfc3339(&updated.updated_at)
                .unwrap()
                .with_timezone(&Utc)
                > DateTime::parse_from_rfc3339(&first.updated_at)
                    .unwrap()
                    .with_timezone(&Utc)
        );
        let barrier = Arc::new(Barrier::new(4));
        let mut handles = vec![];
        for _ in 0..4 {
            let path = path.clone();
            let barrier = barrier.clone();
            handles.push(thread::spawn(move || {
                let mut independent_repository = JsonSubscriptionRepository::new(path);
                barrier.wait();
                independent_repository.create(draft()).unwrap();
            }));
        }
        for handle in handles {
            handle.join().unwrap();
        }
        assert_eq!(repo.list().unwrap().len(), 5);
    }
}
