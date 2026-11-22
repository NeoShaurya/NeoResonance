use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};
use windows::{
    Foundation::{TimeSpan, TypedEventHandler},
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

pub struct SmtcState(pub Arc<Mutex<Option<GlobalSystemMediaTransportControlsSessionManager>>>);

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

pub fn extract_media_state(
    session: &GlobalSystemMediaTransportControlsSession,
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

    // Read album art thumbnail
    let album_art = (|| -> Option<String> {
        let thumb_ref = media_props.Thumbnail().ok()?;
        let stream = thumb_ref.OpenReadAsync().ok()?.get().ok()?;
        let size = stream.Size().ok()? as u32;
        if size == 0 {
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
    })();

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

pub fn fetch_current_state(
    manager_arc: &Arc<Mutex<Option<GlobalSystemMediaTransportControlsSessionManager>>>,
) -> SmtcMediaState {
    let lock = manager_arc.lock().unwrap();
    if let Some(ref manager) = *lock {
        if let Ok(session) = manager.GetCurrentSession() {
            return extract_media_state(&session).unwrap_or_default();
        }
    }
    SmtcMediaState::default()
}

pub fn update_and_emit(
    app: &AppHandle,
    manager_arc: &Arc<Mutex<Option<GlobalSystemMediaTransportControlsSessionManager>>>,
    last_state: &mut SmtcMediaState,
) {
    let new_state = fetch_current_state(manager_arc);
    // Emit if title, artist, playback_status, or track changed, or if position jumped significantly
    let changed = new_state.title != last_state.title
        || new_state.artist != last_state.artist
        || new_state.playback_status != last_state.playback_status
        || new_state.album_art != last_state.album_art
        || (new_state.position_ms as i64 - last_state.position_ms as i64).abs() > 3000;

    if changed {
        *last_state = new_state.clone();
        let _ = app.emit("smtc_state", new_state);
    }
}

pub fn setup_smtc(app: AppHandle) -> Arc<Mutex<Option<GlobalSystemMediaTransportControlsSessionManager>>> {
    let manager_arc = Arc::new(Mutex::new(None));
    let manager_arc_clone = Arc::clone(&manager_arc);

    let app_handle_clone = app.clone();
    std::thread::spawn(move || {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }

        let request_async = GlobalSystemMediaTransportControlsSessionManager::RequestAsync();
        if let Ok(async_op) = request_async {
            if let Ok(manager) = async_op.get() {
                let app_inner = app_handle_clone.clone();
                let mgr_inner = Arc::clone(&manager_arc_clone);

                // Register CurrentSessionChanged
                let handler = TypedEventHandler::new(move |_manager, _args| {
                    let mut dummy = SmtcMediaState::default();
                    update_and_emit(&app_inner, &mgr_inner, &mut dummy);
                    Ok(())
                });

                let _ = manager.CurrentSessionChanged(&handler);

                *manager_arc_clone.lock().unwrap() = Some(manager);

                // Background polling ticker loop (every 800ms) for rock-solid sync
                let mut last_emitted = SmtcMediaState::default();
                loop {
                    update_and_emit(&app_handle_clone, &manager_arc_clone, &mut last_emitted);
                    std::thread::sleep(Duration::from_millis(800));
                }
            }
        }
    });

    manager_arc
}

#[tauri::command]
pub fn get_smtc_state(state: tauri::State<'_, SmtcState>) -> SmtcMediaState {
    fetch_current_state(&state.0)
}

#[tauri::command]
pub fn media_play_pause(state: tauri::State<'_, SmtcState>, app: AppHandle) -> Result<bool, String> {
    let lock = state.0.lock().unwrap();
    if let Some(ref manager) = *lock {
        if let Ok(session) = manager.GetCurrentSession() {
            let _ = session.TryTogglePlayPauseAsync().map_err(|e| e.to_string())?.get();
            let mut dummy = SmtcMediaState::default();
            update_and_emit(&app, &state.0, &mut dummy);
            return Ok(true);
        }
    }
    Err("No active media session".into())
}

#[tauri::command]
pub fn media_next(state: tauri::State<'_, SmtcState>, app: AppHandle) -> Result<bool, String> {
    let lock = state.0.lock().unwrap();
    if let Some(ref manager) = *lock {
        if let Ok(session) = manager.GetCurrentSession() {
            let _ = session.TrySkipNextAsync().map_err(|e| e.to_string())?.get();
            let mut dummy = SmtcMediaState::default();
            update_and_emit(&app, &state.0, &mut dummy);
            return Ok(true);
        }
    }
    Err("No active media session".into())
}

#[tauri::command]
pub fn media_prev(state: tauri::State<'_, SmtcState>, app: AppHandle) -> Result<bool, String> {
    let lock = state.0.lock().unwrap();
    if let Some(ref manager) = *lock {
        if let Ok(session) = manager.GetCurrentSession() {
            let _ = session.TrySkipPreviousAsync().map_err(|e| e.to_string())?.get();
            let mut dummy = SmtcMediaState::default();
            update_and_emit(&app, &state.0, &mut dummy);
            return Ok(true);
        }
    }
    Err("No active media session".into())
}

#[tauri::command]
pub fn media_seek(state: tauri::State<'_, SmtcState>, app: AppHandle, position_ms: u64) -> Result<bool, String> {
    let lock = state.0.lock().unwrap();
    if let Some(ref manager) = *lock {
        if let Ok(session) = manager.GetCurrentSession() {
            let duration_100ns = (position_ms as i64) * 10_000;
            let _ = session
                .TryChangePlaybackPositionAsync(duration_100ns)
                .map_err(|e| e.to_string())?
                .get();
            let mut dummy = SmtcMediaState::default();
            update_and_emit(&app, &state.0, &mut dummy);
            return Ok(true);
        }
    }
    Err("No active media session".into())
}
