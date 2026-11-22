use tauri::{
    image::Image,
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, PhysicalPosition, WebviewWindow,
};
use windows::Win32::{
    Foundation::HWND,
    UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, GWL_EXSTYLE, HWND_BOTTOM,
        SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, WS_EX_TOOLWINDOW,
    },
};

use crate::config::{save_config_to_disk, ConfigState, WidgetConfig};

pub fn setup_desktop_window(window: &WebviewWindow, config: &WidgetConfig) {
    if let Ok(raw_hwnd) = window.hwnd() {
        let hwnd = HWND(raw_hwnd.0 as _);

        unsafe {
            // 1. Set extended styles: WS_EX_TOOLWINDOW (hides from Alt+Tab)
            let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            let target_ex_style = ex_style | (WS_EX_TOOLWINDOW.0 as isize);
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, target_ex_style);

            // 2. Position window
            if let (Some(x), Some(y)) = (config.x, config.y) {
                let _ = window.set_position(PhysicalPosition::new(x, y));
            } else {
                // Default position: lower right desktop region
                if let Ok(Some(monitor)) = window.current_monitor() {
                    let size = monitor.size();
                    let default_x = (size.width as i32) - 760;
                    let default_y = (size.height as i32) - 480;
                    let _ = window.set_position(PhysicalPosition::new(default_x, default_y));
                }
            }

            // 3. Keep behind normal application windows (desktop overlay layer)
            let _ = SetWindowPos(
                hwnd,
                HWND_BOTTOM,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
        }
    }
}

pub fn setup_system_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let lock_item = MenuItemBuilder::with_id("toggle_lock", "Lock / Unlock Position").build(app)?;
    let aura_item = MenuItemBuilder::with_id("toggle_aura", "Toggle Aura / Wave").build(app)?;
    let lyrics_item = MenuItemBuilder::with_id("toggle_lyrics", "Toggle Synced Lyrics").build(app)?;
    let quit_item = MenuItemBuilder::with_id("quit", "Quit").build(app)?;

    let menu = MenuBuilder::new(app)
        .item(&lock_item)
        .item(&aura_item)
        .item(&lyrics_item)
        .separator()
        .item(&quit_item)
        .build()?;

    let mut tray_builder = TrayIconBuilder::new()
        .menu(&menu)
        .show_menu_on_left_click(false);

    if let Some(icon) = app.default_window_icon() {
        tray_builder = tray_builder.icon(icon.clone());
    } else {
        let mut pixels = Vec::with_capacity(16 * 16 * 4);
        for _ in 0..(16 * 16) {
            pixels.extend_from_slice(&[6, 182, 212, 255]); // #06b6d4 cyan
        }
        tray_builder = tray_builder.icon(Image::new_owned(pixels, 16, 16));
    }

    let _tray = tray_builder
        .on_menu_event(move |app, event| {
            let id = event.id().as_ref();
            match id {
                "toggle_lock" => {
                    let state = app.state::<ConfigState>();
                    let mut cfg = state.0.lock().unwrap();
                    cfg.locked = !cfg.locked;
                    save_config_to_disk(app, &cfg);
                    let _ = app.emit("config_changed", cfg.clone());
                }
                "toggle_aura" => {
                    let state = app.state::<ConfigState>();
                    let mut cfg = state.0.lock().unwrap();
                    cfg.show_aura = !cfg.show_aura;
                    save_config_to_disk(app, &cfg);
                    let _ = app.emit("config_changed", cfg.clone());
                }
                "toggle_lyrics" => {
                    let state = app.state::<ConfigState>();
                    let mut cfg = state.0.lock().unwrap();
                    cfg.show_lyrics = !cfg.show_lyrics;
                    save_config_to_disk(app, &cfg);
                    let _ = app.emit("config_changed", cfg.clone());
                }
                "quit" => {
                    app.exit(0);
                }
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app)?;

    Ok(())
}
