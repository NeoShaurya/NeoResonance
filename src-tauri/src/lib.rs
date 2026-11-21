pub mod audio;
pub mod config;
pub mod lyrics;
pub mod smtc;
pub mod window;

use config::{load_config, ConfigState};
use lyrics::LyricsCache;
use smtc::SmtcState;
use std::sync::Mutex;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let initial_config = load_config(app.handle());
            let config_state = ConfigState(Mutex::new(initial_config.clone()));
            app.manage(config_state);

            let lyrics_cache = LyricsCache::default();
            app.manage(lyrics_cache);

            // Setup SMTC
            let smtc_manager = smtc::setup_smtc(app.handle().clone());
            app.manage(SmtcState(smtc_manager));

            // Setup desktop window
            if let Some(main_window) = app.get_webview_window("main") {
                window::setup_desktop_window(&main_window, &initial_config);
            }

            // Setup system tray
            let _ = window::setup_system_tray(app.handle());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            config::get_widget_config,
            config::save_widget_position,
            config::set_widget_locked,
            config::toggle_widget_locked,
            config::toggle_widget_aura,
            config::toggle_widget_lyrics,
            smtc::get_smtc_state,
            smtc::media_play_pause,
            smtc::media_next,
            smtc::media_prev,
            smtc::media_seek,
            audio::start_audio_stream,
            lyrics::get_lyrics,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
