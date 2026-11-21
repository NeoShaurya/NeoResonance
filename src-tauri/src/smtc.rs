use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};
use windows::{
    Foundation::{TimeSpan, TypedEventHandler},
    Media::Control::{
        GlobalSystemMediaTransportControlsSession,
        GlobalSystemMediaTransportControlsSessionManager,
        GlobalSystemMediaTransportControlsSessionPlaybackStatus,
    },
    Storage::Streams::{DataReader, InputStreamOptions},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
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

        let content_type = stream.ContentType().map(|s| s.to_string()).unwrap_or_else(|_| "image/jpeg".into());
        let reader = DataReader::CreateDataReader(&stream).ok()?;
        reader.SetInputStreamOptions(InputStreamOptions::None).ok()?;
        reader.LoadAsync(size).ok()?.get().ok()?;

        let mut buffer = vec![0u8; size as usize];
        reader.ReadBytes(&mut buffer).ok()?;

        let base64_str = BASE64.encode(&buffer);
        Some(format!("data:{};base64,{}", content_type, base64_str))
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

    let active = !title.is_empty() || playback_status == "Playing";

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

pub fn update_and_emit(
    app: &AppHandle,
    manager_arc: &Arc<Mutex<Option<GlobalSystemMediaTransportControlsSessionManager>>>,
) {
    let state = {
        let lock = manager_arc.lock().unwrap();
        if let Some(ref manager) = *lock {
            if let Ok(session) = manager.GetCurrentSession() {
                extract_media_state(&session).unwrap_or_default()
            } else {
                SmtcMediaState::default()
            }
        } else {
            SmtcMediaState::default()
        }
    };

    let _ = app.emit("smtc_state", state);
}

pub fn setup_smtc(app: AppHandle) -> Arc<Mutex<Option<GlobalSystemMediaTransportControlsSessionManager>>> {
    let manager_arc = Arc::new(Mutex::new(None));
    let manager_arc_clone = Arc::clone(&manager_arc);

    let app_handle_clone = app.clone();
    std::thread::spawn(move || {
        let request_async = GlobalSystemMediaTransportControlsSessionManager::RequestAsync();
        if let Ok(async_op) = request_async {
            if let Ok(manager) = async_op.get() {
                let app_inner = app_handle_clone.clone();
                let mgr_inner = Arc::clone(&manager_arc_clone);

                // Register CurrentSessionChanged
                let handler = TypedEventHandler::new(move |_manager, _args| {
                    update_and_emit(&app_inner, &mgr_inner);
                    Ok(())
                });

                let _ = manager.CurrentSessionChanged(&handler);

                // Attach events to current session if present
                if let Ok(session) = manager.GetCurrentSession() {
                    let app_for_props = app_handle_clone.clone();
                    let mgr_for_props = Arc::clone(&manager_arc_clone);
                    let _ = session.MediaPropertiesChanged(&TypedEventHandler::new(
                        move |_session, _args| {
                            update_and_emit(&app_for_props, &mgr_for_props);
                            Ok(())
                        },
                    ));

                    let app_for_pb = app_handle_clone.clone();
                    let mgr_for_pb = Arc::clone(&manager_arc_clone);
                    let _ = session.PlaybackInfoChanged(&TypedEventHandler::new(
                        move |_session, _args| {
                            update_and_emit(&app_for_pb, &mgr_for_pb);
                            Ok(())
                        },
                    ));

                    let app_for_tl = app_handle_clone.clone();
                    let mgr_for_tl = Arc::clone(&manager_arc_clone);
                    let _ = session.TimelinePropertiesChanged(&TypedEventHandler::new(
                        move |_session, _args| {
                            update_and_emit(&app_for_tl, &mgr_for_tl);
                            Ok(())
                        },
                    ));
                }

                *manager_arc_clone.lock().unwrap() = Some(manager);

                // Initial emission
                update_and_emit(&app_handle_clone, &manager_arc_clone);
            }
        }
    });

    manager_arc
}

#[tauri::command]
pub fn get_smtc_state(state: tauri::State<'_, SmtcState>) -> SmtcMediaState {
    let lock = state.0.lock().unwrap();
    if let Some(ref manager) = *lock {
        if let Ok(session) = manager.GetCurrentSession() {
            return extract_media_state(&session).unwrap_or_default();
        }
    }
    SmtcMediaState::default()
}

#[tauri::command]
pub fn media_play_pause(state: tauri::State<'_, SmtcState>) -> Result<bool, String> {
    let lock = state.0.lock().unwrap();
    if let Some(ref manager) = *lock {
        if let Ok(session) = manager.GetCurrentSession() {
            let _ = session.TryTogglePlayPauseAsync().map_err(|e| e.to_string())?.get();
            return Ok(true);
        }
    }
    Err("No active media session".into())
}

#[tauri::command]
pub fn media_next(state: tauri::State<'_, SmtcState>) -> Result<bool, String> {
    let lock = state.0.lock().unwrap();
    if let Some(ref manager) = *lock {
        if let Ok(session) = manager.GetCurrentSession() {
            let _ = session.TrySkipNextAsync().map_err(|e| e.to_string())?.get();
            return Ok(true);
        }
    }
    Err("No active media session".into())
}

#[tauri::command]
pub fn media_prev(state: tauri::State<'_, SmtcState>) -> Result<bool, String> {
    let lock = state.0.lock().unwrap();
    if let Some(ref manager) = *lock {
        if let Ok(session) = manager.GetCurrentSession() {
            let _ = session.TrySkipPreviousAsync().map_err(|e| e.to_string())?.get();
            return Ok(true);
        }
    }
    Err("No active media session".into())
}

#[tauri::command]
pub fn media_seek(state: tauri::State<'_, SmtcState>, position_ms: u64) -> Result<bool, String> {
    let lock = state.0.lock().unwrap();
    if let Some(ref manager) = *lock {
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
