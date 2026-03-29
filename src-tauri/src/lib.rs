use directories::ProjectDirs;
use stremio::models::PlayStreamRequest;

pub mod stremio;

#[tauri::command]
async fn get_installed_addons() -> Result<stremio::store::AddonConfig, String> {
    let config = stremio::store::init_addons().await;
    Ok(config)
}

#[tauri::command]
async fn fetch_catalog_from_addon(
    manifest_url: String,
    item_type: String,
    catalog_id: String,
) -> Result<stremio::models::CatalogResponse, String> {
    match stremio::client::fetch_catalog(&manifest_url, &item_type, &catalog_id).await {
        Ok(response) => Ok(response),
        Err(e) => Err(format!("Failed to fetch catalog: {}", e)),
    }
}

#[tauri::command]
async fn fetch_streams_from_addon(
    manifest_url: String,
    item_type: String,
    id: String,
) -> Result<stremio::models::StreamResponse, String> {
    println!(
        "Fetching streams for item_type: {}, id: {} from manifest_url: {}",
        item_type, id, manifest_url
    );
    match stremio::client::fetch_streams(&manifest_url, &item_type, &id).await {
        Ok(response) => Ok(response),
        Err(e) => Err(format!("Failed to fetch streams: {}", e)),
    }
}

#[tauri::command]
async fn play_stream_command(stream: PlayStreamRequest) -> Result<String, String> {
    if let Some(url) = stream.url {
        println!("Direct stream URL provided: {}", url);
        return Ok(url);
    }

    if let Some(info_hash) = stream.info_hash {
        println!("Requesting torrent stream for hash: {}", info_hash);

        let raw_stream_url = match crate::stremio::torrent::start_stream(&info_hash, stream.file_idx).await {
            Some(url) => url,
            None => return Err("Failed to initialize torrent stream".to_string()),
        };

        let proj_dirs = ProjectDirs::from("com", "fy", "streamix")
            .ok_or("Could not find project directories")?;
        let cache_dir = proj_dirs.cache_dir().join("torrents");

        let hls_route = crate::stremio::transcoder::prepare_stream(&raw_stream_url, &info_hash, cache_dir)
            .await?;

        let port = crate::stremio::server::SERVER_PORT.get()
            .ok_or("Axum server port not initialized")?;

        let final_hls_url = format!("http://127.0.0.1:{}{}", port, hls_route);

        return Ok(final_hls_url);
    }

    Err("Invalid stream request: No URL or infoHash provided".to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|_app| {
            tauri::async_runtime::spawn(async move {
                crate::stremio::server::start_server().await;
            });
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_installed_addons,
            fetch_catalog_from_addon,
            fetch_streams_from_addon,
            play_stream_command
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
