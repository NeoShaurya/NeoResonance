use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};
use windows::{
    Foundation::TimeSpan,
    Media::Control::{
        GlobalSystemMediaTransportControlsSession,
        GlobalSystemMediaTransportControlsSessionManager,
        GlobalSystemMediaTransportControlsSessionPlaybackStatus,
    },
    Storage::Streams::{DataReader, InputStreamOptions},
    Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED},
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SmtcMediaState {
    pub active: bool,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub app_id: String,
    pub playback_status: String, // "Playing", "Paused", "Stopped", "Idle"
    pub position_ms: u64,
    pub duration_ms: u64,
    pub last_updated_unix_ms: u64,
    pub album_art: Option<String>, // "data:image/jpeg;base64,..."
}

impl Default for SmtcMediaState {
    fn default() -> Self {
        Self {
            active: false,
            title: "No Media Playing".into(),
            artist: "Idle".into(),
            album: "".into(),
            app_id: "".into(),
            playback_status: "Idle".into(),
            position_ms: 0,
            duration_ms: 0,
            last_updated_unix_ms: current_unix_ms(),
            album_art: None,
        }
    }
}

#[derive(Clone)]
pub struct SmtcState {
    pub manager: Arc<Mutex<Option<GlobalSystemMediaTransportControlsSessionManager>>>,
    pub cached_state: Arc<Mutex<SmtcMediaState>>,
}

fn current_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn status_to_string(status: GlobalSystemMediaTransportControlsSessionPlaybackStatus) -> String {
    match status {
        GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing => "Playing".into(),
        GlobalSystemMediaTransportControlsSessionPlaybackStatus::Paused => "Paused".into(),
        GlobalSystemMediaTransportControlsSessionPlaybackStatus::Stopped => "Stopped".into(),
        GlobalSystemMediaTransportControlsSessionPlaybackStatus::Changing => "Changing".into(),
        GlobalSystemMediaTransportControlsSessionPlaybackStatus::Opened => "Opened".into(),
        GlobalSystemMediaTransportControlsSessionPlaybackStatus::Closed => "Closed".into(),
        _ => "Idle".into(),
    }
}

fn extract_album_art(session: &GlobalSystemMediaTransportControlsSession) -> Option<String> {
    let media_props = session.TryGetMediaPropertiesAsync().ok()?.get().ok()?;
    let thumb_ref = media_props.Thumbnail().ok()?;
    let stream = thumb_ref.OpenReadAsync().ok()?.get().ok()?;
    let size = stream.Size().ok()? as u32;
    if size == 0 || size > 10_000_000 {
        return None;
    }

    let content_type = stream.ContentType().map(|s| s.to_string()).unwrap_or_default();
    let mime = if content_type.contains("png") {
        "image/png"
    } else if content_type.contains("webp") {
        "image/webp"
    } else {
        "image/jpeg"
    };

    let reader = DataReader::CreateDataReader(&stream).ok()?;
    reader.SetInputStreamOptions(InputStreamOptions::None).ok()?;
    reader.LoadAsync(size).ok()?.get().ok()?;

    let mut buffer = vec![0u8; size as usize];
    reader.ReadBytes(&mut buffer).ok()?;

    let base64_str = BASE64.encode(&buffer);
    Some(format!("data:{};base64,{}", mime, base64_str))
}

pub fn extract_media_state(
    session: &GlobalSystemMediaTransportControlsSession,
    cached_art: Option<String>,
    track_changed: bool,
) -> Option<SmtcMediaState> {
    let app_id = session
        .SourceAppUserModelId()
        .map(|s| s.to_string())
        .unwrap_or_default();

    let playback_info = session.GetPlaybackInfo().ok()?;
    let playback_status = status_to_string(playback_info.PlaybackStatus().ok()?);

    let media_props = session.TryGetMediaPropertiesAsync().ok()?.get().ok()?;
    let title = media_props.Title().map(|s| s.to_string()).unwrap_or_default();
    let artist = media_props.Artist().map(|s| s.to_string()).unwrap_or_default();
    let album = media_props
        .AlbumTitle()
        .map(|s| s.to_string())
        .unwrap_or_default();

    // Only extract album art over COM when track changed or not yet cached
    let album_art = if track_changed || cached_art.is_none() {
        extract_album_art(session)
    } else {
        cached_art
    };

    let timeline = session.GetTimelineProperties().ok();
    let (position_ms, duration_ms, last_updated_ms) = if let Some(t) = timeline {
        let pos = (t.Position().unwrap_or(TimeSpan { Duration: 0 }).Duration / 10_000).max(0) as u64;
        let dur = (t.EndTime().unwrap_or(TimeSpan { Duration: 0 }).Duration / 10_000).max(0) as u64;
        let upd = current_unix_ms();
        (pos, dur, upd)
    } else {
        (0, 0, current_unix_ms())
    };

    let active = !title.is_empty() || playback_status == "Playing" || playback_status == "Paused";

    Some(SmtcMediaState {
        active,
        title,
        artist,
        album,
        app_id,
        playback_status,
        position_ms,
        duration_ms,
        last_updated_unix_ms: last_updated_ms,
        album_art,
    })
}

