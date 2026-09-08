use crate::db::Db;
use crate::models::Source;
use anyhow::{anyhow, Result};
use chrono::Utc;
use sha2::{Digest, Sha256};
use tauri::State;
use url::Url;
use uuid::Uuid;

/// A single item found in a source, before it has been through the AI
/// relevance/summary step. `hash` is what section 10 calls "geeignete IDs,
/// URLs, Hashes" for detecting already-processed content.
pub struct RawItem {
    pub title: String,
    pub url: String,
    pub content: String,
    pub published_at: Option<String>,
    pub hash: String,
}

pub async fn add_source(db: &State<'_, Db>, url: &str, category: &str) -> Result<Source> {
    let parsed = Url::parse(url).map_err(|_| anyhow!("invalid_url"))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(anyhow!("unsupported_scheme")); // guards against SSRF via file://, etc.
    }

    let (kind, title) = probe_source(url).await?;

    let source = Source {
        id: Uuid::new_v4().to_string(),
        url: url.to_string(),
        title,
        category: category.to_string(),
        kind,
        last_checked_at: None,
        last_updated_at: None,
        healthy: true,
        last_error: None,
    };

    let conn = db.0.lock().unwrap();
    conn.execute(
        "INSERT INTO sources (id, url, title, category, kind, healthy) VALUES (?1, ?2, ?3, ?4, ?5, 1)",
        rusqlite::params![source.id, source.url, source.title, source.category, source.kind],
    )?;

    Ok(source)
}

pub fn list_sources(db: &State<Db>) -> Result<Vec<Source>> {
    let conn = db.0.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, url, title, category, kind, last_checked_at, last_updated_at, healthy, last_error FROM sources",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(Source {
            id: r.get(0)?,
            url: r.get(1)?,
            title: r.get(2)?,
            category: r.get(3)?,
            kind: r.get(4)?,
            last_checked_at: r.get(5)?,
            last_updated_at: r.get(6)?,
            healthy: r.get::<_, i64>(7)? != 0,
            last_error: r.get(8)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn remove_source(db: &State<Db>, id: &str) -> Result<()> {
    let conn = db.0.lock().unwrap();
    conn.execute("DELETE FROM sources WHERE id = ?1", [id])?;
    conn.execute("DELETE FROM seen_items WHERE source_id = ?1", [id])?;
    Ok(())
}

/// Tries to find an RSS/Atom feed first (cheaper + more reliable per section
/// 6), falling back to HTML scraping. Returns (kind, human title).
async fn probe_source(url: &str) -> Result<(String, String)> {
    let client = http_client()?;

    if let Ok(resp) = client.get(url).send().await {
        if let Ok(bytes) = resp.bytes().await {
            if let Ok(feed) = feed_rs::parser::parse(&bytes[..]) {
                let title = feed.title.map(|t| t.content).unwrap_or_else(|| url.to_string());
                return Ok(("rss".to_string(), title));
            }
        }
    }

    // Not a feed — try common autodiscovery / fall back to HTML title.
    let html = client.get(url).send().await?.text().await?;
    let document = scraper::Html::parse_document(&html);
    let title_selector = scraper::Selector::parse("title").unwrap();
    let title = document
        .select(&title_selector)
        .next()
        .map(|n| n.text().collect::<String>())
        .unwrap_or_else(|| url.to_string());

    Ok(("html".to_string(), title.trim().to_string()))
}

pub async fn fetch_new_items(
    db: &State<'_, Db>,
    source: &Source,
) -> Result<Vec<RawItem>> {
    let items = match source.kind.as_str() {
        "rss" => fetch_rss(&source.url).await?,
        _ => fetch_html(&source.url).await?,
    };

    let conn = db.0.lock().unwrap();
    let mut new_items = Vec::new();
    for item in items {
        let exists: bool = conn
            .query_row(
                "SELECT 1 FROM seen_items WHERE source_id = ?1 AND item_hash = ?2",
                rusqlite::params![source.id, item.hash],
                |_| Ok(true),
            )
            .unwrap_or(false);

        if !exists {
            conn.execute(
                "INSERT INTO seen_items (source_id, item_hash, item_url, seen_at) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![source.id, item.hash, item.url, Utc::now().to_rfc3339()],
            )?;
            new_items.push(item);
        }
    }

    conn.execute(
        "UPDATE sources SET last_checked_at = ?1, healthy = 1, last_error = NULL WHERE id = ?2",
        rusqlite::params![Utc::now().to_rfc3339(), source.id],
    )?;

    Ok(new_items)
}

async fn fetch_rss(url: &str) -> Result<Vec<RawItem>> {
    let client = http_client()?;
    let bytes = client.get(url).send().await?.bytes().await?;
    let feed = feed_rs::parser::parse(&bytes[..])?;

    Ok(feed
        .entries
        .into_iter()
        .filter_map(|entry| {
            let link = entry.links.first()?.href.clone();
            let title = entry.title.map(|t| t.content).unwrap_or_default();
            let content = entry
                .summary
                .map(|s| s.content)
                .or_else(|| entry.content.and_then(|c| c.body))
                .unwrap_or_default();
            let published_at = entry.published.map(|d| d.to_rfc3339());
            let hash = hash_item(&link, &title);
            Some(RawItem { title, url: link, content, published_at, hash })
        })
        .collect())
}

async fn fetch_html(url: &str) -> Result<Vec<RawItem>> {
    // Minimal generic extraction: real implementation should use a
    // readability-style content extractor. Kept intentionally simple here —
    // this is the piece most likely to need per-site tuning in practice,
    // and section 6 asks the app to detect and surface unreliable sources
    // rather than silently failing.
    let client = http_client()?;
    let html = client.get(url).send().await?.text().await?;
    let document = scraper::Html::parse_document(&html);

    let article_selector = scraper::Selector::parse("article, .post, .entry").unwrap();
    let title_selector = scraper::Selector::parse("h1, h2, .title").unwrap();
    let link_selector = scraper::Selector::parse("a").unwrap();

    let mut items = Vec::new();
    for el in document.select(&article_selector).take(10) {
        let title = el
            .select(&title_selector)
            .next()
            .map(|n| n.text().collect::<String>())
            .unwrap_or_default();
        let link = el
            .select(&link_selector)
            .next()
            .and_then(|n| n.value().attr("href"))
            .map(|href| resolve_url(url, href))
            .unwrap_or_else(|| url.to_string());
        let content = el.text().collect::<String>();

        if title.trim().is_empty() {
            continue;
        }

        let hash = hash_item(&link, &title);
        items.push(RawItem {
            title: title.trim().to_string(),
            url: link,
            content,
            published_at: None,
            hash,
        });
    }

    Ok(items)
}

fn resolve_url(base: &str, href: &str) -> String {
    Url::parse(base)
        .and_then(|b| b.join(href))
        .map(|u| u.to_string())
        .unwrap_or_else(|_| href.to_string())
}

fn hash_item(url: &str, title: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(url.as_bytes());
    hasher.update(title.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn http_client() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .user_agent("PersonalDashboard/0.1 (+local desktop widget)")
        .timeout(std::time::Duration::from_secs(15))
        .build()?)
}
