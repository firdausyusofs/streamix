use std::process::Stdio;

use tokio::process::Command;

#[derive(Debug)]
pub struct CodecInfo {
    pub video_codec: String,
    pub audio_codec: String,
    pub container: String,
    pub width: u32,
    pub height: u32,
    pub bitrate: u64,
}

pub async fn probe_codec(input_path: &str) -> anyhow::Result<CodecInfo> {
    // Retry up to 5 times with backoff — the torrent may have only a handful of
    // pieces when this is first called, so ffprobe can legitimately fail initially.
    let mut last_err = anyhow::anyhow!("ffprobe never ran");

    for attempt in 0..5u32 {
        if attempt > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(800 * attempt as u64)).await;
        }

        let output = Command::new("ffprobe")
            .args(&[
                // Give the HTTP source extra time and buffer space to start up.
                "-analyzeduration", "20000000", // 20 s in µs
                "-probesize",       "50000000", // 50 MB
                "-v",               "quiet",
                "-print_format",    "json",
                "-show_streams",
                "-show_format",
                input_path,
            ])
            .output()
            .await;

        let output = match output {
            Ok(o) => o,
            Err(e) => { last_err = e.into(); continue; }
        };

        if !output.status.success() || output.stdout.is_empty() {
            last_err = anyhow::anyhow!(
                "ffprobe exited {:?}: {}",
                output.status.code(),
                String::from_utf8_lossy(&output.stderr)
            );
            continue;
        }

        let json: serde_json::Value = match serde_json::from_slice(&output.stdout) {
            Ok(v) => v,
            Err(e) => { last_err = e.into(); continue; }
        };

        let streams = match json["streams"].as_array() {
            Some(s) => s,
            None => { last_err = anyhow::anyhow!("no streams in ffprobe output"); continue; }
        };

        let video = match streams.iter().find(|s| s["codec_type"] == "video") {
            Some(v) => v,
            None => { last_err = anyhow::anyhow!("no video stream found"); continue; }
        };

        // Audio is optional — fall back to empty string when absent.
        let audio_codec = streams
            .iter()
            .find(|s| s["codec_type"] == "audio")
            .and_then(|a| a["codec_name"].as_str())
            .unwrap_or("")
            .to_string();

        return Ok(CodecInfo {
            video_codec: video["codec_name"].as_str().unwrap_or("").to_string(),
            audio_codec,
            container: json["format"]["format_name"].as_str().unwrap_or("").to_string(),
            width:   video["width"].as_u64().unwrap_or(0) as u32,
            height:  video["height"].as_u64().unwrap_or(0) as u32,
            bitrate: json["format"]["bit_rate"]
                .as_str()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0),
        });
    }

    Err(last_err)
}

pub enum TranscodeProfile {
    Copy,
    Software { preset: String },
    Nvenc,
    Qsv,
    VideoToolbox,
}

/// Returns true if the embedded WebView on this platform can decode HEVC natively.
/// - macOS: WKWebView supports HEVC (H.265) hardware decode.
/// - Windows: WebView2 support depends on installed codec packs; assume false for safety.
/// - Linux: WebView (gtk-webkit2) rarely supports HEVC; assume false.
pub fn platform_supports_hevc() -> bool {
    #[cfg(target_os = "macos")]
    { true }
    #[cfg(not(target_os = "macos"))]
    { false }
}

pub fn decide_transcode(info: &CodecInfo, client_supports_hevc: bool) -> TranscodeProfile {
    let needs_transcode = match info.video_codec.as_str() {
        "h264" => false,
        "hevc" | "h265" => !client_supports_hevc,
        _ => true,
    };

    // Even when the video stream can be copied we may still need to remux the container
    // (e.g. MKV → fMP4). TranscodeProfile::Copy means "copy video, remux to MP4 pipe".
    if !needs_transcode {
        return TranscodeProfile::Copy;
    }

    if std::env::var("NVENC_AVAILABLE").is_ok() {
        return TranscodeProfile::Nvenc;
    }
    #[cfg(target_os = "macos")]
    return TranscodeProfile::VideoToolbox;

    #[allow(unreachable_code)]
    TranscodeProfile::Software {
        preset: "veryfast".to_string()
    }
}

