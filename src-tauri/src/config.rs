use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetConfig {
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub locked: bool,
    pub show_aura: bool,
    pub show_lyrics: bool,
}

impl Default for WidgetConfig {
    fn default() -> Self {
        Self {
            x: None,
            y: None,
            locked: true,
            show_aura: true,
            show_lyrics: true,
        }
    }
}

pub struct ConfigState(pub Mutex<WidgetConfig>);

fn get_config_path(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .app_config_dir()
        .ok()
        .map(|dir| dir.join("config.json"))
}

pub fn load_config(app: &AppHandle) -> WidgetConfig {
    if let Some(path) = get_config_path(app) {
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(cfg) = serde_json::from_str::<WidgetConfig>(&content) {
                    return cfg;
                }
            }
        }
    }
    WidgetConfig::default()
}

pub fn save_config_to_disk(app: &AppHandle, config: &WidgetConfig) {
    if let Some(path) = get_config_path(app) {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(config) {
            let _ = fs::write(path, json);
        }
    }
}

#[tauri::command]
pub fn get_widget_config(state: tauri::State<'_, ConfigState>) -> WidgetConfig {
    state.0.lock().unwrap().clone()
}

#[tauri::command]
pub fn save_widget_position(
    app: AppHandle,
    state: tauri::State<'_, ConfigState>,
    x: i32,
    y: i32,
) -> Result<(), String> {
    let mut cfg = state.0.lock().unwrap();
    cfg.x = Some(x);
    cfg.y = Some(y);
    save_config_to_disk(&app, &cfg);
    Ok(())
}

#[tauri::command]
pub fn set_widget_locked(
    app: AppHandle,
    state: tauri::State<'_, ConfigState>,
    locked: bool,
) -> Result<bool, String> {
    let mut cfg = state.0.lock().unwrap();
    cfg.locked = locked;
    save_config_to_disk(&app, &cfg);
    let _ = app.emit("config_changed", cfg.clone());
    Ok(cfg.locked)
}

#[tauri::command]
pub fn toggle_widget_locked(
    app: AppHandle,
    state: tauri::State<'_, ConfigState>,
) -> Result<bool, String> {
    let mut cfg = state.0.lock().unwrap();
    cfg.locked = !cfg.locked;
    save_config_to_disk(&app, &cfg);
    let _ = app.emit("config_changed", cfg.clone());
    Ok(cfg.locked)
}

#[tauri::command]
pub fn toggle_widget_aura(
    app: AppHandle,
    state: tauri::State<'_, ConfigState>,
) -> Result<bool, String> {
    let mut cfg = state.0.lock().unwrap();
    cfg.show_aura = !cfg.show_aura;
    save_config_to_disk(&app, &cfg);
    let _ = app.emit("config_changed", cfg.clone());
    Ok(cfg.show_aura)
}

#[tauri::command]
pub fn toggle_widget_lyrics(
    app: AppHandle,
    state: tauri::State<'_, ConfigState>,
) -> Result<bool, String> {
    let mut cfg = state.0.lock().unwrap();
    cfg.show_lyrics = !cfg.show_lyrics;
    save_config_to_disk(&app, &cfg);
    let _ = app.emit("config_changed", cfg.clone());
    Ok(cfg.show_lyrics)
}
