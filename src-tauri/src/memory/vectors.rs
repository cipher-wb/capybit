//! Vector index reads & writes (sqlite-vec).

use rusqlite::params;
use serde::{Deserialize, Serialize};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use super::db::Db;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecalledMemory {
    pub kind: String,
    pub ref_key: String,
    pub text: String,
    pub distance: f32,
}

/// Idempotent insert keyed by (kind, ref_key). If the (kind, ref_key) pair
/// already has a row, the embedding is updated in place — useful when a
/// daily summary gets re-generated.
pub fn upsert_memory(
    db: &Db,
    kind: &str,
    ref_key: &str,
    text: &str,
    embedding: &[f32],
) -> rusqlite::Result<()> {
    let bytes = embedding_to_bytes(embedding);
    let now = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "unknown".into());

    let conn = db.0.lock().unwrap();
    let existing: Option<i64> = conn
        .query_row(
            "SELECT rowid FROM vec_meta WHERE kind = ?1 AND ref_key = ?2",
            params![kind, ref_key],
            |r| r.get(0),
        )
        .ok();

    if let Some(rowid) = existing {
        conn.execute(
            "UPDATE vec_memories SET embedding = ?1 WHERE rowid = ?2",
            params![bytes, rowid],
        )?;
        conn.execute(
            "UPDATE vec_meta SET text = ?1, created_at = ?2 WHERE rowid = ?3",
            params![text, now, rowid],
        )?;
    } else {
        conn.execute(
            "INSERT INTO vec_memories(embedding) VALUES (?1)",
            params![bytes],
        )?;
        let rowid = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO vec_meta(rowid, kind, ref_key, text, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![rowid, kind, ref_key, text, now],
        )?;
    }
    Ok(())
}

pub fn query_similar(
    db: &Db,
    query_embedding: &[f32],
    limit: usize,
) -> rusqlite::Result<Vec<RecalledMemory>> {
    let bytes = embedding_to_bytes(query_embedding);
    let conn = db.0.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT m.kind, m.ref_key, m.text, v.distance \
         FROM vec_memories v \
         JOIN vec_meta m ON m.rowid = v.rowid \
         WHERE v.embedding MATCH ?1 \
         ORDER BY v.distance \
         LIMIT ?2",
    )?;
    let rows = stmt
        .query_map(params![bytes, limit as i64], |row| {
            Ok(RecalledMemory {
                kind: row.get(0)?,
                ref_key: row.get(1)?,
                text: row.get(2)?,
                distance: row.get(3)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Diaries that exist in `daily_summaries` but not yet in `vec_meta`.
/// Returns `(date, diary_text)` pairs, oldest first.
pub fn unindexed_diaries(db: &Db) -> rusqlite::Result<Vec<(String, String)>> {
    let conn = db.0.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT s.date, s.diary FROM daily_summaries s \
         WHERE NOT EXISTS ( \
             SELECT 1 FROM vec_meta v WHERE v.kind = 'daily_diary' AND v.ref_key = s.date \
         ) \
         ORDER BY s.date ASC",
    )?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

fn embedding_to_bytes(emb: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(emb.len() * 4);
    for f in emb {
        bytes.extend_from_slice(&f.to_le_bytes());
    }
    bytes
}
