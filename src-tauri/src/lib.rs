pub mod audio;
pub mod config;
pub mod lyrics;
pub mod smtc;
pub mod window;

use config::{load_config, save_config_to_disk, ConfigState};
use lyrics::LyricsCache;
use std::sync::Mutex;
use tauri::Manager;

fn log(msg: &str) {
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("C:\\Users\\shaur\\NeoResonance\\startup.log")
    {
        let _ = writeln!(f, "{}", msg);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    log("run() called");
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            log("Inside setup");
            let initial_config = load_config(app.handle());
            let config_state = ConfigState(Mutex::new(initial_config.clone()));
            app.manage(config_state);
            log("Config managed");

            let lyrics_cache = LyricsCache::default();
            app.manage(lyrics_cache);
            log("Lyrics cache managed");

            // Setup SMTC
            log("Setting up SMTC");
            let smtc_state = smtc::setup_smtc(app.handle().clone());
            app.manage(smtc_state);
            log("SMTC managed");

            // Setup desktop window
            if let Some(main_window) = app.get_webview_window("main") {
                log("Setting up desktop window");
                window::setup_desktop_window(&main_window, &initial_config);
                log("Desktop window setup done");
            } else {
                log("Could not find main window!");
            }

            // Setup system tray
            log("Setting up system tray");
            if let Err(e) = window::setup_system_tray(app.handle()) {
                log(&format!("System tray setup error: {:?}", e));
            } else {
                log("System tray setup done");
            }

            log("Setup complete, returning Ok(())");
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Moved(pos) = event {
                let app = window.app_handle();
                if let Some(state) = app.try_state::<ConfigState>() {
                    let mut cfg = state.0.lock().unwrap();
                    if !cfg.locked {
                        cfg.x = Some(pos.x);
                        cfg.y = Some(pos.y);
                        save_config_to_disk(app, &cfg);
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            config::get_widget_config,
            config::save_widget_position,
            config::set_widget_locked,
            config::toggle_widget_locked,
            config::toggle_widget_aura,
            config::toggle_widget_lyrics,
            window::start_window_drag,
            window::reset_widget_position,
            smtc::get_smtc_state,
            smtc::media_play_pause,
            smtc::media_next,
            smtc::media_prev,
            smtc::media_seek,
            audio::start_audio_stream,
            lyrics::get_lyrics,
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            log(&format!("tauri run error: {:?}", e));
        });
}
