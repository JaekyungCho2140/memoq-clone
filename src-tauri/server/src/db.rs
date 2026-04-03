use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;

pub type DbPool = Pool<SqliteConnectionManager>;

pub fn init_pool(database_url: &str) -> anyhow::Result<DbPool> {
    let path = database_url
        .strip_prefix("sqlite://")
        .unwrap_or(database_url);
    let manager = SqliteConnectionManager::file(path).with_init(|c| {
        c.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        Ok(())
    });
    let pool = Pool::new(manager)?;
    Ok(pool)
}

/// Create an in-memory pool for tests.
/// Each call with a unique `name` gets an isolated in-memory database.
/// Uses `max_size(1)` so the single connection persists and data survives
/// across handler calls within the same pool.
pub fn in_memory_pool_named(name: &str) -> anyhow::Result<DbPool> {
    let uri = format!("file:{}?mode=memory&cache=shared", name);
    let manager = SqliteConnectionManager::file(&uri)
        .with_flags(
            rusqlite::OpenFlags::SQLITE_OPEN_URI
                | rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE
                | rusqlite::OpenFlags::SQLITE_OPEN_CREATE,
        )
        .with_init(|c| {
            c.execute_batch("PRAGMA foreign_keys=ON;")?;
            Ok(())
        });
    let pool = Pool::builder().max_size(1).build(manager)?;
    Ok(pool)
}

pub async fn run_migrations(pool: &DbPool) -> anyhow::Result<()> {
    let conn = pool.get()?;
    create_schema(&conn)?;
    Ok(())
}

fn create_schema(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id            TEXT PRIMARY KEY,
            username      TEXT NOT NULL UNIQUE,
            email         TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            role          TEXT NOT NULL DEFAULT 'translator',
            created_at    TEXT NOT NULL,
            updated_at    TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS refresh_tokens (
            id         TEXT PRIMARY KEY,
            user_id    TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            token_hash TEXT NOT NULL UNIQUE,
            expires_at TEXT NOT NULL,
            created_at TEXT NOT NULL,
            revoked    INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS projects (
            id          TEXT PRIMARY KEY,
            name        TEXT NOT NULL,
            source_lang TEXT NOT NULL,
            target_lang TEXT NOT NULL,
            owner_id    TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            created_at  TEXT NOT NULL,
            updated_at  TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS project_files (
            id         TEXT PRIMARY KEY,
            project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            name       TEXT NOT NULL,
            file_path  TEXT NOT NULL,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS segments (
            id         TEXT PRIMARY KEY,
            file_id    TEXT NOT NULL REFERENCES project_files(id) ON DELETE CASCADE,
            seg_order  INTEGER NOT NULL,
            source     TEXT NOT NULL,
            target     TEXT NOT NULL DEFAULT '',
            status     TEXT NOT NULL DEFAULT 'untranslated',
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS tm_entries (
            id          TEXT PRIMARY KEY,
            source      TEXT NOT NULL,
            target      TEXT NOT NULL,
            source_lang TEXT NOT NULL,
            target_lang TEXT NOT NULL,
            owner_id    TEXT REFERENCES users(id) ON DELETE SET NULL,
            created_at  TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS tb_entries (
            id          TEXT PRIMARY KEY,
            source_term TEXT NOT NULL,
            target_term TEXT NOT NULL,
            source_lang TEXT NOT NULL,
            target_lang TEXT NOT NULL,
            notes       TEXT NOT NULL DEFAULT '',
            forbidden   INTEGER NOT NULL DEFAULT 0,
            owner_id    TEXT REFERENCES users(id) ON DELETE SET NULL,
            created_at  TEXT NOT NULL
        );
        "#,
    )
}

/// Run a blocking database closure on Tokio's blocking thread pool.
pub async fn run_db<F, T>(pool: DbPool, f: F) -> crate::error::AppResult<T>
where
    F: FnOnce(&Connection) -> crate::error::AppResult<T> + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(move || {
        let conn = pool
            .get()
            .map_err(|e| crate::error::AppError::Internal(anyhow::anyhow!(e)))?;
        f(&conn)
    })
    .await
    .map_err(|e| crate::error::AppError::Internal(anyhow::anyhow!("spawn_blocking: {}", e)))?
}
