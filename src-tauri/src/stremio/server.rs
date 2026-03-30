use std::sync::OnceLock;

use axum::{Router, body::Body, extract::{Path, Query, Request}, http::Response, routing::get};
use directories::ProjectDirs;
use reqwest::{StatusCode, header};
use serde::Deserialize;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio_util::io::ReaderStream;
use tower_http::cors::{Any, CorsLayer};

// use crate::stremio::transcoder::ProbeOutput;

pub static SERVER_PORT: OnceLock<u16> = OnceLock::new();

#[derive(Deserialize, Debug)]
pub struct StreamQuery {
    pub start: Option<f64>, // For seeking later!
}

pub async fn start_server() {
    let proj_dirs = ProjectDirs::from("com", "fy", "streamix").unwrap();
    let hls_dir = proj_dirs.cache_dir().join("torrents").join("hls");
    std::fs::create_dir_all(&hls_dir).unwrap();

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/raw/:info_hash/:file_idx", get(raw_stream_handler))
        .route("/stream/:info_hash/:file_idx", get(stream_handler))
        // .nest_service("/hls", ServeDir::new(hls_dir))
        .layer(cors);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    SERVER_PORT.set(port).expect("Failed to set server port");

    println!("Server is running on http://127.0.0.1:{}", port);
    axum::serve(listener, app).await.unwrap();
}

async fn raw_stream_handler(
    Path((info_hash, file_idx)): Path<(String, usize)>,
    req: Request
) -> Result<Response<Body>, StatusCode> {
    let manager = crate::stremio::torrent::TORRENT_MANAGER
        .get()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;


    let handle = manager
        .handles
        .read()
        .unwrap()
        .get(&info_hash)
        .cloned()
        .ok_or(StatusCode::NOT_FOUND)?;

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

    // CRITICAL: limit the read to exactly the requested byte range.
    // Without .take(), ReaderStream sends data until EOF regardless of Content-Length,
    // which causes FFmpeg / the browser to see more bytes than declared and lose sync.
    let limited = stream.take(chunk_size);
    let reader_stream = ReaderStream::new(limited);
    let body = Body::from_stream(reader_stream);

    Ok(response.body(body).unwrap())
}

pub async fn stream_handler(
    Path((info_hash, file_idx)): Path<(String, usize)>,
    Query(query): Query<StreamQuery>,
) -> Result<Response<Body>, StatusCode> {
    let port = crate::stremio::server::SERVER_PORT.get().unwrap_or(&0);
    let raw_stream_url = format!("http://127.0.0.1:{}/raw/{}/{}", port, info_hash, file_idx);

    println!("🔍 Probing raw stream for fMP4 pipeline: {}", raw_stream_url);

    // 1. Probe the stream. probe_codec already retries internally with backoff.
    let codec_info = match crate::stremio::ffmpeg::probe_codec(&raw_stream_url).await {
        Ok(info) => info,
        Err(e) => {
            eprintln!("FFprobe failed to read stream: {}", e);
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    println!("📊 Detected Codecs -> Video: {}, Audio: {}", codec_info.video_codec, codec_info.audio_codec);

    // 2. Decide the transcode profile using platform-aware HEVC detection.
    //    On macOS, WKWebView can decode HEVC (H.265) natively, so we just remux.
    //    On other platforms we transcode to H.264 for compatibility.
    let profile = crate::stremio::ffmpeg::decide_transcode(
        &codec_info,
        crate::stremio::ffmpeg::platform_supports_hevc(),
    );

    if let Some(start_time) = query.start {
        println!("⏩ Seeking to {} seconds", start_time);
    }

    // 3. Spawn the hardware-accelerated fMP4 pipe.
    let mut child = crate::stremio::ffmpeg::spawn_ffmpeg_mp4_pipe(
        &raw_stream_url,
        profile
    ).await.map_err(|e| {
        eprintln!("Failed to spawn FFmpeg: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let stdout = child.stdout.take().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    // Reap the child process once it exits (either naturally or via SIGPIPE when the
    // client disconnects). Without this the process remains a zombie in the OS table
    // until the parent process itself exits.
    tokio::spawn(async move {
        match child.wait().await {
            Ok(status) if !status.success() => {
                // Exit code 141 (128+SIGPIPE) is normal when the browser closes the
                // connection before FFmpeg finishes.  Don't log that as an error.
                if status.code() != Some(141) {
                    eprintln!("FFmpeg exited with status: {}", status);
                }
            }
            Err(e) => eprintln!("Failed to wait for FFmpeg: {}", e),
            _ => {}
        }
    });

    // 4. Wrap stdout in an Axum Body stream.
    let stream = ReaderStream::new(stdout);
    let body = Body::from_stream(stream);

    // 5. Send the HTTP Response.
    //    Accept-Ranges: none — the fMP4 is a live pipe, byte-range seeking is not
    //    possible here. The seek bar still works via timeupdate / currentTime.
    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "video/mp4")
        .header(header::ACCEPT_RANGES, "none")
        .body(body)
        .unwrap();

    Ok(response)
}
