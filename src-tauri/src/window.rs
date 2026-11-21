use std::sync::atomic::{AtomicIsize, Ordering};
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, PhysicalPosition, WebviewWindow,
};
use windows::{
    core::w,
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        UI::WindowsAndMessaging::{
            CallWindowProcW, EnumWindows, FindWindowExW, FindWindowW, GetWindowLongPtrW,
            SendMessageTimeoutW, SetParent, SetWindowLongPtrW, SetWindowPos, GWLP_WNDPROC,
            GWL_EXSTYLE, HWND_BOTTOM, SC_MINIMIZE, SMTO_NORMAL, SWP_NOACTIVATE, SWP_NOMOVE,
            SWP_NOSIZE, WINDOWPOS, WM_SYSCOMMAND, WM_WINDOWPOSCHANGING, WNDPROC,
            WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
        },
    },
};

use crate::config::{save_config_to_disk, ConfigState, WidgetConfig};

static ORIGINAL_WNDPROC: AtomicIsize = AtomicIsize::new(0);

// Subclass window procedure to prevent Win+D minimization and keep at bottom
unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_SYSCOMMAND => {
            // Block minimize attempts (e.g. Win+D or show desktop signals)
            if (wparam.0 as u32 & 0xFFF0) == SC_MINIMIZE {
                return LRESULT(0);
            }
        }
        WM_WINDOWPOSCHANGING => {
            // Force bottom of Z-order
            let winpos = &mut *(lparam.0 as *mut WINDOWPOS);
            winpos.hwndInsertAfter = HWND_BOTTOM;
        }
        _ => {}
    }

    let orig = ORIGINAL_WNDPROC.load(Ordering::SeqCst);
    if orig != 0 {
        let orig_proc: WNDPROC = std::mem::transmute(orig);
        CallWindowProcW(orig_proc, hwnd, msg, wparam, lparam)
    } else {
        LRESULT(0)
    }
}

pub fn setup_desktop_window(window: &WebviewWindow, config: &WidgetConfig) {
    if let Ok(raw_hwnd) = window.hwnd() {
        let hwnd = HWND(raw_hwnd.0 as _);

        unsafe {
            // 1. Set extended styles: WS_EX_TOOLWINDOW (hides from Alt+Tab) & WS_EX_NOACTIVATE
            let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            let target_ex_style = ex_style
                | (WS_EX_TOOLWINDOW.0 as isize)
                | (WS_EX_NOACTIVATE.0 as isize);
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, target_ex_style);

            // 2. Desktop Attachment (survives Win+D)
            let mut _attached = false;
            let progman = FindWindowW(w!("Progman"), None);
            if let Ok(progman_hwnd) = progman {
                if !progman_hwnd.is_invalid() {
                    // Send 0x052C to Progman to spawn a WorkerW
                    let _ = SendMessageTimeoutW(
                        progman_hwnd,
                        0x052C,
                        WPARAM(0xD),
                        LPARAM(0x1),
                        SMTO_NORMAL,
                        1000,
                        None,
                    );

                    struct FindWorkerWData {
                        workerw: HWND,
                    }

                    unsafe extern "system" fn enum_windows_callback(
                        top_hwnd: HWND,
                        lparam: LPARAM,
                    ) -> windows::Win32::Foundation::BOOL {
                        let data = &mut *(lparam.0 as *mut FindWorkerWData);
                        let shelldll = FindWindowExW(top_hwnd, None, w!("SHELLDLL_DefView"), None);
                        if let Ok(shelldll_hwnd) = shelldll {
                            if !shelldll_hwnd.is_invalid() {
                                // Sibling workerw after this one is the wallpaper layer
                                let workerw = FindWindowExW(None, top_hwnd, w!("WorkerW"), None);
                                if let Ok(w_hwnd) = workerw {
                                    if !w_hwnd.is_invalid() {
                                        data.workerw = w_hwnd;
                                        return windows::Win32::Foundation::BOOL(0);
                                    }
                                }
                            }
                        }
                        windows::Win32::Foundation::BOOL(1)
                    }

                    let mut data = FindWorkerWData { workerw: HWND(std::ptr::null_mut()) };
                    let _ = EnumWindows(
                        Some(enum_windows_callback),
                        LPARAM(&mut data as *mut _ as isize),
                    );

                    if !data.workerw.is_invalid() {
                        let _ = SetParent(hwnd, data.workerw);
                        _attached = true;
                    }
                }
            }

            // 3. Fallback or additional security: Subclass WndProc to catch SC_MINIMIZE & keep HWND_BOTTOM
            let orig = SetWindowLongPtrW(hwnd, GWLP_WNDPROC, window_proc as *const () as isize);
            if orig != 0 {
                ORIGINAL_WNDPROC.store(orig, Ordering::SeqCst);
            }

            // 4. Position window
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

            // Anchor to bottom
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

    let _tray = TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .show_menu_on_left_click(false)
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
