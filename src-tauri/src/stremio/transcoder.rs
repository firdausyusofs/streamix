use std::{path::PathBuf, process::{Command, Stdio}};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct ProbeOutput {
    pub streams: Vec<StreamInfo>,
}

#[derive(Deserialize, Debug)]
pub struct StreamInfo {
    pub codec_type: String,
    pub codec_name: Option<String>,
}

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

    let probe_cmd = Command::new("ffprobe")
        .args(&[
            "-v", "error",
            "-select_streams", "v:0",
            "-show_streams",
            "-show_format",
            "-print_format", "json",
            raw_stream_url,
        ])
        .output()
        .map_err(|e| format!("Failed to execute ffprobe: {}", e))?;

    if !probe_cmd.status.success() {
        let err_msg = String::from_utf8_lossy(&probe_cmd.stderr);
        eprintln!("❌ FFprobe failed: {}", err_msg);
        return Err(format!("FFprobe failed to read the stream: {}", err_msg));
    }

    let probe_data: ProbeOutput = serde_json::from_slice(&probe_cmd.stdout)
        .map_err(|e| format!("Failed to parse ffprobe JSON: {}", e))?;

    let video_stream = probe_data.streams.iter().find(|s| s.codec_type == "video");
    let audio_streams: Vec<&StreamInfo> = probe_data.streams.iter().filter(|s| s.codec_type == "audio").collect();

    if video_stream.is_none() {
        return Err("No video stream found in the input.".to_string());
    }

    let v_codec = video_stream.unwrap().codec_name.as_deref().unwrap_or("unknown");
    let a_codec = audio_streams.first().and_then(|s| s.codec_name.as_deref()).unwrap_or("none");

    println!("Detected video codec: {}, audio codec: {}", v_codec, a_codec);

    let mut ffmpeg = Command::new("ffmpeg");
    ffmpeg.args(&["-i", raw_stream_url, "-sn"]);

    ffmpeg.args(&["-map", "0:v:0", "-map", "0:a?", "-map_metadata", "0"]);

    let safe_video_codecs = ["h264", "vp8", "vp9", "av1"];
    if safe_video_codecs.contains(&v_codec) {
        println!("⚡ Direct Stream: Copying Video ({})", v_codec);
        ffmpeg.args(&["-c:v", "copy"]);
    } else {
        println!("🔥 Transcoding {} to H264...", v_codec);
        ffmpeg.args(&[
            "-c:v", "libx264",
            "-preset", "ultrafast",
            "-crf", "23",
            "-force_key_frames", "expr:gte(t,n_forced*4)"
        ]);
    }

    let safe_audio_codecs = ["aac", "mp3", "opus", "vorbis", "flac"];
    if safe_audio_codecs.contains(&a_codec) {
        println!("⚡ Direct Stream: Copying Audio ({})", a_codec);
        ffmpeg.args(&["-c:a", "copy"]);
    } else {
        println!("🔥 Transcoding Audio: {} -> aac", a_codec);
        ffmpeg.args(&["-c:a", "aac", "-b:a", "192k", "-ac", "2"]);
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

    let mut attempts = 0;
    while !playlist_path.exists() && attempts < 40 {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        attempts += 1;
    }

    if !playlist_path.exists() {
        return Err("FFmpeg failed to generate the HLS playlist in time.".to_string());
    }

    Ok(format!("/hls/{}/stream.m3u8", info_hash))
}