pub fn setup_smtc(app: AppHandle) -> SmtcState {
    let manager_arc = Arc::new(Mutex::new(None));
    let cached_state = Arc::new(Mutex::new(SmtcMediaState::default()));

    let mgr_clone = Arc::clone(&manager_arc);
    let state_clone = Arc::clone(&cached_state);
    let app_handle = app.clone();

    std::thread::Builder::new()
        .name("smtc-polling-thread".into())
        .spawn(move || {
            unsafe {
                let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            }

            let mut last_title = String::new();
            let mut last_artist = String::new();
            let mut last_status = String::new();
            let mut cached_art: Option<String> = None;
            let mut last_emitted_pos = 0u64;
            let mut last_emit_time = current_unix_ms();

            loop {
                // Ensure session manager is initialized
                let mgr_opt = { mgr_clone.lock().unwrap().clone() };
                let manager = match mgr_opt {
                    Some(m) => m,
                    None => {
                        if let Ok(req) = GlobalSystemMediaTransportControlsSessionManager::RequestAsync() {
                            if let Ok(m) = req.get() {
                                *mgr_clone.lock().unwrap() = Some(m.clone());
                                m
                            } else {
                                std::thread::sleep(Duration::from_millis(500));
                                continue;
                            }
                        } else {
                            std::thread::sleep(Duration::from_millis(500));
                            continue;
                        }
                    }
                };

                // Poll current session
                if let Ok(session) = manager.GetCurrentSession() {
                    let pb_info = session.GetPlaybackInfo().ok();
                    let cur_status = pb_info
                        .and_then(|i| i.PlaybackStatus().ok())
                        .map(status_to_string)
                        .unwrap_or_else(|| "Idle".into());

                    let props = session.TryGetMediaPropertiesAsync().ok().and_then(|op| op.get().ok());
                    let cur_title = props.as_ref().and_then(|p| p.Title().ok()).map(|s| s.to_string()).unwrap_or_default();
                    let cur_artist = props.as_ref().and_then(|p| p.Artist().ok()).map(|s| s.to_string()).unwrap_or_default();

                    let track_changed = cur_title != last_title || cur_artist != last_artist;
                    let status_changed = cur_status != last_status;

                    if track_changed {
                        cached_art = None; // Reset cached thumbnail on track change
                    }

                    if let Some(new_state) = extract_media_state(&session, cached_art.clone(), track_changed) {
                        cached_art = new_state.album_art.clone();
                        last_title = new_state.title.clone();
                        last_artist = new_state.artist.clone();
                        last_status = new_state.playback_status.clone();

                        let now = current_unix_ms();
                        let time_since_emit = now.saturating_sub(last_emit_time);
                        let is_seek = (new_state.position_ms as i64 - (last_emitted_pos as i64 + time_since_emit as i64)).abs() > 1500;
                        let periodic_sync = new_state.playback_status == "Playing" && time_since_emit >= 2000;

                        if track_changed || status_changed || is_seek || periodic_sync {
                            last_emit_time = now;
                            last_emitted_pos = new_state.position_ms;
                            *state_clone.lock().unwrap() = new_state.clone();
                            let _ = app_handle.emit("smtc_state", new_state);
                        }
                    }
                } else {
                    // No current session
                    if !last_title.is_empty() {
                        last_title.clear();
                        last_artist.clear();
                        last_status = "Idle".into();
                        cached_art = None;
                        let default_state = SmtcMediaState::default();
                        *state_clone.lock().unwrap() = default_state.clone();
                        let _ = app_handle.emit("smtc_state", default_state);
                    }
                }

                std::thread::sleep(Duration::from_millis(400));
            }
        })
        .expect("failed to spawn smtc polling thread");

    SmtcState {
        manager: manager_arc,
        cached_state,
    }
}

#[tauri::command]
pub fn get_smtc_state(state: tauri::State<'_, SmtcState>) -> SmtcMediaState {
    state.cached_state.lock().unwrap().clone()
}

#[tauri::command]
pub async fn media_play_pause(state: tauri::State<'_, SmtcState>) -> Result<bool, String> {
    let mgr_opt = { state.manager.lock().unwrap().clone() };
    if let Some(manager) = mgr_opt {
        if let Ok(session) = manager.GetCurrentSession() {
            let _ = session.TryTogglePlayPauseAsync().map_err(|e| e.to_string())?.get();
            return Ok(true);
        }
    }
    Err("No active media session".into())
}

#[tauri::command]
pub async fn media_next(state: tauri::State<'_, SmtcState>) -> Result<bool, String> {
    let mgr_opt = { state.manager.lock().unwrap().clone() };
    if let Some(manager) = mgr_opt {
        if let Ok(session) = manager.GetCurrentSession() {
            let _ = session.TrySkipNextAsync().map_err(|e| e.to_string())?.get();
            return Ok(true);
        }
    }
    Err("No active media session".into())
}

#[tauri::command]
pub async fn media_prev(state: tauri::State<'_, SmtcState>) -> Result<bool, String> {
    let mgr_opt = { state.manager.lock().unwrap().clone() };
    if let Some(manager) = mgr_opt {
        if let Ok(session) = manager.GetCurrentSession() {
            let _ = session.TrySkipPreviousAsync().map_err(|e| e.to_string())?.get();
            return Ok(true);
        }
    }
    Err("No active media session".into())
}

#[tauri::command]
pub async fn media_seek(state: tauri::State<'_, SmtcState>, position_ms: u64) -> Result<bool, String> {
    let mgr_opt = { state.manager.lock().unwrap().clone() };
    if let Some(manager) = mgr_opt {
        if let Ok(session) = manager.GetCurrentSession() {
            let duration_100ns = (position_ms as i64) * 10_000;
            let _ = session
                .TryChangePlaybackPositionAsync(duration_100ns)
                .map_err(|e| e.to_string())?
                .get();
            return Ok(true);
        }
    }
    Err("No active media session".into())
}
