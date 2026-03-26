use std::hash::DefaultHasher;
use std::hash::{Hash, Hasher};
use directories::ProjectDirs;

fn is_valid_image(bytes: &[u8]) -> bool {
    if bytes.len() < 12 {
        return false;
    }

    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) { return true; }
    if bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) { return true; }
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") { return true; }
    if &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" { return true; }

    false
}

pub async fn fetch_or_cache_image(url: String) -> Option<Vec<u8>> {
    let mut hasher = DefaultHasher::new();
    url.hash(&mut hasher);
    let hash = hasher.finish();

    let proj_dirs = ProjectDirs::from("com", "fy", "streamix").unwrap();
    let cache_dir = proj_dirs.cache_dir().join("images");
    let _ = std::fs::create_dir_all(&cache_dir);

    let file_path = cache_dir.join(format!("{}.jpg", hash));

    if file_path.exists() {
        if let Ok(bytes) = std::fs::read(&file_path) {
            return Some(bytes);
        }
    }

    if let Ok(response) = reqwest::get(&url).await {
        if let Ok(bytes) = response.bytes().await {
            let vec_bytes = bytes.to_vec();

            if is_valid_image(&vec_bytes) {
                if let Err(e) = std::fs::write(&file_path, &vec_bytes) {
                    eprintln!("Failed to write cache file: {}", e);
                }

                return Some(vec_bytes);
            } else {
                eprintln!("Invalid image data from URL: {}", url);
            }
        }
    }

    None
}
