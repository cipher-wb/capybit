//! daily_summaries table read/write.

use rusqlite::params;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use super::db::Db;

pub fn write_summary(
    db: &Db,
    date: &str,
    diary: &str,
    extracted_facts_json: Option<&str>,
) -> rusqlite::Result<()> {
    let now = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "unknown".into());
    let conn = db.0.lock().unwrap();
    conn.execute(
        "INSERT OR REPLACE INTO daily_summaries (date, diary, extracted_facts, created_at) \
         VALUES (?1, ?2, ?3, ?4)",
        params![date, diary, extracted_facts_json, now],
    )?;
    Ok(())
}

/// Local dates that have messages but no summary yet, oldest first.
/// Caller decides how many to actually run (cap on first launch).
pub fn unsummarized_dates(db: &Db, today_local: &str) -> rusqlite::Result<Vec<String>> {
    let with_messages = super::conversations::dates_with_messages(db)?;
    let summarized: std::collections::HashSet<String> = {
        let conn = db.0.lock().unwrap();
        let mut stmt = conn.prepare("SELECT date FROM daily_summaries")?;
        let collected: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(0))?
            .filter_map(|r| r.ok())
            .collect();
        collected.into_iter().collect()
    };
    Ok(with_messages
        .into_iter()
        .filter(|d| !summarized.contains(d))
        // Don't summarize "today" mid-day; wait until 23:59 or next-day catch-up.
        .filter(|d| d.as_str() != today_local)
        .collect())
}

/// Recent N diaries (oldest→newest) for injection into the system prompt
/// `{recent_summaries}` placeholder.
pub fn recent_diaries(db: &Db, n: usize) -> rusqlite::Result<Vec<(String, String)>> {
    let conn = db.0.lock().unwrap();
    let mut stmt =
        conn.prepare("SELECT date, diary FROM daily_summaries ORDER BY date DESC LIMIT ?1")?;
    let rows = stmt
        .query_map(params![n as i64], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows.into_iter().rev().collect())
}
