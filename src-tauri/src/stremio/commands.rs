use directories::ProjectDirs;
use librqbit::api::TorrentIdOrHash;

use super::models::PlayStreamRequest;

#[tauri::command]
pub async fn get_installed_addons() -> Result<super::store::AddonConfig, String> {
    let config = super::store::init_addons().await;
    Ok(config)
}

#[tauri::command]
pub async fn fetch_catalog_from_addon(
    manifest_url: String,
    item_type: String,
    catalog_id: String,
) -> Result<super::models::CatalogResponse, String> {
    match super::client::fetch_catalog(&manifest_url, &item_type, &catalog_id).await {
        Ok(response) => Ok(response),
        Err(e) => Err(format!("Failed to fetch catalog: {}", e)),
    }
}

#[tauri::command]
pub async fn fetch_streams_from_addon(
    manifest_url: String,
    item_type: String,
    id: String,
) -> Result<super::models::StreamResponse, String> {
    println!(
        "Fetching streams for item_type: {}, id: {} from manifest_url: {}",
        item_type, id, manifest_url
    );
    match super::client::fetch_streams(&manifest_url, &item_type, &id).await {
        Ok(response) => Ok(response),
        Err(e) => Err(format!("Failed to fetch streams: {}", e)),
    }
}

#[tauri::command]
pub async fn play_stream_command(stream: PlayStreamRequest) -> Result<String, String> {
    if let Some(url) = stream.url {
        println!("Direct stream URL provided: {}", url);
        return Ok(url);
    }

    if let Some(info_hash) = stream.info_hash {
        println!("Requesting torrent stream for hash: {}", info_hash);

        let raw_stream_url = match super::torrent::start_stream(&info_hash, stream.file_idx).await {
            Some(url) => url,
            None => return Err("Failed to initialize torrent stream".to_string()),
        };

        let proj_dirs = ProjectDirs::from("com", "fy", "streamix")
            .ok_or("Could not find project directories")?;
        let cache_dir = proj_dirs.cache_dir().join("torrents");

        let hls_route = super::transcoder::prepare_stream(&raw_stream_url, &info_hash, cache_dir)
            .await?;

        let port = super::server::SERVER_PORT.get()
            .ok_or("Axum server port not initialized")?;

        let final_hls_url = format!("http://127.0.0.1:{}{}", port, hls_route);

        return Ok(final_hls_url);
    }

    Err("Invalid stream request: No URL or infoHash provided".to_string())
}

#[tauri::command]
pub async fn play_stream_for_mpv(stream: PlayStreamRequest) -> Result<String, String> {
    if let Some(url) = stream.url {
        println!("Direct stream URL for mpv: {}", url);
        return Ok(url);
    }

    if let Some(info_hash) = stream.info_hash {
        println!("Requesting torrent stream for mpv, hash: {}", info_hash);

        let raw_stream_url = match super::torrent::start_stream(&info_hash, stream.file_idx).await {
            Some(url) => url,
            None => return Err("Failed to initialize torrent stream".to_string()),
        };

        // Return the raw stream URL directly — mpv plays it natively
        return Ok(raw_stream_url);
    }

    Err("Invalid stream request: No URL or infoHash provided".to_string())
}

#[tauri::command]
pub async fn stop_stream_command(info_hash: String) -> Result<(), String> {
    println!("Stopping torrent stream for hash: {}", info_hash);

    if let Some(map) = super::torrent::TORRENT_HANDLES.get() {
        let handle_opt = map.write().unwrap().remove(&info_hash);

        if let Some(handle) = handle_opt {
            let session = super::torrent::get_session().await;

            if let Err(e) = session.delete(TorrentIdOrHash::Id(handle.id()), true).await {
                eprintln!("Failed to remove torrent: {}", e);
                return Err(format!("Failed to stop stream: {}", e));
            } else {
                println!("Torrent stream stopped successfully for hash: {}", info_hash);
                return Ok(());
            }
        }
    }

    Ok(())
}
