use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;

pub type DbPool = Pool<SqliteConnectionManager>;

pub fn init_pool(database_url: &str) -> anyhow::Result<DbPool> {
    let path = database_url
        .strip_prefix("sqlite://")
        .unwrap_or(database_url);
    let manager = SqliteConnectionManager::file(path)
        .with_init(|c| {
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
        "#,
    )
}

/// Run a blocking database operation on a thread pool via spawn_blocking.
pub async fn run_blocking<F, T>(pool: DbPool, f: F) -> anyhow::Result<T>
where
    F: FnOnce(&Connection) -> anyhow::Result<T> + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        f(&conn)
    })
    .await?
}
