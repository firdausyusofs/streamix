use std::sync::OnceLock;

use axum::{Router, body::Body, extract::{Path, Request}, http::Response, routing::get};
use reqwest::{StatusCode, header};
use tokio::io::AsyncSeekExt;
use tokio_util::io::ReaderStream;

pub static SERVER_PORT: OnceLock<u16> = OnceLock::new();

pub async fn start_server() {
    let app = Router::new().route("/stream/:info_hash/:file_idx", get(stream_handler));

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

    let mut response = Response::builder()
        .status(if start == 0 && end == total_len - 1 { StatusCode::OK } else { StatusCode::PARTIAL_CONTENT })
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(header::CONTENT_LENGTH, chunk_size.to_string())
        .header(header::CONTENT_RANGE, format!("bytes {}-{}/{}", start, end, total_len));

    let reader_stream = ReaderStream::new(stream);
    let body = Body::from_stream(reader_stream);

    Ok(response.body(body).unwrap())
}
