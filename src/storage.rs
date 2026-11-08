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
