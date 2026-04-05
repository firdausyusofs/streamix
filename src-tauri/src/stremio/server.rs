use std::sync::OnceLock;

use axum::{Json, Router, body::Body, extract::{Path, Request}, http::Response, routing::get};
use directories::ProjectDirs;
use reqwest::{StatusCode, header};
use tokio::io::AsyncSeekExt;
use tokio_util::io::ReaderStream;
use tower_http::{cors::{Any, CorsLayer}, services::ServeDir};

use super::models::{TorrentStatsResponse, FileStats};

pub static SERVER_PORT: OnceLock<u16> = OnceLock::new();

pub async fn start_server() {
    let proj_dirs = ProjectDirs::from("com", "fy", "streamix").unwrap();
    let hls_dir = proj_dirs.cache_dir().join("torrents").join("hls");
    std::fs::create_dir_all(&hls_dir).unwrap();

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/stream/:info_hash/:file_idx", get(stream_handler))
        .route("/:info_hash/stats", get(stats_handler))
        .nest_service("/hls", ServeDir::new(hls_dir))
        .layer(cors);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    SERVER_PORT.set(port).expect("Failed to set server port");

    println!("Server is running on http://127.0.0.1:{}", port);
    axum::serve(listener, app).await.unwrap();
}

async fn stream_handler(
    Path((info_hash, file_idx)): Path<(String, usize)>,
    req: Request
) -> Result<Response<Body>, StatusCode> {
    let handle = {
        let handles = crate::stremio::torrent::TORRENT_HANDLES
            .get()
            .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?
            .read()
            .unwrap();

        handles.get(&info_hash).ok_or(StatusCode::NOT_FOUND)?.clone()
    };

    let mut stream = handle.stream(file_idx).map_err(|e| {
        eprintln!("Error accessing torrent stream: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let total_len = stream.seek(std::io::SeekFrom::End(0)).await.map_err(|e| {
        eprintln!("Error seeking torrent stream: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let mut start = 0;
    let mut end = total_len - 1;

    if let Some(range) = req.headers().get(header::RANGE).and_then(|v| v.to_str().ok()) {
        if range.starts_with("bytes=") {
            let parts: Vec<&str> = range["bytes=".len()..].split('-').collect();
            if parts.len() == 2 {
                if let Ok(s) = parts[0].parse::<u64>() { start = s; }
                if let Ok(e) = parts[1].parse::<u64>() { end = e; }
            }
        }
    }

    if start > end || start >= total_len {
        return Err(StatusCode::RANGE_NOT_SATISFIABLE);
    }

    let chunk_size = end - start + 1;

    stream.seek(std::io::SeekFrom::Start(start)).await.map_err(|e| {
        eprintln!("Error seeking torrent stream: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let response = Response::builder()
        .status(if start == 0 && end == total_len - 1 { StatusCode::OK } else { StatusCode::PARTIAL_CONTENT })
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(header::CONTENT_LENGTH, chunk_size.to_string())
        .header(header::CONTENT_RANGE, format!("bytes {}-{}/{}", start, end, total_len));

    let reader_stream = ReaderStream::new(stream);
    let body = Body::from_stream(reader_stream);

    Ok(response.body(body).unwrap())
}

async fn stats_handler(
    Path(info_hash): Path<String>
) -> Result<Json<TorrentStatsResponse>, StatusCode> {
    let handle = {
        let handles = crate::stremio::torrent::TORRENT_HANDLES
            .get()
            .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?
            .read()
            .unwrap();

        handles.get(&info_hash).ok_or(StatusCode::NOT_FOUND)?.clone()
    };

    let is_paused = handle.is_paused();
    let stats = handle.stats();

    let downloaded = stats.progress_bytes as i64;
    let uploaded = stats.uploaded_bytes as i64;

    let mut down_speed = 0.0;
    let mut up_speed = 0.0;

    let mut peers = 0;
    let mut queued_peers = 0;
    let mut seen_peers = 0;
    let mut connecting_peers = 0;

    if let Some(live) = &stats.live {
        let peer_stats = &live.snapshot.peer_stats;
        peers = peer_stats.live as i64;
        queued_peers = peer_stats.queued as i64;
        seen_peers = peer_stats.seen as i64;
        connecting_peers = peer_stats.connecting as i64;

        down_speed = live.download_speed.mbps * 1_048_576.0;
        up_speed = live.upload_speed.mbps * 1_048_576.0;
    }

    let file_stats: Vec<FileStats> = handle.with_metadata(|metadata| {
        metadata.file_infos.iter().map(|file| {
            let path = file.relative_filename.to_string_lossy().to_string();

            let name = file.relative_filename.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| path.clone());

            FileStats {
                path,
                name,
                length: file.len as i64,
                offset: file.offset_in_torrent as i64,
            }
        }).collect()
    }).unwrap_or_else(|_| vec![]);

    let response = TorrentStatsResponse {
        info_hash: info_hash.clone(),
        name: handle.name().unwrap_or("Unknown".to_string()).to_string(),
        downloaded,
        uploaded,
        download_speed: down_speed,
        upload_speed: up_speed,
        peers,
        queued: queued_peers,
        unique: seen_peers,
        is_paused,
        files: file_stats,
    };

    Ok(Json(response))
}
