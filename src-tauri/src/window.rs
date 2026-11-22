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
                // Default position: lower right desktop region, DPI-aware and taskbar-safe
                if let Ok(Some(monitor)) = window.current_monitor() {
                    let scale_factor = monitor.scale_factor();
                    let monitor_size = monitor.size();

                    let win_phys_w = (720.0 * scale_factor) as i32;
                    let win_phys_h = (310.0 * scale_factor) as i32;

                    let margin_right = (32.0 * scale_factor) as i32;
                    let margin_bottom = (70.0 * scale_factor) as i32; // clearance for taskbar + spacing

                    let default_x = (monitor_size.width as i32) - win_phys_w - margin_right;
                    let default_y = (monitor_size.height as i32) - win_phys_h - margin_bottom;

                    let _ = window.set_position(PhysicalPosition::new(default_x.max(20), default_y.max(20)));
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
    let reset_pos_item = MenuItemBuilder::with_id("reset_pos", "Reset Position (Center)").build(app)?;
    let aura_item = MenuItemBuilder::with_id("toggle_aura", "Toggle Aura / Wave").build(app)?;
    let lyrics_item = MenuItemBuilder::with_id("toggle_lyrics", "Toggle Synced Lyrics").build(app)?;
    let quit_item = MenuItemBuilder::with_id("quit", "Quit").build(app)?;

    let menu = MenuBuilder::new(app)
        .item(&lock_item)
        .item(&reset_pos_item)
        .separator()
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
                "reset_pos" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.center();
                        if let Ok(pos) = window.outer_position() {
                            let state = app.state::<ConfigState>();
                            let mut cfg = state.0.lock().unwrap();
                            cfg.x = Some(pos.x);
                            cfg.y = Some(pos.y);
                            cfg.locked = false;
                            save_config_to_disk(app, &cfg);
                            let _ = app.emit("config_changed", cfg.clone());
                        }
                    }
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

#[tauri::command]
pub fn start_window_drag(window: WebviewWindow) -> Result<(), String> {
    window.start_dragging().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn reset_widget_position(
    app: AppHandle,
    window: WebviewWindow,
    state: tauri::State<'_, ConfigState>,
) -> Result<(), String> {
    let _ = window.center();
    if let Ok(pos) = window.outer_position() {
        let mut cfg = state.0.lock().unwrap();
        cfg.x = Some(pos.x);
        cfg.y = Some(pos.y);
        cfg.locked = false;
        save_config_to_disk(&app, &cfg);
        let _ = app.emit("config_changed", cfg.clone());
    }
    Ok(())
}