pub async fn spawn_ffmpeg_hls(
    input: &str,
    output_dir: &str,
    profile: TranscodeProfile,
) -> anyhow::Result<tokio::process::Child> {
    let video_codecs = match &profile {
        TranscodeProfile::Copy => "copy".to_string(),
        TranscodeProfile::Software { preset } => format!("libx264 -preset {} -crf 22", preset),
        TranscodeProfile::Nvenc => "h264_nvenc -preset p4".to_string(),
        TranscodeProfile::Qsv => "h264_qsv".to_string(),
        TranscodeProfile::VideoToolbox => "h264_videotoolbox".to_string(),
    };

    let segment_path = format!("{output_dir}/seg%03d.ts");
    let playlist_path = format!("{output_dir}/index.m3u8");

    let vc_args: Vec<&str> = video_codecs.split_whitespace().collect();
    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-i", input])
        .args(["-c:v"]).args(&vc_args)
        .args(["-c:a", "aac", "-b:a", "128k"])
        .args(["-f", "hls",
            "-hls_time", "3",
            "-hls_list_size", "0",
            "-hls_flags", "delete_segments+append_list",
            "-hls_segment_filename", &segment_path,
            &playlist_path])
        .stdout(Stdio::null())
        .stderr(Stdio::piped());

    Ok(cmd.spawn()?)
}

pub async fn spawn_ffmpeg_mp4_pipe(
    input: &str,
    profile: TranscodeProfile,
) -> anyhow::Result<tokio::process::Child> {
    let is_copy = matches!(&profile, TranscodeProfile::Copy);

    let video_codec = match &profile {
        TranscodeProfile::Copy => "copy".to_string(),
        TranscodeProfile::Software { preset } => format!("libx264 -preset {preset} -crf 22"),
        TranscodeProfile::Nvenc => "h264_nvenc -preset p4".to_string(),
        TranscodeProfile::Qsv => "h264_qsv".to_string(),
        TranscodeProfile::VideoToolbox => "h264_videotoolbox".to_string(),
    };

    let vc_args: Vec<&str> = video_codec.split_whitespace().collect();
    let mut cmd = Command::new("ffmpeg");

    // Give FFmpeg plenty of room to analyze the beginning of a slow HTTP stream.
    cmd.args(["-analyzeduration", "20000000"]) // 20 s in µs
       .args(["-probesize", "50000000"])        // 50 MB
       .args(["-i", input])
       .args(["-c:v"]).args(&vc_args)
       .args(["-c:a", "aac", "-b:a", "192k"])
       // Use all available CPU cores for encoding (especially useful for 4K transcodes).
       .args(["-threads", "0"])
       // fMP4 output flags.
       // • empty_moov  — write a minimal, self-contained moov at the very start of the
       //                 stream so the browser can start parsing immediately.
       // • frag_keyframe — start a new fragment at each keyframe (clean seek points).
       // NOTE: `faststart` is intentionally absent.  It works by seeking backward to
       //       relocate the moov atom, which is impossible on a pipe and causes FFmpeg
       //       to buffer ALL output internally before attempting the seek — resulting in
       //       the browser receiving nothing for several seconds and then closing the
       //       connection with a Broken Pipe error.
       .args(["-movflags", "frag_keyframe+empty_moov"])
       // Flush a new fragment at least every 2 seconds even when no keyframe has
       // occurred.  Without this, a 4K source with a large GOP (e.g. every 5–8 s) keeps
       // all encoded frames in memory until the first keyframe; the browser gets only the
       // 4 KB moov atom, times out waiting for video data, and closes the connection.
       .args(["-frag_duration", "2000000"])  // 2 s in µs
       // Prevent queue overflows when video bitrate is much higher than audio (4K).
       .args(["-max_muxing_queue_size", "9999"]);

    // When re-encoding (not copy), force a keyframe at least every 2 seconds so that
    // frag_keyframe produces fragments at regular, predictable intervals.
    if !is_copy {
        cmd.args(["-force_key_frames", "expr:gte(t,n_forced*2)"]);
    }

    cmd.args(["-f", "mp4", "pipe:1"])
       .stdout(Stdio::piped())
       .stderr(Stdio::inherit());

    Ok(cmd.spawn()?)
}
