//! Memory layer (PRD §3.4).
//!
//! M3 scope (this push):
//!   - Layer 1: profile.json (lives in profile.rs, already done in M5 birth)
//!   - Layer 2: conversations.sqlite — messages + daily_summaries
//!   - Daily summary trigger + fact extraction → write back to profile
//!
//! Deferred to next push:
//!   - Layer 3: vectors.sqlite (sqlite-vec embeddings + 召唤词 recall)
//!   - Manual-audit UI for low-confidence facts

pub mod conversations;
pub mod db;
pub mod summaries;
pub mod vectors;

pub use conversations::{recent_history, write_message, MessageRow};
pub use db::Db;
pub use summaries::{unsummarized_dates, write_summary};
pub use vectors::{query_similar, unindexed_diaries, upsert_memory};
