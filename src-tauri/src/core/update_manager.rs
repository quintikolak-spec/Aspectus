use crate::core::{news_engine, notifications, settings};
use crate::db::Db;
use crate::models::OperatingMode;
use tauri::{AppHandle, Emitter, Manager};

fn news_interval_secs(mode: &OperatingMode) -> u64 {
    match mode {
        OperatingMode::Normal => 10 * 60,
        OperatingMode::Sparsam => 30 * 60,
        // Eco mode deliberately has no background timer at all — refresh
        // only happens when the dashboard is opened (see refresh_all_sources
        // command, called from the frontend on mount). We still spawn the
        // loop but with a very long sleep as a safety-net "at least once a
        // day" refresh rather than a true poll.
        OperatingMode::Eco => 24 * 60 * 60,
    }
}

/// Spawned once from `main.rs` setup. Runs for the lifetime of the app,
/// re-reading settings each cycle so a mode change takes effect on the next
/// tick without a restart.
pub fn spawn(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            let db = app.state::<Db>();
            let mode = settings::load(&db)
                .map(|s| s.mode)
                .unwrap_or(OperatingMode::Normal);

            let wait = news_interval_secs(&mode);

            match news_engine::refresh_all(&db).await {
                Ok(cards) => {
                    let _ = app.emit("news-updated", &cards);
                    if let Some(card) = cards.iter().max_by(|a, b| {
                        a.importance.partial_cmp(&b.importance).unwrap()
                    }) {
                        notifications::notify_if_important(&app, card);
                    }
                }
                Err(e) => eprintln!("background news refresh failed: {e}"),
            }

            tokio::time::sleep(std::time::Duration::from_secs(wait)).await;
        }
    });
}
