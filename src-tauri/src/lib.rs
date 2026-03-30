use std::sync::Arc;

use stremio::models::PlayStreamRequest;
use stremio::mpv::MpvStatus;

pub mod stremio;

// ── Stremio addon commands ────────────────────────────────────────────────────

#[tauri::command]
async fn get_installed_addons() -> Result<stremio::store::AddonConfig, String> {
    Ok(stremio::store::init_addons().await)
}

#[tauri::command]
async fn fetch_catalog_from_addon(
    manifest_url: String,
    item_type: String,
    catalog_id: String,
) -> Result<stremio::models::CatalogResponse, String> {
    stremio::client::fetch_catalog(&manifest_url, &item_type, &catalog_id)
        .await
        .map_err(|e| format!("Failed to fetch catalog: {e}"))
}

#[tauri::command]
async fn fetch_streams_from_addon(
    manifest_url: String,
    item_type: String,
    id: String,
) -> Result<stremio::models::StreamResponse, String> {
    println!("Fetching streams for {item_type}/{id} from {manifest_url}");
    stremio::client::fetch_streams(&manifest_url, &item_type, &id)
        .await
        .map_err(|e| format!("Failed to fetch streams: {e}"))
}

// ── Primary play path: Torrent → raw URL → MPV ───────────────────────────────

/// Resolve the stream to a playable URL, then load it into MPV.
///
/// MPV opens in its own OS-native window and handles every codec
/// (HEVC, AV1, FLAC, …) with hardware decoding — no transcoding needed.
#[tauri::command]
async fn play_in_mpv(stream: PlayStreamRequest) -> Result<(), String> {
    let url = resolve_stream_url(&stream).await?;

    let mpv = stremio::mpv::MPV_MANAGER
        .get()
        .ok_or("MPV not initialised — is mpv installed and in PATH?")?;

    mpv.load_file(&url)
        .await
        .map_err(|e| format!("MPV load failed: {e}"))
}

async fn resolve_stream_url(stream: &PlayStreamRequest) -> Result<String, String> {
    if let Some(info_hash) = &stream.info_hash {
        let file_idx = stream.file_idx.unwrap_or(0) as usize;

        let manager = stremio::torrent::TORRENT_MANAGER
            .get()
            .ok_or("Torrent Manager not initialised")?;

        manager
            .stream_torrent(info_hash, file_idx)
            .await
            .map_err(|e| e.to_string())?;

        let port = stremio::server::SERVER_PORT.get().unwrap();
        return Ok(format!("http://127.0.0.1:{port}/raw/{info_hash}/{file_idx}"));
    }

    if let Some(url) = &stream.url {
        if !url.is_empty() {
            return Ok(url.clone());
        }
    }

    Err("Invalid stream request: no infoHash or URL provided".into())
}

// ── MPV playback controls ─────────────────────────────────────────────────────

#[tauri::command]
async fn mpv_set_pause(paused: bool) -> Result<(), String> {
    get_mpv()?.set_pause(paused).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn mpv_seek(seconds: f64) -> Result<(), String> {
    get_mpv()?.seek(seconds).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn mpv_set_volume(volume: f64) -> Result<(), String> {
    get_mpv()?.set_volume(volume).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn mpv_get_status() -> Result<MpvStatus, String> {
    get_mpv()?.get_status().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn mpv_stop() -> Result<(), String> {
    get_mpv()?.stop().await.map_err(|e| e.to_string())
}

fn get_mpv() -> Result<&'static Arc<stremio::mpv::MpvManager>, String> {
    stremio::mpv::MPV_MANAGER
        .get()
        .ok_or_else(|| "MPV not initialised — is mpv installed and in PATH?".into())
}

// ── App entry point ───────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let proj_dirs =
                directories::ProjectDirs::from("com", "fy", "streamix").unwrap();
            let cache_dir = proj_dirs.cache_dir().join("torrents");
            std::fs::create_dir_all(&cache_dir).unwrap();

            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                // TorrentManager
                let manager = stremio::torrent::TorrentManager::new(cache_dir)
                    .await
                    .expect("Failed to init TorrentManager");
                let _ = stremio::torrent::TORRENT_MANAGER.set(Arc::new(manager));

                // Axum HTTP server (raw torrent byte streaming)
                tokio::spawn(stremio::server::start_server());

                // MPV — spawns in its own OS window, cross-platform
                let mpv_manager = Arc::new(stremio::mpv::MpvManager::new());
                match mpv_manager.launch().await {
                    Ok(_)  => println!("✅ MPV ready"),
                    Err(e) => eprintln!("⚠️  MPV launch failed: {e}"),
                }
                let _ = stremio::mpv::MPV_MANAGER.set(mpv_manager);

                drop(app_handle); // keep app_handle alive until here
            });

            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_installed_addons,
            fetch_catalog_from_addon,
            fetch_streams_from_addon,
            play_in_mpv,
            mpv_set_pause,
            mpv_seek,
            mpv_set_volume,
            mpv_get_status,
            mpv_stop,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
