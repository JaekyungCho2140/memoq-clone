use crate::models::TbEntry;
use anyhow::Result;
use rusqlite::{params, Connection};
use std::path::PathBuf;

fn db_path(tb_id: &str) -> PathBuf {
    let mut path = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("memoq-clone/tb");
    std::fs::create_dir_all(&path).ok();
    path.push(format!("{tb_id}.db"));
    path
}

pub struct TbDb {
    conn: Connection,
}

impl TbDb {
    pub fn init(tb_id: &str, name: &str) -> Result<()> {
        let conn = Connection::open(db_path(tb_id))?;
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS entries (
                id TEXT PRIMARY KEY, source_term TEXT NOT NULL, target_term TEXT NOT NULL,
                source_lang TEXT NOT NULL, target_lang TEXT NOT NULL,
                notes TEXT NOT NULL DEFAULT '', forbidden INTEGER NOT NULL DEFAULT 0
            );
            CREATE INDEX IF NOT EXISTS idx_term ON entries(source_term, source_lang);
        ",
        )?;
        conn.execute(
            "INSERT OR REPLACE INTO meta VALUES ('name', ?1)",
            params![name],
        )?;
        Ok(())
    }

    pub fn open(tb_id: &str) -> Result<Self> {
        Ok(Self {
            conn: Connection::open(db_path(tb_id))?,
        })
    }

    pub fn insert(&self, entry: &TbEntry) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO entries (id,source_term,target_term,source_lang,target_lang,notes,forbidden) VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![entry.id, entry.source_term, entry.target_term, entry.source_lang, entry.target_lang, entry.notes, entry.forbidden as i32],
        )?;
        Ok(())
    }

    pub fn search(&self, term: &str, source_lang: &str) -> Result<Vec<TbEntry>> {
        let pattern = format!("%{term}%");
        let mut stmt = self.conn.prepare(
            "SELECT id,source_term,target_term,source_lang,target_lang,notes,forbidden FROM entries WHERE source_lang=?1 AND source_term LIKE ?2 ORDER BY forbidden DESC"
        )?;
        let entries = stmt
            .query_map(params![source_lang, pattern], |row| {
                Ok(TbEntry {
                    id: row.get(0)?,
                    source_term: row.get(1)?,
                    target_term: row.get(2)?,
                    source_lang: row.get(3)?,
                    target_lang: row.get(4)?,
                    notes: row.get(5)?,
                    forbidden: row.get::<_, i32>(6)? != 0,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(entries)
    }
}
