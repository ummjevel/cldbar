mod commands;
mod profile;
mod providers;

use commands::AppState;
use profile::load_config;
use providers::claude::ClaudeProvider;
use providers::claude_api::ClaudeApiProvider;
use providers::gemini::GeminiProvider;
use providers::zai::ZaiProvider;
use providers::Provider;
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::{
    tray::{MouseButton, MouseButtonState, TrayIconEvent},
    Listener, Manager,
};

pub fn run() {
    let config = load_config().unwrap_or_else(|_| profile::default_config());

    // Create providers from config
    let mut provider_map: HashMap<String, Box<dyn Provider>> = HashMap::new();
    for p in &config.profiles {
        if !p.enabled {
            continue;
        }
        let provider: Box<dyn Provider> = match (p.provider_type.as_str(), p.source_type.as_str()) {
            ("claude", "api") => {
                if let Some(ref key) = p.api_key {
                    Box::new(ClaudeApiProvider::new(key.clone()))
                } else {
                    continue;
                }
            }
            ("claude", _) => Box::new(ClaudeProvider::new(p.config_dir.clone().into())),
            ("gemini", _) => Box::new(GeminiProvider::new(p.config_dir.clone().into())),
            ("zai", _) => Box::new(ZaiProvider::new(p.config_dir.clone().into())),
            _ => continue,
        };
        provider_map.insert(p.id.clone(), provider);
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            config: Mutex::new(config),
            providers: Mutex::new(provider_map),
        })
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
        ])
        .setup(|app| {
            // Set up tray icon click handler.
            // In Tauri v2, a single trayIcon config entry gets the default tray handle.
            // Hide window when it loses focus (click outside)
            if let Some(window) = app.get_webview_window("main") {
                let w = window.clone();
                window.listen("tauri://blur", move |_| {
                    let _ = w.hide();
                });
            }

            if let Some(tray) = app.tray_by_id("main") {
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
                                // Position window near tray icon area.
                                // On Windows, the system tray is at the bottom-right.
                                if let Ok(Some(rect)) = tray.rect() {
                                    let pos: tauri::PhysicalPosition<i32> = rect.position.to_physical(1.0);
                                    let x = pos.x - 190;
                                    let y = pos.y - 550;
                                    let _ = window.set_position(tauri::Position::Physical(
                                        tauri::PhysicalPosition { x, y },
                                    ));
                                }
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
