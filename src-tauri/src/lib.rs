pub mod stremio;

#[tauri::command]
async fn get_installed_addons() -> Result<stremio::store::AddonConfig, String> {
    let config = stremio::store::init_addons().await;
    Ok(config)
}

#[tauri::command]
async fn fetch_catalog_from_addon(
    manifest_url: String,
    item_type: String,
    catalog_id: String,
) -> Result<stremio::models::CatalogResponse, String> {
    match stremio::client::fetch_catalog(&manifest_url, &item_type, &catalog_id).await {
        Ok(response) => Ok(response),
        Err(e) => Err(format!("Failed to fetch catalog: {}", e)),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_installed_addons,
            fetch_catalog_from_addon
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
