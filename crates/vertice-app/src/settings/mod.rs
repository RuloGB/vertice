//! The durable `settings.json` document: `locale`, `enabled`,
//! `disclosure_seen` (`add-locale-persistence`). Distinct from
//! `freshness::cache`, which stays a disposable, TTL'd cache with different
//! write semantics.

pub mod store;
