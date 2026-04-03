use crate::models::TmEntry;
use anyhow::Result;
use rusqlite::{Connection, params};
use std::path::PathBuf;

fn db_path(tm_id: &str) -> PathBuf {
    let mut path = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("memoq-clone/tm");
    std::fs::create_dir_all(&path).ok();
    path.push(format!("{tm_id}.db"));
    path
}

pub struct TmDb { conn: Connection }

impl TmDb {
    pub fn init(tm_id: &str, name: &str, source_lang: &str, target_lang: &str) -> Result<()> {
        let conn = Connection::open(db_path(tm_id))?;
        conn.execute_batch("
            CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS entries (
                id TEXT PRIMARY KEY, source TEXT NOT NULL, target TEXT NOT NULL,
                source_lang TEXT NOT NULL, target_lang TEXT NOT NULL, created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_lang ON entries(source_lang, target_lang);
        ")?;
        conn.execute("INSERT OR REPLACE INTO meta VALUES ('name', ?1), ('source_lang', ?2), ('target_lang', ?3)", params![name, source_lang, target_lang])?;
        Ok(())
    }

    pub fn open(tm_id: &str) -> Result<Self> {
        Ok(Self { conn: Connection::open(db_path(tm_id))? })
    }

    pub fn insert(&self, entry: &TmEntry) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO entries (id, source, target, source_lang, target_lang, created_at) VALUES (?1,?2,?3,?4,?5,?6)",
            params![entry.id, entry.source, entry.target, entry.source_lang, entry.target_lang, entry.created_at.to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn all(&self, source_lang: &str, target_lang: &str) -> Result<Vec<TmEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id,source,target,source_lang,target_lang,created_at FROM entries WHERE source_lang=?1 AND target_lang=?2"
        )?;
        let entries = stmt.query_map(params![source_lang, target_lang], |row| {
            Ok((row.get::<_,String>(0)?, row.get::<_,String>(1)?, row.get::<_,String>(2)?,
                row.get::<_,String>(3)?, row.get::<_,String>(4)?, row.get::<_,String>(5)?))
        })?.filter_map(|r| r.ok()).map(|(id,source,target,sl,tl,ts)| TmEntry {
            id, source, target, source_lang: sl, target_lang: tl,
            created_at: ts.parse().unwrap_or_default(), metadata: Default::default()
        }).collect();
        Ok(entries)
    }
}
