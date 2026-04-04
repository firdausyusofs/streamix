use tauri::{AppHandle, Manager};
use tauri::webview::Color;

pub mod mpv;
pub mod stremio;

#[tauri::command]
fn set_window_background(transparent: bool, app: AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or("main window not found")?;
    let color = if transparent {
        Color(0, 0, 0, 0)
    } else {
        Color(7, 5, 17, 255)
    };
    window.set_background_color(Some(color)).map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(mpv::commands::MpvHandle(std::sync::Mutex::new(None)))
        .setup(|app| {
            // Make window opaque by default; transparency is only enabled during mpv playback
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_background_color(Some(Color(7, 5, 17, 255)));
            }
            tauri::async_runtime::spawn(async move {
                crate::stremio::server::start_server().await;
            });
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            // stremio commands
            stremio::commands::get_installed_addons,
            stremio::commands::fetch_catalog_from_addon,
            stremio::commands::fetch_streams_from_addon,
            stremio::commands::play_stream_command,
            stremio::commands::play_stream_for_mpv,
            stremio::commands::stop_stream_command,
            // mpv commands
            mpv::commands::mpv_play,
            mpv::commands::mpv_stop,
            mpv::commands::mpv_toggle_pause,
            mpv::commands::mpv_set_pause,
            mpv::commands::mpv_seek,
            mpv::commands::mpv_set_volume,
            mpv::commands::mpv_set_mute,
            mpv::commands::mpv_get_state,
            mpv::commands::mpv_get_tracks,
            mpv::commands::mpv_set_track,
            mpv::commands::mpv_update_context,
            // other commands
            set_window_background,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
