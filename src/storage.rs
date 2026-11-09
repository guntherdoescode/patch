//! Local SQLite persistence. Secrets are intentionally not stored here.

use rusqlite::{Connection, Result};
use std::path::PathBuf;

pub fn data_dir() -> Result<PathBuf> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or(rusqlite::Error::InvalidPath(PathBuf::from("HOME is not set")))?;
    let dir = home.join(".patch");
    std::fs::create_dir_all(&dir)
        .map_err(|_| rusqlite::Error::InvalidPath(dir.clone()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))
            .map_err(|_| rusqlite::Error::InvalidPath(dir.clone()))?;
    }

    Ok(dir)
}

pub fn open_database() -> Result<Connection> {
    let path = data_dir()?.join("patch.db");
    let connection = Connection::open(path)?;
    connection.execute_batch(
        "
        PRAGMA foreign_keys = ON;
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;

        CREATE TABLE IF NOT EXISTS workspaces (
            id INTEGER PRIMARY KEY,
            canonical_path TEXT NOT NULL UNIQUE,
            display_name TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            last_opened_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS chats (
            id INTEGER PRIMARY KEY,
            workspace_id INTEGER NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
            title TEXT NOT NULL DEFAULT 'New chat',
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );

        CREATE INDEX IF NOT EXISTS chats_workspace_updated
            ON chats(workspace_id, updated_at DESC);

        CREATE TABLE IF NOT EXISTS messages (
            id INTEGER PRIMARY KEY,
            chat_id INTEGER NOT NULL REFERENCES chats(id) ON DELETE CASCADE,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            input_tokens INTEGER,
            output_tokens INTEGER,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );

        CREATE INDEX IF NOT EXISTS messages_chat_order
            ON messages(chat_id, id);

        CREATE TABLE IF NOT EXISTS provider_settings (
            provider TEXT PRIMARY KEY,
            base_url TEXT NOT NULL,
            model_id TEXT,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS model_pricing (
            provider TEXT NOT NULL,
            model_id TEXT NOT NULL,
            input_usd_per_million REAL NOT NULL,
            output_usd_per_million REAL NOT NULL,
            verified_at TEXT,
            source TEXT,
            PRIMARY KEY(provider, model_id)
        );
        ",
    )?;
    Ok(connection)
}

pub fn register_workspace(path: &std::path::Path) -> Result<i64> {
    let canonical = path.canonicalize()
        .map_err(|_| rusqlite::Error::InvalidPath(path.to_path_buf()))?;
    let display_name = canonical.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Workspace");
    let connection = open_database()?;
    connection.execute(
        "INSERT INTO workspaces (canonical_path, display_name) VALUES (?1, ?2)
         ON CONFLICT(canonical_path) DO UPDATE SET
         last_opened_at = CURRENT_TIMESTAMP",
        rusqlite::params![canonical.to_string_lossy(), display_name],
    )?;
    connection.query_row(
        "SELECT id FROM workspaces WHERE canonical_path = ?1",
        [canonical.to_string_lossy().as_ref()],
        |row| row.get::<_, String>(0).map(PathBuf::from),
    )
}

pub fn list_chat_titles(workspace_id: i64) -> Result<Vec<(i64, String)>> {
    let connection = open_database()?;
    let mut statement = connection.prepare(
        "SELECT id, title FROM chats WHERE workspace_id = ?1 ORDER BY updated_at DESC"
    )?;
    let rows = statement.query_map([workspace_id], |row| {
        Ok((row.get(0)?, row.get(1)?))
    })?;
    rows.collect()
}

pub fn create_chat(workspace_id: i64) -> Result<i64> {
    let connection = open_database()?;
    connection.execute(
        "INSERT INTO chats (workspace_id, title) VALUES (?1, 'New chat')",
        [workspace_id],
    )?;
    Ok(connection.last_insert_rowid())
}

/// Store an explicit rate card. Rates are USD per million tokens and should
/// only be written from a verified provider/model price source.
pub fn set_model_pricing(
    provider: &str,
    model_id: &str,
    input_usd_per_million: f64,
    output_usd_per_million: f64,
    source: &str,
) -> Result<()> {
    let connection = open_database()?;
    connection.execute(
        "INSERT INTO model_pricing
         (provider, model_id, input_usd_per_million, output_usd_per_million,
          verified_at, source)
         VALUES (?1, ?2, ?3, ?4, CURRENT_TIMESTAMP, ?5)
         ON CONFLICT(provider, model_id) DO UPDATE SET
         input_usd_per_million = excluded.input_usd_per_million,
         output_usd_per_million = excluded.output_usd_per_million,
         verified_at = CURRENT_TIMESTAMP,
         source = excluded.source",
        rusqlite::params![
            provider,
            model_id,
            input_usd_per_million,
            output_usd_per_million,
            source
        ],
    )?;
    Ok(())
}

pub fn get_model_pricing(
    provider: &str,
    model_id: &str,
) -> Result<Option<crate::usage::TokenPricing>> {
    let connection = open_database()?;
    let mut statement = connection.prepare(
        "SELECT input_usd_per_million, output_usd_per_million
         FROM model_pricing WHERE provider = ?1 AND model_id = ?2"
    )?;
    let mut rows = statement.query(rusqlite::params![provider, model_id])?;
    match rows.next()? {
        Some(row) => Ok(Some(crate::usage::TokenPricing {
            input_usd_per_million: row.get(0)?,
            output_usd_per_million: row.get(1)?,
        })),
        None => Ok(None),
    }
}

pub fn save_message(
    chat_id: i64,
    role: &str,
    content: &str,
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
) -> Result<i64> {
    let connection = open_database()?;
    connection.execute(
        "INSERT INTO messages (chat_id, role, content, input_tokens, output_tokens)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![chat_id, role, content, input_tokens, output_tokens],
    )?;
    connection.execute(
        "UPDATE chats SET updated_at = CURRENT_TIMESTAMP WHERE id = ?1",
        [chat_id],
    )?;
    Ok(connection.last_insert_rowid())
}


/// Messages are returned in insertion order so a chat can be restored exactly.
pub fn list_messages(chat_id: i64) -> Result<Vec<(String, String)>> {
    let connection = open_database()?;
    let mut statement = connection.prepare(
        "SELECT role, content FROM messages WHERE chat_id = ?1 ORDER BY id"
    )?;
    let rows = statement.query_map([chat_id], |row| {
        Ok((row.get(0)?, row.get(1)?))
    })?;
    rows.collect()
}

/// Rename a chat without changing its workspace or message history.
pub fn rename_chat(chat_id: i64, title: &str) -> Result<()> {
    let title = title.trim();
    if title.is_empty() {
        return Err(rusqlite::Error::InvalidParameterName(
            "Chat title cannot be empty".to_owned()
        ));
    }
    open_database()?.execute(
        "UPDATE chats SET title = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
        rusqlite::params![title, chat_id],
    )?;
    Ok(())
}

/// Delete a chat. Its messages are removed through the chats foreign-key cascade.
pub fn delete_chat(chat_id: i64) -> Result<()> {
    open_database()?.execute("DELETE FROM chats WHERE id = ?1", [chat_id])?;
    Ok(())
}

/// Resolve the canonical workspace path for file browsing.
pub fn workspace_path(workspace_id: i64) -> Result<PathBuf> {
    open_database()?.query_row(
        "SELECT canonical_path FROM workspaces WHERE id = ?1",
        [workspace_id],
        |row| row.get(0),
    )
}
