use std::{
    path::PathBuf,
    sync::{Arc, Mutex, OnceLock},
};

use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

// ── Globals ───────────────────────────────────────────────────────────────────

pub static MPV_MANAGER: OnceLock<Arc<MpvManager>> = OnceLock::new();

// ── MpvManager ────────────────────────────────────────────────────────────────

pub struct MpvManager {
    pub socket_path: PathBuf,
    child: Mutex<Option<tokio::process::Child>>,
}

impl MpvManager {
    pub fn new() -> Self {
        // Unix: use a temp socket file.
        // Windows: MPV named-pipe IPC uses \\.\pipe\<name>.
        #[cfg(unix)]
        let socket_path = std::env::temp_dir().join("streamix-mpv.sock");
        #[cfg(windows)]
        let socket_path = PathBuf::from(r"\\.\pipe\streamix-mpv");

        Self { socket_path, child: Mutex::new(None) }
    }

    /// Spawn MPV in its own OS-native borderless window.
    /// Works on Windows, macOS, and Linux without any platform-specific code.
    pub async fn launch(&self) -> anyhow::Result<()> {
        self.kill().await;

        #[cfg(unix)]
        let _ = tokio::fs::remove_file(&self.socket_path).await;

        let socket_arg = self.socket_path.to_str()
            .ok_or_else(|| anyhow::anyhow!("socket path is not valid UTF-8"))?
            .to_owned();

        let child = tokio::process::Command::new("mpv")
            .args([
                "--idle=yes",
                "--keep-open=yes",
                "--no-terminal",
                "--force-window=yes",
                // No window chrome — the OS window is the player UI.
                "--no-border",
                // Hardware decoding for 4K/HEVC without pegging the CPU.
                "--hwdec=auto-safe",
                // Generous read-ahead so torrent streams don't stutter.
                "--cache=yes",
                "--demuxer-max-bytes=150MiB",
                "--demuxer-readahead-secs=30",
            ])
            .arg(format!("--input-ipc-server={socket_arg}"))
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to start mpv: {e}. Is mpv installed and in PATH?"))?;

        *self.child.lock().unwrap() = Some(child);

        // Wait up to 3 s for the IPC endpoint to be ready.
        for _ in 0..30 {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            if Self::ipc_ready(&self.socket_path).await {
                return Ok(());
            }
        }

        anyhow::bail!("MPV IPC did not become available — is mpv installed?")
    }

    /// Returns true once MPV's IPC endpoint is accepting connections.
    async fn ipc_ready(path: &PathBuf) -> bool {
        #[cfg(unix)]
        { path.exists() }

        #[cfg(windows)]
        {
            use tokio::net::windows::named_pipe::ClientOptions;
            ClientOptions::new().open(path).is_ok()
        }
    }

    pub async fn kill(&self) {
        let child_opt = self.child.lock().unwrap().take();
        if let Some(mut child) = child_opt {
            let _ = child.kill().await;
            let _ = child.wait().await;
        }
    }

    // ── IPC ──────────────────────────────────────────────────────────────────

    /// Send a JSON-IPC command to MPV and return its response.
    pub async fn send_ipc(&self, command: Vec<Value>) -> anyhow::Result<Value> {
        use tokio::time::{timeout, Duration};

        let payload =
            serde_json::to_string(&serde_json::json!({ "command": command, "request_id": 1 }))?
                + "\n";

        #[cfg(unix)]
        {
            use tokio::net::UnixStream;

            let stream = timeout(
                Duration::from_secs(3),
                UnixStream::connect(&self.socket_path),
            )
            .await
            .map_err(|_| anyhow::anyhow!("MPV IPC: connect timeout"))?
            .map_err(|e| anyhow::anyhow!("MPV IPC: {e}"))?;

            Self::exchange(stream, &payload).await
        }

        #[cfg(windows)]
        {
            use tokio::net::windows::named_pipe::ClientOptions;

            // Named pipes may be busy; retry briefly.
            let stream = timeout(Duration::from_secs(3), async {
                loop {
                    match ClientOptions::new().open(&self.socket_path) {
                        Ok(s)  => return Ok(s),
                        Err(e) if e.raw_os_error() == Some(231) => {
                            // ERROR_PIPE_BUSY
                            tokio::time::sleep(Duration::from_millis(50)).await;
                        }
                        Err(e) => return Err(anyhow::anyhow!("MPV IPC pipe: {e}")),
                    }
                }
            })
            .await
            .map_err(|_| anyhow::anyhow!("MPV IPC: connect timeout"))??;

            Self::exchange(stream, &payload).await
        }

        #[cfg(not(any(unix, windows)))]
        {
            let _ = payload;
            Err(anyhow::anyhow!("MPV IPC not supported on this platform"))
        }
    }

