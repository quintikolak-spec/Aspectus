use crate::core::system_monitor::SystemMonitor;
use crate::core::{auth, cache, media, news_engine, settings, source_manager};
use crate::db::Db;
use std::sync::Mutex;
use crate::models::{AppSettings, NewsCard, NowPlaying, Source, SystemStats, WeatherSnapshot};
use tauri::State;

#[tauri::command]
pub fn get_settings(db: State<Db>) -> Result<AppSettings, String> {
    settings::load(&db).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_settings(db: State<Db>, settings: AppSettings) -> Result<(), String> {
    settings::save(&db, &settings).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_system_stats(monitor: State<Mutex<SystemMonitor>>) -> SystemStats {
    // The SystemMonitor instance is kept alive for the app's lifetime in
    // Tauri-managed state (see main.rs) — a fresh sysinfo::System per call
    // would always report ~0% CPU, since usage is only meaningful as a
    // delta between two samples.
    monitor.lock().unwrap().snapshot()
}

#[tauri::command]
pub async fn get_weather(db: State<'_, Db>, lat: f64, lon: f64) -> Result<WeatherSnapshot, String> {
    let cache_key = format!("weather:{lat:.2}:{lon:.2}");

    if let Ok(Some(cached)) = cache::get(&db, &cache_key) {
        if let Ok(snapshot) = serde_json::from_str(&cached) {
            return Ok(snapshot);
        }
    }

    // Open-Meteo: free, no API key, matches the "no secrets in frontend"
    // requirement trivially since there's nothing to leak.
    let url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={lat}&longitude={lon}&current=temperature_2m,apparent_temperature,relative_humidity_2m,wind_speed_10m,weather_code&daily=temperature_2m_max,temperature_2m_min,weather_code&timezone=auto"
    );
    let resp = reqwest::get(&url).await.map_err(|e| e.to_string())?;
    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    let forecast = build_forecast(&json);
    let location_name = reverse_geocode(lat, lon).await.unwrap_or_else(|| "Aktueller Standort".into());

    let snapshot = WeatherSnapshot {
        location_name,
        temp_c: json["current"]["temperature_2m"].as_f64().unwrap_or(0.0) as f32,
        condition: weather_code_to_text(json["current"]["weather_code"].as_i64().unwrap_or(0)),
        humidity_percent: json["current"]["relative_humidity_2m"].as_f64().map(|v| v as f32),
        wind_speed_kmh: json["current"]["wind_speed_10m"].as_f64().map(|v| v as f32),
        feels_like_c: json["current"]["apparent_temperature"].as_f64().map(|v| v as f32),
        forecast,
    };

    let _ = cache::set(&db, &cache_key, &serde_json::to_string(&snapshot).unwrap(), Some(15 * 60));

    Ok(snapshot)
}

/// Maps Open-Meteo's `daily` arrays (parallel arrays indexed by day) into
/// our `ForecastDay` list. Skips today (index 0) since the "current"
/// block already covers it, matching section 4's "Tages-/Mehrtageseinsicht".
fn build_forecast(json: &serde_json::Value) -> Vec<crate::models::ForecastDay> {
    let times = json["daily"]["time"].as_array();
    let highs = json["daily"]["temperature_2m_max"].as_array();
    let lows = json["daily"]["temperature_2m_min"].as_array();
    let codes = json["daily"]["weather_code"].as_array();

    let (Some(times), Some(highs), Some(lows), Some(codes)) = (times, highs, lows, codes) else {
        return vec![];
    };

    times
        .iter()
        .zip(highs.iter())
        .zip(lows.iter())
        .zip(codes.iter())
        .skip(1) // today is already shown as "current"
        .take(4)
        .filter_map(|(((t, h), l), c)| {
            Some(crate::models::ForecastDay {
                day: t.as_str()?.to_string(),
                high: h.as_f64()? as f32,
                low: l.as_f64()? as f32,
                condition: weather_code_to_text(c.as_i64().unwrap_or(0)),
            })
        })
        .collect()
}

/// Reverse geocoding via Nominatim (OpenStreetMap) — free, no API key.
/// Their usage policy requires a descriptive User-Agent and reasonable
/// request volume; the 15-minute cache on the caller side already keeps
/// this well within that.
async fn reverse_geocode(lat: f64, lon: f64) -> Option<String> {
    let url = format!(
        "https://nominatim.openstreetmap.org/reverse?lat={lat}&lon={lon}&format=json&zoom=10"
    );
    let client = reqwest::Client::builder()
        .user_agent("PersonalDashboard/0.1 (local desktop widget)")
        .build()
        .ok()?;
    let json: serde_json::Value = client.get(&url).send().await.ok()?.json().await.ok()?;
    let addr = &json["address"];
    addr.get("city")
        .or_else(|| addr.get("town"))
        .or_else(|| addr.get("village"))
        .or_else(|| addr.get("municipality"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn weather_code_to_text(code: i64) -> String {
    match code {
        0 => "Klar".into(),
        1..=3 => "Bewölkt".into(),
        45 | 48 => "Nebel".into(),
        51..=67 => "Regen".into(),
        71..=77 => "Schnee".into(),
        95..=99 => "Gewitter".into(),
        _ => "Unbekannt".into(),
    }
}

#[tauri::command]
pub async fn add_source(db: State<'_, Db>, url: String, category: String) -> Result<Source, String> {
    source_manager::add_source(&db, &url, &category)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_sources(db: State<Db>) -> Result<Vec<Source>, String> {
    source_manager::list_sources(&db).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remove_source(db: State<Db>, id: String) -> Result<(), String> {
    source_manager::remove_source(&db, &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn refresh_source(db: State<'_, Db>, id: String) -> Result<Vec<NewsCard>, String> {
    // Manual "check now" for one source: reuses the same pipeline as the
    // background loop, just scoped down. For the MVP we simply trigger a
    // full refresh and let the caller filter by source_id client-side.
    let _ = id;
    news_engine::refresh_all(&db).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_cached_news(db: State<Db>) -> Result<Vec<NewsCard>, String> {
    news_engine::get_cached(&db).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn refresh_all_sources(db: State<'_, Db>) -> Result<Vec<NewsCard>, String> {
    news_engine::refresh_all(&db).await.map_err(|e| e.to_string())
}

/// Practical, always-available sign-in: the user pastes an API key from
/// platform.openai.com/api-keys. Stored only in the OS keychain.
#[tauri::command]
pub fn openai_sign_in_with_api_key(key: String) -> Result<(), String> {
    auth::sign_in_with_api_key(&key).map_err(|e| e.to_string())
}

/// "Sign in with ChatGPT": opens the system browser for the OAuth/PKCE
/// flow. Requires this app to have an OpenAI-approved client_id (see
/// core::auth) -- until then this command returns a clear error rather
/// than silently falling back, so the frontend can point the user at the
/// API-key path instead.
#[tauri::command]
pub async fn openai_sign_in_oauth(app: tauri::AppHandle) -> Result<(), String> {
    auth::sign_in_oauth(&app).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub fn openai_sign_out() -> Result<(), String> {
    auth::sign_out().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn openai_is_signed_in() -> bool {
    auth::is_signed_in()
}

#[tauri::command]
pub fn get_now_playing(preferred: Option<String>) -> Option<NowPlaying> {
    media::now_playing(preferred.as_deref())
}

#[tauri::command]
pub fn media_play_pause(preferred: Option<String>) {
    media::play_pause(preferred.as_deref());
}

#[tauri::command]
pub fn media_next(preferred: Option<String>) {
    media::next(preferred.as_deref());
}

#[tauri::command]
pub fn media_previous(preferred: Option<String>) {
    media::previous(preferred.as_deref());
}

#[tauri::command]
pub fn media_get_volume(preferred: Option<String>) -> Option<f64> {
    media::get_volume(preferred.as_deref())
}

#[tauri::command]
pub fn media_set_volume(preferred: Option<String>, level: f64) {
    media::set_volume(preferred.as_deref(), level);
}

#[tauri::command]
pub fn media_list_players() -> Vec<String> {
    media::list_players()
}
