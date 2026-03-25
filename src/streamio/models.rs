use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Manifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub catalogs: Vec<CatalogDescriptor>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CatalogDescriptor {
    #[serde(rename = "type")]
    pub item_type: String,
    pub id: String,
    pub name: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CatalogResponse {
    pub metas: Vec<MetaPreview>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MetaPreview {
    pub id: String,
    pub name: String,
    pub poster: String,
    #[serde(rename = "type")]
    pub item_type: String,
}
