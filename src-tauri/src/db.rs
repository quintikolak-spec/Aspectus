use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Mutex;

/// Wraps a single SQLite connection behind a mutex. sysinfo/network calls
/// live in async tasks, but SQLite access itself is kept synchronous and
/// short-lived — cheaper than pooling for a single-user desktop app.
pub struct Db(pub Mutex<Connection>);

pub fn app_data_dir() -> Result<PathBuf> {
    let dir = dirs_next_data_dir().context("could not resolve app data directory")?;
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

// Deliberately not pulling in the `dirs` crate for one lookup — a couple of
// std/env based fallbacks keep the dependency tree (and thus binary size
// and RAM footprint) smaller, in line with the ressourcenschonend goal.
fn dirs_next_data_dir() -> Option<PathBuf> {
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        return Some(PathBuf::from(xdg).join("personal-dashboard"));
    }
    if let Ok(home) = std::env::var("HOME") {
        return Some(PathBuf::from(home).join(".local/share/personal-dashboard"));
    }
    if let Ok(appdata) = std::env::var("APPDATA") {
        return Some(PathBuf::from(appdata).join("PersonalDashboard"));
    }
    None
}

pub fn open() -> Result<Connection> {
    let path = app_data_dir()?.join("dashboard.sqlite3");
    let conn = Connection::open(path)?;
    conn.execute_batch(
        "
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;

        CREATE TABLE IF NOT EXISTS settings (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            json TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS sources (
            id TEXT PRIMARY KEY,
            url TEXT NOT NULL UNIQUE,
            title TEXT NOT NULL,
            category TEXT NOT NULL,
            kind TEXT NOT NULL,
            last_checked_at TEXT,
            last_updated_at TEXT,
            healthy INTEGER NOT NULL DEFAULT 1,
            last_error TEXT
        );

        -- One row per item ever seen for a source, keyed by a content hash.
        -- This is the de-dup mechanism from section 10: before calling the
        -- AI we check whether this hash already exists.
        CREATE TABLE IF NOT EXISTS seen_items (
            source_id TEXT NOT NULL,
            item_hash TEXT NOT NULL,
            item_url TEXT NOT NULL,
            seen_at TEXT NOT NULL,
            PRIMARY KEY (source_id, item_hash)
        );

        CREATE TABLE IF NOT EXISTS news_cards (
            id TEXT PRIMARY KEY,
            source_id TEXT NOT NULL,
            source_title TEXT NOT NULL,
            original_url TEXT NOT NULL,
            headline TEXT NOT NULL,
            summary TEXT NOT NULL,
            category TEXT NOT NULL,
            importance REAL NOT NULL,
            published_at TEXT,
            fetched_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_news_cards_fetched_at ON news_cards (fetched_at DESC);

        -- Generic key/value cache for anything else worth persisting
        -- across restarts (weather snapshots, etag/last-modified headers,
        -- etc.) per section 12.
        CREATE TABLE IF NOT EXISTS kv_cache (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            expires_at TEXT
        );
        ",
    )?;
    Ok(conn)
}
