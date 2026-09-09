use crate::core::{ai_engine, auth, settings, source_manager};
use crate::db::Db;
use crate::models::NewsCard;
use anyhow::Result;
use chrono::Utc;
use tauri::State;
use uuid::Uuid;

/// Runs the full pipeline for every configured source. Cheap when there is
/// nothing new: sources with no new items never reach the AI call at all.
pub async fn refresh_all(db: &State<'_, Db>) -> Result<Vec<NewsCard>> {
    let app_settings = settings::load(db)?;
    let sources = source_manager::list_sources(db)?;

    let mut all_cards = get_cached(db)?;

    for source in &sources {
        let new_items = match source_manager::fetch_new_items(db, source).await {
            Ok(items) => items,
            Err(e) => {
                mark_unhealthy(db, &source.id, &e.to_string())?;
                continue;
            }
        };

        if new_items.is_empty() {
            continue; // section 10: no new content -> no AI call at all
        }

        let candidates: Vec<_> = new_items
            .into_iter()
            .filter(|item| ai_engine::passes_local_prefilter(item, &app_settings.interests))
            .collect();

        if candidates.is_empty() {
            continue;
        }

        let Ok(token) = auth::get_token().await else {
            // Not signed in to an AI backend yet — surface raw items
            // untouched rather than silently dropping them, so the MVP is
            // still usable before auth (section 18) is wired up. These
            // MUST be persisted like any other card: the underlying items
            // are already marked "seen" by fetch_new_items above, so an
            // unpersisted card here would vanish forever on the next
            // refresh with no way to recover it once the user signs in.
            for item in &candidates {
                let card = NewsCard {
                    id: Uuid::new_v4().to_string(),
                    source_id: source.id.clone(),
                    source_title: source.title.clone(),
                    original_url: item.url.clone(),
                    headline: item.title.clone(),
                    summary: "KI-Anmeldung erforderlich für Zusammenfassungen.".into(),
                    category: source.category.clone(),
                    importance: 0.5,
                    published_at: item.published_at.clone(),
                    fetched_at: Utc::now().to_rfc3339(),
                };
                persist_card(db, &card)?;
                all_cards.push(card);
            }
            continue;
        };

        let evaluations = ai_engine::evaluate_batch(
            db,
            &token,
            &candidates,
            &app_settings.interests,
            &app_settings.info_filter,
        )
        .await?;

        let min_importance = match app_settings.info_filter.as_str() {
            "all" => 0.0,
            "relevant" => 0.3,
            "important" => 0.6,
            "critical" => 0.85,
            _ => 0.3,
        };

        for (item, eval) in candidates.iter().zip(evaluations.iter()) {
            if eval.is_duplicate_of.is_some() || eval.importance < min_importance {
                continue;
            }

            let card = NewsCard {
                id: Uuid::new_v4().to_string(),
                source_id: source.id.clone(),
                source_title: source.title.clone(),
                original_url: item.url.clone(),
                headline: eval.headline.clone(),
                summary: eval.summary.clone(),
                category: eval.category.clone(),
                importance: eval.importance,
                published_at: item.published_at.clone(),
                fetched_at: Utc::now().to_rfc3339(),
            };
            persist_card(db, &card)?;
            all_cards.push(card);
        }
    }

    Ok(all_cards)
}

pub fn get_cached(db: &State<Db>) -> Result<Vec<NewsCard>> {
    let conn = db.0.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, source_id, source_title, original_url, headline, summary, category, importance, published_at, fetched_at
         FROM news_cards ORDER BY fetched_at DESC LIMIT 100",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(NewsCard {
            id: r.get(0)?,
            source_id: r.get(1)?,
            source_title: r.get(2)?,
            original_url: r.get(3)?,
            headline: r.get(4)?,
            summary: r.get(5)?,
            category: r.get(6)?,
            importance: r.get(7)?,
            published_at: r.get(8)?,
            fetched_at: r.get(9)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

fn persist_card(db: &State<Db>, card: &NewsCard) -> Result<()> {
    let conn = db.0.lock().unwrap();
    conn.execute(
        "INSERT INTO news_cards (id, source_id, source_title, original_url, headline, summary, category, importance, published_at, fetched_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        rusqlite::params![
            card.id, card.source_id, card.source_title, card.original_url,
            card.headline, card.summary, card.category, card.importance,
            card.published_at, card.fetched_at
        ],
    )?;
    Ok(())
}

fn mark_unhealthy(db: &State<Db>, source_id: &str, error: &str) -> Result<()> {
    let conn = db.0.lock().unwrap();
    conn.execute(
        "UPDATE sources SET healthy = 0, last_error = ?1, last_checked_at = ?2 WHERE id = ?3",
        rusqlite::params![error, Utc::now().to_rfc3339(), source_id],
    )?;
    Ok(())
}
