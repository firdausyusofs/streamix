use super::models::{CatalogResponse, Manifest, StreamResponse};
use reqwest::Error;

pub async fn fetch_manifest(manifest_url: &str) -> Result<Manifest, Error> {
    let manifest = reqwest::get(manifest_url)
        .await?
        .json::<Manifest>()
        .await?;
    Ok(manifest)
}

pub async fn fetch_catalog(
    manifest_url: &str,
    item_type: &str,
    catalog_id: &str,
) -> Result<CatalogResponse, Error> {
    let base_url = manifest_url.trim_end_matches("/manifest.json");
    let url = format!("{}/catalog/{}/{}.json", base_url, item_type, catalog_id);
    println!("Fetching catalog from URL: {}", url);

    let response = reqwest::get(&url)
        .await?
        .json::<CatalogResponse>()
        .await?;

    Ok(response)
}

pub async fn fetch_streams(base_url: &str, item_type: &str, id: &str) -> Result<StreamResponse, Error> {
    let clean_base = base_url.trim_end_matches("/manifest.json");
    let url = format!("{}/stream/{}/{}.json", clean_base, item_type, id);
    println!("Fetching streams from URL: {}", url);

    let response = reqwest::get(&url)
        .await?
        .json::<StreamResponse>()
        .await?;

    Ok(response)
}
