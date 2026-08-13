mod alerts;
mod commands;
mod profile;
mod providers;

use alerts::AlertState;
use commands::AppState;
use profile::load_config;
use providers::Provider;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconEvent},
    Manager,
};

/// Ask DWM to round the native window.
///
/// The webview clips its content to a rounded box, but the OS window behind it
/// is square, so its corners and the shadow it casts show past the rounded
/// content. Windows 11 can round the window itself; on older builds the call
/// simply fails and the clip-path stays the only rounding.
#[cfg(windows)]
fn round_window_corners(window: &tauri::WebviewWindow) {
    use std::ffi::c_void;

    #[link(name = "dwmapi")]
    extern "system" {
        fn DwmSetWindowAttribute(hwnd: isize, attr: u32, value: *const c_void, size: u32) -> i32;
    }

    const DWMWA_WINDOW_CORNER_PREFERENCE: u32 = 33;
    const DWMWCP_ROUND: u32 = 2;

    if let Ok(hwnd) = window.hwnd() {
        let preference = DWMWCP_ROUND;
        unsafe {
            DwmSetWindowAttribute(
                hwnd.0 as isize,
                DWMWA_WINDOW_CORNER_PREFERENCE,
                &preference as *const u32 as *const c_void,
                std::mem::size_of::<u32>() as u32,
            );
        }
    }
}

#[cfg(not(windows))]
fn round_window_corners(_window: &tauri::WebviewWindow) {}

/// Sit the popup just above the tray icon, clamped to the work area so it can
/// never land off-screen or behind the taskbar.
///
/// `tray.rect()` is not reported on every platform/shell, so the bottom-right
/// corner of the work area is used as the fallback anchor.
fn position_popup(window: &tauri::WebviewWindow, tray: &tauri::tray::TrayIcon) {
    let win_size = window
        .outer_size()
        .unwrap_or(tauri::PhysicalSize { width: 380, height: 490 });

    let tray_rect = tray.rect().ok().flatten();

    // Prefer the monitor the tray icon lives on, which is not necessarily the
    // one the hidden window was last placed on.
    let monitor = tray_rect
        .as_ref()
        .and_then(|r| {
            let p: tauri::PhysicalPosition<i32> = r.position.to_physical(1.0);
            window.app_handle().monitor_from_point(p.x as f64, p.y as f64).ok().flatten()
        })
        .or_else(|| window.current_monitor().ok().flatten())
        .or_else(|| window.primary_monitor().ok().flatten());

    let Some(monitor) = monitor else {
        return;
    };

    let scale = monitor.scale_factor();
    let area = monitor.work_area();
    let margin = (8.0 * scale).round() as i32;

    let (anchor_x, anchor_y) = match &tray_rect {
        Some(rect) => {
            let pos: tauri::PhysicalPosition<i32> = rect.position.to_physical(scale);
            let size: tauri::PhysicalSize<u32> = rect.size.to_physical(scale);
            (pos.x + size.width as i32 / 2, pos.y)
        }
        None => (
            area.position.x + area.size.width as i32,
            area.position.y + area.size.height as i32,
        ),
    };

    let min_x = area.position.x + margin;
    let min_y = area.position.y + margin;
    let max_x = (area.position.x + area.size.width as i32 - win_size.width as i32 - margin).max(min_x);
    let max_y = (area.position.y + area.size.height as i32 - win_size.height as i32 - margin).max(min_y);

    let x = (anchor_x - win_size.width as i32 / 2).clamp(min_x, max_x);
    let y = (anchor_y - win_size.height as i32 - margin).clamp(min_y, max_y);

    let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition { x, y }));
}

pub fn run() {
    let config = load_config().unwrap_or_else(|_| profile::default_config());

    // Create providers from config. A profile that cannot be built (unknown
    // type, missing API key) is skipped rather than failing startup.
    let mut provider_map: HashMap<String, Arc<dyn Provider>> = HashMap::new();
    for p in config.profiles.iter().filter(|p| p.enabled) {
        if let Ok(provider) = providers::create_provider(p) {
            provider_map.insert(p.id.clone(), provider);
        }
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            config: Mutex::new(config),
            providers: Mutex::new(provider_map),
        })
        .manage(AlertState::default())
        .invoke_handler(tauri::generate_handler![
            commands::get_profiles,
            commands::add_profile,
            commands::remove_profile,
            commands::get_usage_stats,
            commands::get_active_sessions,
            commands::get_daily_usage,
            commands::get_session_history,
            commands::get_settings,
            commands::update_settings,
            commands::get_all_usage_stats,
            commands::validate_api_key,
            commands::get_rate_limit_status,
            alerts::drain_alerts,
            alerts::resize_alert_window,
            alerts::hide_alert_window,
            alerts::show_main_window,
            alerts::send_test_alert,
        ])
        .setup(|app| {
            // Keep the toast overlay out of the way until an alert fires.
            if let Some(win) = app.get_webview_window(alerts::ALERT_WINDOW) {
                let _ = win.hide();
                round_window_corners(&win);
            }
            if let Some(win) = app.get_webview_window("main") {
                round_window_corners(&win);
            }
            alerts::spawn(app.handle().clone());

            // Set up tray icon with context menu and click handler.
            if let Some(tray) = app.tray_by_id("main") {
                let quit_item = MenuItemBuilder::with_id("quit", "Quit").build(app)?;
                let menu = MenuBuilder::new(app).item(&quit_item).build()?;
                tray.set_menu(Some(menu))?;

                tray.on_menu_event(|app, event| {
                    if event.id() == "quit" {
                        app.exit(0);
                    }
                });

                tray.on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            if window.is_visible().unwrap_or(false) {
                                let _ = window.hide();
                            } else {
                                position_popup(&window, tray);
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    }
                });
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running cldbar");
}
