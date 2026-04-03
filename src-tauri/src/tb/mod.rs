mod storage;

use crate::models::TbEntry;
use anyhow::Result;
use uuid::Uuid;

pub struct TbEngine {
    db: storage::TbDb,
}

impl TbEngine {
    pub fn create(name: &str) -> Result<String> {
        let id = Uuid::new_v4().to_string();
        storage::TbDb::init(&id, name)?;
        Ok(id)
    }

    pub fn open(tb_id: &str) -> Result<Self> {
        Ok(Self {
            db: storage::TbDb::open(tb_id)?,
        })
    }

    pub fn add(
        &self,
        source_term: &str,
        target_term: &str,
        source_lang: &str,
        target_lang: &str,
        notes: &str,
        forbidden: bool,
    ) -> Result<TbEntry> {
        let entry = TbEntry {
            id: Uuid::new_v4().to_string(),
            source_term: source_term.to_string(),
            target_term: target_term.to_string(),
            source_lang: source_lang.to_string(),
            target_lang: target_lang.to_string(),
            notes: notes.to_string(),
            forbidden,
        };
        self.db.insert(&entry)?;
        Ok(entry)
    }

    pub fn lookup(&self, term: &str, source_lang: &str) -> Result<Vec<TbEntry>> {
        self.db.search(term, source_lang)
    }

    pub fn all_entries(&self) -> Result<Vec<TbEntry>> {
        self.db.all()
    }
}