    async fn exchange<S>(stream: S, payload: &str) -> anyhow::Result<Value>
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    {
        use tokio::time::{timeout, Duration};

        let (reader, mut writer) = tokio::io::split(stream);
        writer.write_all(payload.as_bytes()).await?;
        writer.flush().await?;

        // MPV may emit async events before our response — skip them.
        timeout(Duration::from_secs(3), async {
            let mut reader = BufReader::new(reader);
            let mut line = String::new();
            loop {
                line.clear();
                if reader.read_line(&mut line).await? == 0 {
                    anyhow::bail!("MPV IPC: connection closed before response");
                }
                if let Ok(json) = serde_json::from_str::<Value>(line.trim()) {
                    if json.get("request_id").and_then(|v| v.as_i64()) == Some(1) {
                        return Ok(json);
                    }
                }
            }
        })
        .await
        .map_err(|_| anyhow::anyhow!("MPV IPC: response timeout"))?
    }

    // ── Playback control ─────────────────────────────────────────────────────

    pub async fn load_file(&self, url: &str) -> anyhow::Result<()> {
        self.send_ipc(vec![
            Value::String("loadfile".into()),
            Value::String(url.into()),
            Value::String("replace".into()),
        ])
        .await?;
        Ok(())
    }

    pub async fn stop(&self) -> anyhow::Result<()> {
        self.send_ipc(vec![Value::String("stop".into())]).await?;
        Ok(())
    }

    pub async fn set_pause(&self, paused: bool) -> anyhow::Result<()> {
        self.send_ipc(vec![
            Value::String("set_property".into()),
            Value::String("pause".into()),
            Value::Bool(paused),
        ])
        .await?;
        Ok(())
    }

    pub async fn seek(&self, seconds: f64) -> anyhow::Result<()> {
        self.send_ipc(vec![
            Value::String("seek".into()),
            Value::Number(serde_json::Number::from_f64(seconds).unwrap()),
            Value::String("absolute".into()),
        ])
        .await?;
        Ok(())
    }

    pub async fn set_volume(&self, volume_0_to_1: f64) -> anyhow::Result<()> {
        self.send_ipc(vec![
            Value::String("set_property".into()),
            Value::String("volume".into()),
            Value::Number(serde_json::Number::from_f64(volume_0_to_1 * 100.0).unwrap()),
        ])
        .await?;
        Ok(())
    }

    pub async fn get_status(&self) -> anyhow::Result<MpvStatus> {
        let (pos, dur, paused) = tokio::join!(
            self.get_f64("time-pos"),
            self.get_f64("duration"),
            self.get_bool("pause"),
        );
        Ok(MpvStatus {
            position: pos.unwrap_or(0.0),
            duration: dur.unwrap_or(0.0),
            paused:   paused.unwrap_or(true),
        })
    }

    async fn get_f64(&self, prop: &str) -> anyhow::Result<f64> {
        let resp = self
            .send_ipc(vec![
                Value::String("get_property".into()),
                Value::String(prop.into()),
            ])
            .await?;
        resp["data"]
            .as_f64()
            .ok_or_else(|| anyhow::anyhow!("property {prop} is not a number"))
    }

    async fn get_bool(&self, prop: &str) -> anyhow::Result<bool> {
        let resp = self
            .send_ipc(vec![
                Value::String("get_property".into()),
                Value::String(prop.into()),
            ])
            .await?;
        Ok(resp["data"].as_bool().unwrap_or(false))
    }
}

// ── Public types ──────────────────────────────────────────────────────────────

#[derive(serde::Serialize)]
pub struct MpvStatus {
    pub position: f64,
    pub duration: f64,
    pub paused:   bool,
}
