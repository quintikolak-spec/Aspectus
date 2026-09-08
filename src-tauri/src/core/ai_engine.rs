use crate::core::cache;
use crate::core::source_manager::RawItem;
use crate::db::Db;
use crate::models::Interest;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use tauri::State;

// Current flagship model per OpenAI's own docs examples at the time this
// was wired up. Model names change; if calls start failing with an
// "unknown model" error, check https://developers.openai.com/api/docs/models
// for the current default and update this constant.
const MODEL: &str = "gpt-6-astra";

const CONVERSATION_CACHE_KEY: &str = "openai_conversation_id";

pub struct Evaluation {
    pub headline: String,
    pub summary: String,
    pub category: String,
    pub importance: f32, // 0.0..1.0
    pub is_duplicate_of: Option<String>,
}

/// Cheap, local, non-AI relevance gate (section 13: don't call the AI for
/// content that's obviously off-interest). Only items that pass this go on
/// to `evaluate_batch`. This is a heuristic, not a replacement for the AI's
/// own relevance judgement -- it just avoids paying for calls on items that
/// have zero keyword overlap with anything the user cares about.
pub fn passes_local_prefilter(item: &RawItem, interests: &[Interest]) -> bool {
    let enabled: Vec<&str> = interests
        .iter()
        .filter(|i| i.enabled)
        .map(|i| i.label.as_str())
        .collect();

    if enabled.is_empty() {
        return true; // no filter configured -- let the AI decide everything
    }

    let haystack = format!("{} {}", item.title, item.content).to_lowercase();
    enabled.iter().any(|kw| haystack.contains(&kw.to_lowercase()))
}

/// Returns the persistent OpenAI Conversation object id for this
/// installation, creating one on first use. This is section 19's
/// "persistente Projekt-/Kontextstruktur": rather than starting a fresh
/// stateless call every refresh, every batch evaluation is attached to the
/// same conversation via the Conversations API, so the model can build up
/// context about this user's interests and prior categorizations across
/// restarts. Conversation items have no 30-day TTL (unlike bare Responses),
/// which matches "dauerhaft laufen" from section 21.
async fn get_or_create_conversation_id(db: &State<'_, Db>, token: &str) -> Result<String> {
    if let Some(id) = cache::get(db, CONVERSATION_CACHE_KEY)? {
        return Ok(id);
    }

    #[derive(Deserialize)]
    struct ConversationResp {
        id: String,
    }

    let client = reqwest::Client::new();
    let resp: ConversationResp = client
        .post("https://api.openai.com/v1/conversations")
        .bearer_auth(token)
        .json(&serde_json::json!({
            "metadata": { "app": "personal-dashboard" }
        }))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    // No TTL: this id should survive for the lifetime of the install.
    cache::set(db, CONVERSATION_CACHE_KEY, &resp.id, None)?;
    Ok(resp.id)
}

/// Batches multiple new items into a single Responses API call where
/// possible (section 13: "mehrere neue Artikel möglichst effizient
/// gemeinsam verarbeiten"), asking for relevance, dedup, category and a
/// short summary per item via a strict JSON schema (Structured Outputs),
/// attached to the persistent conversation from `get_or_create_conversation_id`.
pub async fn evaluate_batch(
    db: &State<'_, Db>,
    token: &str,
    items: &[RawItem],
    interests: &[Interest],
    info_filter: &str,
) -> Result<Vec<Evaluation>> {
    if items.is_empty() {
        return Ok(vec![]);
    }

    let conversation_id = get_or_create_conversation_id(db, token).await?;

    let interest_list = interests
        .iter()
        .filter(|i| i.enabled)
        .map(|i| i.label.clone())
        .collect::<Vec<_>>()
        .join(", ");

    let items_payload: Vec<_> = items
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            serde_json::json!({
                "index": idx,
                "title": item.title,
                "excerpt": item.content.chars().take(1500).collect::<String>(),
            })
        })
        .collect();

    let instructions = format!(
        "Du bewertest neue Artikel fuer ein persoenliches Dashboard. \
         Interessen des Nutzers: {interest_list}. \
         Gewuenschte Filterstufe: {info_filter} (all/relevant/important/critical). \
         Nutze den bisherigen Konversationsverlauf, um Wiederholungen und bereits \
         bekannte Themen zu erkennen."
    );

    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "results": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "index": { "type": "integer" },
                        "headline": { "type": "string" },
                        "summary": { "type": "string" },
                        "category": { "type": "string" },
                        "importance": { "type": "number" },
                        "is_duplicate_of": { "type": ["integer", "null"] }
                    },
                    "required": ["index", "headline", "summary", "category", "importance", "is_duplicate_of"],
                    "additionalProperties": false
                }
            }
        },
        "required": ["results"],
        "additionalProperties": false
    });

    let body = serde_json::json!({
        "model": MODEL,
        "conversation": conversation_id,
        "instructions": instructions,
        "input": serde_json::to_string(&items_payload)?,
        "text": {
            "format": {
                "type": "json_schema",
                "name": "news_evaluation",
                "schema": schema,
                "strict": true
            }
        }
    });

    let client = reqwest::Client::new();
    let resp = client
        .post("https://api.openai.com/v1/responses")
        .bearer_auth(token)
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(anyhow!("OpenAI Responses API error ({status}): {text}"));
    }

    let json: serde_json::Value = resp.json().await?;
    let output_text = extract_output_text(&json)
        .ok_or_else(|| anyhow!("no text output in Responses API result"))?;

    let parsed: BatchResult = serde_json::from_str(&output_text)?;

    Ok(parsed
        .results
        .into_iter()
        .map(|r| Evaluation {
            headline: r.headline,
            summary: r.summary,
            category: r.category,
            importance: r.importance.clamp(0.0, 1.0),
            is_duplicate_of: r.is_duplicate_of.map(|i| i.to_string()),
        })
        .collect())
}

/// The Responses API returns a polymorphic `output` array (messages, tool
/// calls, reasoning items, ...). We only care about the first output_text
/// content block from a message item.
fn extract_output_text(response: &serde_json::Value) -> Option<String> {
    // Convenience field some SDKs surface directly; fall back to walking
    // `output` manually if it's absent from the raw JSON.
    if let Some(text) = response.get("output_text").and_then(|v| v.as_str()) {
        return Some(text.to_string());
    }

    response.get("output")?.as_array()?.iter().find_map(|item| {
        item.get("content")?
            .as_array()?
            .iter()
            .find_map(|c| c.get("text").and_then(|t| t.as_str()).map(String::from))
    })
}

#[derive(Deserialize, Serialize, Default)]
struct BatchResult {
    results: Vec<AiItemResult>,
}

#[derive(Deserialize, Serialize, Default)]
struct AiItemResult {
    index: usize,
    headline: String,
    summary: String,
    category: String,
    importance: f32,
    is_duplicate_of: Option<usize>,
}
