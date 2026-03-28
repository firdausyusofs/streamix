use std::{fs, path::PathBuf};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use super::models::Manifest;

const CONFIG_FILE: &str = "addons.json";

const DEFAULT_ADDONS: &[&str] = &["https://v3-cinemeta.strem.io/manifest.json"];

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InstalledAddon {
    pub transport_url: String,
    pub manifest: Manifest,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct AddonConfig {
    pub addons: Vec<InstalledAddon>,
}

fn get_config_path() -> PathBuf {
    if let Some(proj_dirs) = ProjectDirs::from("com", "fy", "streamix") {
        let config_dir = proj_dirs.config_dir();
        if !config_dir.exists() {
            if let Err(e) = fs::create_dir_all(config_dir) {
                eprintln!("Failed to create config directory: {}", e);
            }
        }
        return config_dir.join(CONFIG_FILE);
    } else {
        PathBuf::from(CONFIG_FILE)
    }
}

pub fn load_addons() -> AddonConfig {
    let config_file_path = get_config_path();

    if let Ok(data) = fs::read_to_string(config_file_path) {
        if let Ok(config) = serde_json::from_str(&data) {
            return config;
        }
    }

    AddonConfig::default()
}

pub fn save_addons(config: &AddonConfig) {
    let config_file_path = get_config_path();

    if let Ok(data) = serde_json::to_string_pretty(config) {
        if let Err(e) = fs::write(config_file_path, data) {
            eprintln!("Failed to save addons: {}", e);
        }
    }
}

pub async fn init_addons() -> AddonConfig {
    let mut config = load_addons();

    if !config.addons.is_empty() {
        return config;
    }

    for &url in DEFAULT_ADDONS {
        if let Ok(manifest) = super::client::fetch_manifest(url).await {
            config.addons.push(InstalledAddon {
                transport_url: url.to_string(),
                manifest,
            });
        } else {
            eprintln!("Failed to fetch manifest from URL: {}", url);
        }
    }

    save_addons(&config);

    config
}
