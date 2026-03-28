use std::{collections::HashMap, sync::{Arc, OnceLock, RwLock}};

use directories::ProjectDirs;
use librqbit::{AddTorrent, AddTorrentOptions, ManagedTorrent, Session};
use tokio::sync::OnceCell;

pub static TORRENT_HANDLES: OnceLock<RwLock<HashMap<String, Arc<ManagedTorrent>>>> = OnceLock::new();
pub static TORRENT_SESSION: OnceCell<Arc<Session>> = OnceCell::const_new();

pub async fn get_session() -> Arc<Session> {
    TORRENT_SESSION
        .get_or_init(|| async {
            let proj_dirs = ProjectDirs::from("com", "fy", "streamix").unwrap();
            let download_dir = proj_dirs.cache_dir().join("torrents");

            if let Err(e) = std::fs::create_dir_all(&download_dir) {
                eprintln!("Failed to create torrent download directory: {}", e);
            }

            println!("Initializing torrent session with download directory: {:?}", download_dir);

            let session = Session::new(download_dir)
                .await
                .expect("Failed to initialize torrent session");

            session
        })
        .await
        .clone()
}

pub async fn start_stream(info_hash: &str, file_idx: Option<u32>) -> Option<String> {
    let session = get_session().await;
    let magnet_uri = format!("magnet:?xt=urn:btih:{}", info_hash);

    println!("🧲 Connecting to swarm for: {}", magnet_uri);

    let mut options = AddTorrentOptions::default();
    options.overwrite = true;

    if let Some(idx) = file_idx {
        options.only_files = Some(vec![idx as usize]);
    }

    let add_result = match session
        .add_torrent(AddTorrent::from_url(&magnet_uri), Some(options))
        .await {
        Ok(res) => res,
        Err(e) => {
            eprintln!("Failed to add torrent: {}", e);
            return None;
        }
    };

    println!("⏳ Torrent added to session, waiting for metadata...");

    let handle = match add_result.into_handle() {
        Some(h) => h,
        None => {
            eprintln!("Failed to get torrent handle after adding torrent");
            return None;
        }
    };
    if let Err(e) = handle.wait_until_initialized().await {
        eprintln!("Error waiting for torrent metadata: {}", e);
        return None;
    }

    println!("✅ Metadata retrieved for info hash: {}", info_hash);

    let handles_map = TORRENT_HANDLES.get_or_init(|| RwLock::new(HashMap::new()));
    handles_map.write().unwrap().insert(info_hash.to_string(), handle.clone());

    let port = crate::stremio::server::SERVER_PORT.get().expect("Server port not initialized");

    println!("🎬 Starting stream for info hash: {} on port {}", info_hash, port);

    let idx = file_idx.unwrap_or(0);
    let stream_url = format!("http://127.0.0.1:{}/stream/{}/{}", port, info_hash, idx);

    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    Some(stream_url)
}
