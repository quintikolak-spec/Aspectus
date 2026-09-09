use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperatingMode {
    #[serde(rename = "normal")]
    Normal,
    #[serde(rename = "sparsam")]
    Sparsam,
    #[serde(rename = "eco")]
    Eco,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interest {
    pub id: String,
    pub label: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WidgetLayout {
    pub id: String,
    pub kind: String,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    /// Process names the user has manually hidden from the system-monitor
    /// widget's "top processes" list. `#[serde(default)]` keeps older
    /// saved settings (from before this field existed) loading cleanly.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hidden_processes: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preferred_media_player: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub mode: OperatingMode,
    pub theme: String,
    pub corner_style: String,
    pub accent_color: String,
    pub interests: Vec<Interest>,
    pub info_filter: String,
    pub layout: Vec<WidgetLayout>,
    pub launch_on_startup: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    pub id: String,
    pub url: String,
    pub title: String,
    pub category: String,
    pub kind: String, // "rss" | "html"
    pub last_checked_at: Option<String>,
    pub last_updated_at: Option<String>,
    pub healthy: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewsCard {
    pub id: String,
    pub source_id: String,
    pub source_title: String,
    pub original_url: String,
    pub headline: String,
    pub summary: String,
    pub category: String,
    pub importance: f32,
    pub published_at: Option<String>,
    pub fetched_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessInfo {
    pub name: String,
    pub cpu_percent: f32,
    pub memory_mb: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemStats {
    pub cpu_percent: f32,
    pub ram_percent: f32,
    pub gpu_percent: Option<f32>,
    pub system_load_percent: f32,
    pub disk_free_gb: f32,
    pub battery_percent: Option<f32>,
    pub temps_c: std::collections::HashMap<String, f32>,
    pub top_processes: Vec<ProcessInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NowPlaying {
    pub title: String,
    pub artist: Option<String>,
    /// Player identity from MPRIS, e.g. "Brave", "Spotify" — shown so the
    /// user knows which app is actually playing.
    pub source: String,
    pub is_playing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForecastDay {
    pub day: String,
    pub high: f32,
    pub low: f32,
    pub condition: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeatherSnapshot {
    pub location_name: String,
    pub temp_c: f32,
    pub condition: String,
    pub humidity_percent: Option<f32>,
    pub wind_speed_kmh: Option<f32>,
    pub feels_like_c: Option<f32>,
    pub forecast: Vec<ForecastDay>,
}
