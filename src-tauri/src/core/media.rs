use crate::models::NowPlaying;

/// MPRIS (Media Player Remote Interfacing Specification) is the standard
/// Linux desktops use for "now playing" info and media-key control over
/// D-Bus. Chromium-based browsers (Brave, Chrome) register an MPRIS
/// interface automatically whenever a page uses the Media Session API and
/// something is playing -- which is exactly how sites like Deezer or
/// YouTube Music show up here, no site-specific integration needed.
///
/// Every function here takes an optional `preferred` player identity
/// (e.g. "Spotify", "Brave") so the user can pick which app to control
/// when more than one is active at once, instead of the app guessing.
#[cfg(target_os = "linux")]
mod linux_impl {
    use super::NowPlaying;
    use mpris::{Player, PlayerFinder};

    /// Picks the player matching `preferred` (case-insensitive substring
    /// match against its MPRIS identity) if given and found; otherwise
    /// falls back to whichever player is actively playing, or just the
    /// first one available.
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
            // Preferred player isn't currently active (e.g. Spotify closed) —
            // fall through to the default heuristic rather than showing nothing.
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

    /// Identities of every currently active MPRIS player, for a "which app
    /// should I control" picker in the UI.
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

    /// Same selection strategy as the Linux MPRIS implementation: prefer
    /// an explicit match on the app identity, otherwise whichever session
    /// is actually playing, otherwise just the first one.
    fn select_session(manager: &SessionManager, preferred: Option<&str>) -> Option<Session> {
        let sessions = all_sessions(manager);
        if sessions.is_empty() {
            return None;
        }

        if let Some(name) = preferred {
            let name_lower = name.to_lowercase();
            if let Some(s) = sessions.iter().find(|s| {
                s.SourceAppUserModelId()
                    .map(|id| id.to_string().to_lowercase().contains(&name_lower))
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

    /// `SourceAppUserModelId` is often a raw AUMID like
    /// "Spotify.exe" or a package-family id for UWP/browser apps --
    /// trim the noisy parts so the UI shows something readable.
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
            .map(|id| friendly_app_name(&id.to_string()))
            .collect()
    }

    pub fn now_playing(preferred: Option<&str>) -> Option<NowPlaying> {
        let manager = get_manager()?;
        let session = select_session(&manager, preferred)?;

        let props = session.TryGetMediaPropertiesAsync().ok()?.get().ok()?;
        let title = props.Title().ok()?.to_string();
        let artist = props
            .Artist()
            .ok()
            .map(|a| a.to_string())
            .filter(|s| !s.is_empty());
        let source = session
            .SourceAppUserModelId()
            .map(|id| friendly_app_name(&id.to_string()))
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

    // Windows' Media Session API (SMTC) has no volume control -- per-app
    // volume on Windows lives in a separate Core Audio API (the volume
    // mixer, via IAudioSessionManager2/ISimpleAudioVolume), which isn't
    // wired up here. The volume slider simply won't show on Windows,
    // matching the "no data" behaviour the widget already has.
    pub fn get_volume(_preferred: Option<&str>) -> Option<f64> {
        None
    }
    pub fn set_volume(_preferred: Option<&str>, _level: f64) {}
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
mod fallback_impl {
    use super::NowPlaying;

    // No media integration implemented for this platform yet.
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
