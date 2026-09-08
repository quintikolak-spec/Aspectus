use crate::db::Db;
use crate::models::{AppSettings, Interest, OperatingMode, WidgetLayout};
use anyhow::Result;
use tauri::State;

pub fn default_settings() -> AppSettings {
    AppSettings {
        mode: OperatingMode::Normal,
        theme: "dark".into(),
        corner_style: "soft".into(),
        accent_color: "#e8a33d".into(),
        interests: vec![
            "Gaming", "Minecraft", "Linux", "Hardware", "KI", "FPV", "IT",
        ]
        .into_iter()
        .map(|label| Interest {
            id: label.to_lowercase(),
            label: label.into(),
            enabled: true,
        })
        .collect(),
        info_filter: "relevant".into(),
        layout: vec![
            WidgetLayout { id: "clock-1".into(), kind: "clock".into(), x: 0, y: 0, w: 2, h: 1, source_id: None, hidden_processes: None },
            WidgetLayout { id: "weather-1".into(), kind: "weather".into(), x: 2, y: 0, w: 2, h: 1, source_id: None, hidden_processes: None },
            WidgetLayout { id: "news-1".into(), kind: "news".into(), x: 0, y: 1, w: 4, h: 2, source_id: None, hidden_processes: None },
            WidgetLayout { id: "system-1".into(), kind: "system".into(), x: 0, y: 3, w: 4, h: 1, source_id: None, hidden_processes: None },
        ],
        launch_on_startup: true,
    }
}

pub fn load(db: &State<Db>) -> Result<AppSettings> {
    let conn = db.0.lock().unwrap();
    let json: Option<String> = conn
        .query_row("SELECT json FROM settings WHERE id = 1", [], |r| r.get(0))
        .ok();

    match json {
        Some(j) => Ok(serde_json::from_str(&j)?),
        None => Ok(default_settings()),
    }
}

pub fn save(db: &State<Db>, settings: &AppSettings) -> Result<()> {
    let json = serde_json::to_string(settings)?;
    let conn = db.0.lock().unwrap();
    conn.execute(
        "INSERT INTO settings (id, json) VALUES (1, ?1)
         ON CONFLICT(id) DO UPDATE SET json = excluded.json",
        [json],
    )?;
    Ok(())
}
