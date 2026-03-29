use std::{path::PathBuf, process::{Command, Stdio}};

pub async fn prepare_stream(
    raw_stream_url: &str,
    info_hash: &str,
    cache_dir: PathBuf
) -> Result<String, String> {
    let hls_dir = cache_dir.join("hls").join(info_hash);
    if !hls_dir.exists() {
        std::fs::create_dir_all(&hls_dir).map_err(|e| format!("Failed to create HLS directory: {}", e))?;
    }

    let playlist_path = hls_dir.join("stream.m3u8");

    if playlist_path.exists() {
        return Ok(format!("/hls/{}/stream.m3u8", info_hash));
    }

    println!("Probing stream URL: {}", raw_stream_url);

    let probe_output = Command::new("ffprobe")
        .args(&[
            "-v", "error",
            "-probesize", "5000000",
            "-analyzeduration", "5000000",
            "-select_streams", "v:0",
            "-show_entries", "stream=codec_name",
            "-of", "default=noprint_wrappers=1:nokey=1",
            raw_stream_url,
        ])
        .output()
        .map_err(|e| format!("Failed to execute ffprobe: {}", e))?;

    if !probe_output.status.success() {
        let err_msg = String::from_utf8_lossy(&probe_output.stderr);
        eprintln!("❌ FFprobe failed: {}", err_msg);
        return Err(format!("FFprobe failed to read the stream: {}", err_msg));
    }

    let video_codec = String::from_utf8_lossy(&probe_output.stdout).trim().to_string();
    println!("Detected video codec: {}", video_codec);

    let mut ffmpeg = Command::new("ffmpeg");
    ffmpeg.args(&["-i", raw_stream_url]);

    ffmpeg.arg("-sn");

    if video_codec == "h264" {
        println!("⚡ Direct Stream (Remuxing H264 to HLS)");
        ffmpeg.args(&["-c:v", "copy", "-c:a", "aac", "-b:a", "192k", "-ac", "2"]);
    } else {
        println!("🔥 Transcoding {} to H264...", video_codec);
        ffmpeg.args(&[
            "-c:v", "libx264",
            "-preset", "ultrafast",
            "-crf", "23", // Good balance of quality/speed
            "-c:a", "aac",
            "-b:a", "192k",
            "-ac", "2",
            "-force_key_frames", "expr:gte(t,n_forced*4)"
        ]);
    }

    println!("Starting ffmpeg with HLS output to: {}", playlist_path.display());

    ffmpeg.args(&[
        "-f", "hls",
        "-hls_time", "4", // 4 second chunks
        "-hls_list_size", "0", // Keep all chunks in the playlist to allow seeking
        "-hls_playlist_type", "event",
        "-max_muxing_queue_size", "1024",
        "-hls_segment_filename", hls_dir.join("segment_%03d.ts").to_str().unwrap(),
        playlist_path.to_str().unwrap()
    ]);

    ffmpeg.stdin(Stdio::null())
          .stdout(Stdio::inherit())
          .stderr(Stdio::inherit())
          .spawn()
          .map_err(|e| format!("Failed to spawn ffmpeg: {}", e))?;

    // let mut attempts = 0;
    // while !playlist_path.exists() && attempts < 20 {
    while !playlist_path.exists() {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        // attempts += 1;
    }

    if !playlist_path.exists() {
        return Err("FFmpeg failed to generate the HLS playlist in time.".to_string());
    }

    Ok(format!("/hls/{}/stream.m3u8", info_hash))
}
