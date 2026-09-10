cd ~/Aspectus

cat > src-tauri/Cargo.toml << 'CARGOEOF'
[package]
name = "personal-dashboard"
version = "0.1.0"
edition = "2021"
default-run = "personal-dashboard"

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = ["tray-icon"] }
tauri-plugin-autostart = "2"
tauri-plugin-notification = "2"
tauri-plugin-shell = "2"
tauri-plugin-opener = "2"

serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["rt-multi-thread", "macros", "time"] }

reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }
feed-rs = "2"
scraper = "0.20"
url = "2"

rusqlite = { version = "0.32", features = ["bundled"] }
sha2 = "0.10"
base64 = "0.22"
rand = "0.8"
sysinfo = "0.32"
keyring = "3"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["v4"] }
anyhow = "1"
thiserror = "2"

[target.'cfg(target_os = "linux")'.dependencies]
mpris = "2"

[target.'cfg(target_os = "windows")'.dependencies]
windows = { version = "0.61", features = ["Media_Control", "Foundation"] }

[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
panic = "abort"
strip = true
CARGOEOF

cat > src-tauri/src/core/media.rs << 'MEDIAEOF'
use crate::models::NowPlaying;

#[cfg(target_os = "linux")]
mod linux_impl {
    use super::NowPlaying;
    use mpris::{Player, PlayerFinder};

    fn select_player(finder: &PlayerFinder, preferred: Option<&str>) -> Option<Player> {
        let players = finder.find_all().ok()?;
        if players.is_empty() {
            return None;
        }

        if let Some(name) = preferred {
            let name_lower = name.to_lowercase();
            if let Some(p) = players
                .into_iter()
                .find(|p| p.identity().to_lowercase().contains(&name_lower))
            {
                return Some(p);
            }
            let players = finder.find_all().ok()?;
            return players
                .into_iter()
                .find(|p| {
                    p.get_playback_status()
                        .map(|s| s == mpris::PlaybackStatus::Playing)
                        .unwrap_or(false)
                })
                .or_else(|| finder.find_all().ok().and_then(|p| p.into_iter().next()));
        }

        let players_for_search = finder.find_all().ok()?;
        players_for_search
            .into_iter()
            .find(|p| {
                p.get_playback_status()
                    .map(|s| s == mpris::PlaybackStatus::Playing)
                    .unwrap_or(false)
            })
            .or_else(|| players.into_iter().next())
    }

    pub fn list_players() -> Vec<String> {
        let Ok(finder) = PlayerFinder::new() else {
            return vec![];
        };
        finder
            .find_all()
            .map(|players| players.iter().map(|p| p.identity().to_string()).collect())
            .unwrap_or_default()
    }

    pub fn now_playing(preferred: Option<&str>) -> Option<NowPlaying> {
        let finder = PlayerFinder::new().ok()?;
        let player = select_player(&finder, preferred)?;

        let metadata = player.get_metadata().ok()?;
        let title = metadata.title().unwrap_or("Unbekannter Titel").to_string();
        let artist = metadata
            .artists()
            .map(|a| a.join(", "))
            .filter(|s| !s.is_empty());
        let is_playing = player
            .get_playback_status()
            .map(|s| s == mpris::PlaybackStatus::Playing)
            .unwrap_or(false);

        Some(NowPlaying {
            title,
            artist,
            source: player.identity().to_string(),
            is_playing,
        })
    }

    pub fn play_pause(preferred: Option<&str>) {
        if let Ok(finder) = PlayerFinder::new() {
            if let Some(player) = select_player(&finder, preferred) {
                let _ = player.play_pause();
            }
        }
    }

    pub fn next(preferred: Option<&str>) {
        if let Ok(finder) = PlayerFinder::new() {
            if let Some(player) = select_player(&finder, preferred) {
                let _ = player.next();
            }
        }
    }

    pub fn previous(preferred: Option<&str>) {
        if let Ok(finder) = PlayerFinder::new() {
            if let Some(player) = select_player(&finder, preferred) {
                let _ = player.previous();
            }
        }
    }

    pub fn get_volume(preferred: Option<&str>) -> Option<f64> {
        let finder = PlayerFinder::new().ok()?;
        let player = select_player(&finder, preferred)?;
        player.get_volume().ok()
    }

    pub fn set_volume(preferred: Option<&str>, level: f64) {
        if let Ok(finder) = PlayerFinder::new() {
            if let Some(player) = select_player(&finder, preferred) {
                let _ = player.set_volume(level.clamp(0.0, 1.0));
            }
        }
    }
}

#[cfg(target_os = "windows")]
mod windows_impl {
    use super::NowPlaying;
    use windows::Media::Control::{
        GlobalSystemMediaTransportControlsSession as Session,
        GlobalSystemMediaTransportControlsSessionManager as SessionManager,
        GlobalSystemMediaTransportControlsSessionPlaybackStatus as PlaybackStatus,
    };

    fn get_manager() -> Option<SessionManager> {
        SessionManager::RequestAsync().ok()?.get().ok()
    }

    fn all_sessions(manager: &SessionManager) -> Vec<Session> {
        let Ok(sessions) = manager.GetSessions() else {
            return vec![];
        };
        let Ok(size) = sessions.Size() else {
            return vec![];
        };
        (0..size).filter_map(|i| sessions.GetAt(i).ok()).collect()
    }

    fn is_playing(session: &Session) -> bool {
        session
            .GetPlaybackInfo()
            .and_then(|info| info.PlaybackStatus())
            .map(|status| status == PlaybackStatus::Playing)
            .unwrap_or(false)
    }

    fn select_session(manager: &SessionManager, preferred: Option<&str>) -> Option<Session> {
        let sessions = all_sessions(manager);
        if sessions.is_empty() {
            return None;
        }

        if let Some(name) = preferred {
            let name_lower = name.to_lowercase();
            if let Some(s) = sessions.iter().find(|s| {
                s.SourceAppUserModelId()
                    .map(|id| id.to_string_lossy().to_lowercase().contains(&name_lower))
                    .unwrap_or(false)
            }) {
                return Some(s.clone());
            }
        }

        sessions
            .iter()
            .find(|s| is_playing(s))
            .or_else(|| sessions.first())
            .cloned()
    }

    fn friendly_app_name(aumid: &str) -> String {
        aumid
            .split('!')
            .next()
            .unwrap_or(aumid)
            .trim_end_matches(".exe")
            .to_string()
    }

    pub fn list_players() -> Vec<String> {
        let Some(manager) = get_manager() else {
            return vec![];
        };
        all_sessions(&manager)
            .iter()
            .filter_map(|s| s.SourceAppUserModelId().ok())
            .map(|id| friendly_app_name(&id.to_string_lossy()))
            .collect()
    }

    pub fn now_playing(preferred: Option<&str>) -> Option<NowPlaying> {
        let manager = get_manager()?;
        let session = select_session(&manager, preferred)?;

        let props = session.TryGetMediaPropertiesAsync().ok()?.get().ok()?;
        let title = props.Title().ok()?.to_string_lossy();
        let artist = props
            .Artist()
            .ok()
            .map(|a| a.to_string_lossy())
            .filter(|s| !s.is_empty());
        let source = session
            .SourceAppUserModelId()
            .map(|id| friendly_app_name(&id.to_string_lossy()))
            .unwrap_or_else(|_| "Unbekannt".into());

        Some(NowPlaying {
            title,
            artist,
            source,
            is_playing: is_playing(&session),
        })
    }

    pub fn play_pause(preferred: Option<&str>) {
        if let Some(manager) = get_manager() {
            if let Some(session) = select_session(&manager, preferred) {
                let _ = session.TryTogglePlayPauseAsync().and_then(|op| op.get());
            }
        }
    }

    pub fn next(preferred: Option<&str>) {
        if let Some(manager) = get_manager() {
            if let Some(session) = select_session(&manager, preferred) {
                let _ = session.TrySkipNextAsync().and_then(|op| op.get());
            }
        }
    }

    pub fn previous(preferred: Option<&str>) {
        if let Some(manager) = get_manager() {
            if let Some(session) = select_session(&manager, preferred) {
                let _ = session.TrySkipPreviousAsync().and_then(|op| op.get());
            }
        }
    }

    pub fn get_volume(_preferred: Option<&str>) -> Option<f64> {
        None
    }
    pub fn set_volume(_preferred: Option<&str>, _level: f64) {}
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
mod fallback_impl {
    use super::NowPlaying;

    pub fn list_players() -> Vec<String> {
        vec![]
    }
    pub fn now_playing(_preferred: Option<&str>) -> Option<NowPlaying> {
        None
    }
    pub fn play_pause(_preferred: Option<&str>) {}
    pub fn next(_preferred: Option<&str>) {}
    pub fn previous(_preferred: Option<&str>) {}
    pub fn get_volume(_preferred: Option<&str>) -> Option<f64> {
        None
    }
    pub fn set_volume(_preferred: Option<&str>, _level: f64) {}
}

#[cfg(target_os = "linux")]
pub use linux_impl::*;

#[cfg(target_os = "windows")]
pub use windows_impl::*;

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub use fallback_impl::*;
MEDIAEOF

echo "Fertig — beide Dateien geschrieben."
