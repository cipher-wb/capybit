//! SQLite connection wrapper. Single connection behind Mutex; DB ops are
//! cheap (< 1ms typical) so contention isn't a concern at this scale.
//!
//! sqlite-vec extension is registered globally on first DB open via
//! `sqlite3_auto_extension`, so all subsequent connections inherit it.

use std::path::PathBuf;
use std::sync::{Mutex, Once};

use rusqlite::Connection;
use tauri::{AppHandle, Manager};

static VEC_INIT: Once = Once::new();

pub struct Db(pub Mutex<Connection>);

impl Db {
    pub fn open(app: &AppHandle) -> rusqlite::Result<Self> {
        register_vec_extension();
        let path = db_path(app);
        let conn = Connection::open(&path)?;
        init_schema(&conn)?;
        tracing::info!(?path, "opened conversations.sqlite (with sqlite-vec)");
        Ok(Self(Mutex::new(conn)))
    }
}

/// Statically link sqlite-vec into every SQLite connection that opens after
/// this point. Safe to call multiple times; only the first call has effect.
fn register_vec_extension() {
    VEC_INIT.call_once(|| {
        unsafe {
            #[allow(clippy::missing_transmute_annotations)]
            rusqlite::ffi::sqlite3_auto_extension(Some(std::mem::transmute(
                sqlite_vec::sqlite3_vec_init as *const (),
            )));
        }
        tracing::debug!("sqlite-vec auto-extension registered");
    });
}

fn db_path(app: &AppHandle) -> PathBuf {
    let dir = app.path().app_data_dir().expect("app_data_dir");
    let _ = std::fs::create_dir_all(&dir);
    dir.join("conversations.sqlite")
}

fn init_schema(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS messages (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            role TEXT NOT NULL,             -- 'user' | 'capybit'
            content TEXT NOT NULL,
            context_snapshot TEXT,          -- JSON, nullable
            created_at TEXT NOT NULL        -- RFC3339
        );
        CREATE INDEX IF NOT EXISTS idx_messages_created ON messages(created_at);

        CREATE TABLE IF NOT EXISTS daily_summaries (
            date TEXT PRIMARY KEY,          -- 'YYYY-MM-DD' local
            diary TEXT NOT NULL,
            extracted_facts TEXT,           -- JSON array
            created_at TEXT NOT NULL
        );

        -- Vector index for semantic recall (M3). EMBEDDING_DIM must match
        -- llm/embedder.rs::EMBEDDING_DIM (text-embedding-3-small = 1536).
        CREATE VIRTUAL TABLE IF NOT EXISTS vec_memories USING vec0(
            embedding float[1536]
        );

        -- Sidecar metadata keyed by vec_memories rowid.
        CREATE TABLE IF NOT EXISTS vec_meta (
            rowid INTEGER PRIMARY KEY,
            kind TEXT NOT NULL,             -- 'daily_diary' | 'milestone'
            ref_key TEXT NOT NULL,          -- date (YYYY-MM-DD) for diaries
            text TEXT NOT NULL,
            created_at TEXT NOT NULL
        );
        CREATE UNIQUE INDEX IF NOT EXISTS idx_vec_meta_kind_ref
            ON vec_meta(kind, ref_key);
        "#,
    )
}
