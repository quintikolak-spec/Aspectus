use crate::db::Db;
use anyhow::Result;
use chrono::{Duration, Utc};
use tauri::State;

pub fn get(db: &State<Db>, key: &str) -> Result<Option<String>> {
    let conn = db.0.lock().unwrap();
    let row: Option<(String, Option<String>)> = conn
        .query_row(
            "SELECT value, expires_at FROM kv_cache WHERE key = ?1",
            [key],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .ok();

    match row {
        Some((value, Some(expires_at))) => {
            let expires: chrono::DateTime<Utc> = expires_at.parse()?;
            if Utc::now() < expires {
                Ok(Some(value))
            } else {
                Ok(None) // stale — caller should refetch and call `set` again
            }
        }
        Some((value, None)) => Ok(Some(value)),
        None => Ok(None),
    }
}

pub fn set(db: &State<Db>, key: &str, value: &str, ttl_seconds: Option<i64>) -> Result<()> {
    let expires_at = ttl_seconds.map(|s| (Utc::now() + Duration::seconds(s)).to_rfc3339());
    let conn = db.0.lock().unwrap();
    conn.execute(
        "INSERT INTO kv_cache (key, value, expires_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, expires_at = excluded.expires_at",
        rusqlite::params![key, value, expires_at],
    )?;
    Ok(())
}
