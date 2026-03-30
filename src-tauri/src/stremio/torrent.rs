use std::{collections::HashMap, path::PathBuf, sync::{Arc, OnceLock, RwLock}};

use librqbit::{AddTorrent, AddTorrentOptions, ManagedTorrent, Session, SessionOptions};

pub struct TorrentManager {
    pub session: Arc<Session>,
    pub cache_dir: PathBuf,
    pub handles: RwLock<HashMap<String, Arc<ManagedTorrent>>>,
}

pub static TORRENT_MANAGER: OnceLock<Arc<TorrentManager>> = OnceLock::new();

impl TorrentManager {
    pub async fn new(cache_dir: PathBuf) -> anyhow::Result<Self> {
        let session = Session::new_with_opts(
            cache_dir.clone(),
            SessionOptions {
                // Force sequential piece picking for streaming
                ..Default::default()
            },
        )
        .await?;

        Ok(Self {
            session: session,
            cache_dir,
            handles: RwLock::new(HashMap::new()),
        })
    }

    pub async fn stream_torrent(&self, info_hash: &str, file_idx: usize) -> anyhow::Result<Arc<ManagedTorrent>> {
        if let Some(handle) = self.handles.read().unwrap().get(info_hash) {
            return Ok(handle.clone());
        }

        let magnet = format!("magnet:?xt=urn:btih:{}", info_hash);

        let response = self
            .session
            .add_torrent(
                AddTorrent::from_url(magnet),
                Some(AddTorrentOptions {
                    paused: false,
                    list_only: false,
                    // Only download the specific file that's being played; this focuses
                    // piece selection on the right file and avoids wasting bandwidth.
                    only_files: Some(vec![file_idx]),
                    overwrite: true,
                    ..Default::default()
                })
            )
            .await?;

        let handle = response
            .into_handle()
            .ok_or_else(|| anyhow::anyhow!("Failed to get torrent handle"))?;

        self.handles.write().unwrap().insert(info_hash.to_string(), handle.clone());

        Ok(handle)
    }
}

pub fn largest_file_id(handle: &Arc<ManagedTorrent>) -> anyhow::Result<usize> {
    handle.with_metadata(|meta| {
        meta.file_infos
            .iter()
            .enumerate()
            .max_by_key(|(_, fi)| fi.len)
            .map(|(i, _)| i)
            .unwrap_or(0)
    })
}

pub fn file_name(handle: &Arc<ManagedTorrent>, file_id: usize) -> anyhow::Result<String> {
    handle.with_metadata(|meta| {
        match &meta.info.files {
            Some(files) => files
                .get(file_id)
                .map(|f| {
                    let segments: Vec<String> = f.path.iter()
                        .map(|b| String::from_utf8_lossy(&b[..]).to_string())
                        .collect();
                    segments.join("/")
                })
                .unwrap_or_else(|| format!("file_{file_id}")),
            None => meta
                .name
                .clone()
                .unwrap_or_else(|| format!("file_{file_id}")),
        }
    })
}
