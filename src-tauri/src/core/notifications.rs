use crate::models::NewsCard;
use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

const NOTIFY_THRESHOLD: f32 = 0.85;

pub fn notify_if_important(app: &AppHandle, card: &NewsCard) {
    if card.importance < NOTIFY_THRESHOLD {
        return;
    }

    let _ = app
        .notification()
        .builder()
        .title(&card.headline)
        .body(&card.summary)
        .show();
}
