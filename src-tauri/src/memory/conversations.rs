//! conversations table read/write.

use rusqlite::params;
use serde::{Deserialize, Serialize};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use super::db::Db;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageRow {
    pub id: i64,
    pub role: String,
    pub content: String,
    pub created_at: String,
}

/// Write a turn (user or capybit). Returns the row id.
///
/// `context_snapshot_json` is the JSON-stringified perception snapshot at
/// turn time. M4 will fill this with active window / time / state numbers;
/// M3 just passes `None` for now.
pub fn write_message(
    db: &Db,
    role: &str,
    content: &str,
    context_snapshot_json: Option<&str>,
) -> rusqlite::Result<i64> {
    let now = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "unknown".into());
    let conn = db.0.lock().unwrap();
    conn.execute(
        "INSERT INTO messages (role, content, context_snapshot, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![role, content, context_snapshot_json, now],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Pull the most recent N turns, oldest first (so we can drop straight into
/// chat history). PRD §3.2.3 says "最近 5 轮" — caller picks N.
pub fn recent_history(db: &Db, n: usize) -> rusqlite::Result<Vec<MessageRow>> {
    let conn = db.0.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, role, content, created_at FROM messages \
         ORDER BY id DESC LIMIT ?1",
    )?;
    let rows = stmt
        .query_map(params![n as i64], |row| {
            Ok(MessageRow {
                id: row.get(0)?,
                role: row.get(1)?,
                content: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows.into_iter().rev().collect())
}

/// All messages for one local-date day, oldest first. Used by the daily
/// summarizer to feed the LLM the full transcript.
pub fn messages_for_date(db: &Db, date_local: &str) -> rusqlite::Result<Vec<MessageRow>> {
    // We store created_at in UTC RFC3339; convert per-row to local date for
    // comparison. Cheaper at write time would be to also store a local-date
    // column — TODO(cipher) if this query becomes hot. [due: M6]
    let conn = db.0.lock().unwrap();
    let mut stmt =
        conn.prepare("SELECT id, role, content, created_at FROM messages ORDER BY id ASC")?;
    let all = stmt
        .query_map([], |row| {
            Ok(MessageRow {
                id: row.get(0)?,
                role: row.get(1)?,
                content: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    Ok(all
        .into_iter()
        .filter(|m| utc_to_local_date(&m.created_at).as_deref() == Some(date_local))
        .collect())
}

/// Distinct local dates for which messages exist but no summary row does yet.
pub fn dates_with_messages(db: &Db) -> rusqlite::Result<Vec<String>> {
    let conn = db.0.lock().unwrap();
    let mut stmt = conn.prepare("SELECT created_at FROM messages")?;
    let mut dates: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .filter_map(|r| r.ok())
        .filter_map(|ts| utc_to_local_date(&ts))
        .collect();
    dates.sort();
    dates.dedup();
    Ok(dates)
}

fn utc_to_local_date(rfc3339: &str) -> Option<String> {
    let dt = OffsetDateTime::parse(rfc3339, &Rfc3339).ok()?;
    let offset = time::UtcOffset::current_local_offset().unwrap_or(time::UtcOffset::UTC);
    let local = dt.to_offset(offset);
    Some(format!(
        "{:04}-{:02}-{:02}",
        local.year(),
        local.month() as u8,
        local.day()
    ))
}
