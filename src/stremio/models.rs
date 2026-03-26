use serde::{Deserialize, Serialize};

#[derive(Serialize, Debug, Deserialize, Clone)]
pub struct Manifest {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub logo: String,
    #[serde(default)]
    pub types: Vec<String>,
    #[serde(default)]
    pub resources: Vec<ResourceDescriptor>,
    #[serde(default)]
    pub catalogs: Vec<CatalogDescriptor>,
}

impl Manifest {
    pub fn supports_resource(&self, resource_name: &str, item_type: &str) -> bool {
        for resource in &self.resources {
            match resource {
                ResourceDescriptor::Short(name) => {
                    if name == resource_name && self.types.contains(&item_type.to_string()) {
                        return true;
                    }
                }
                ResourceDescriptor::Full { name, types, .. } => {
                    if name == resource_name {
                        if let Some(specific_types) = types {
                            if specific_types.contains(&item_type.to_string()) {
                                return true;
                            }
                        } else if self.types.contains(&item_type.to_string()) {
                            return true;
                        }
                    }
                }
            }
        }

        false
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum ResourceDescriptor {
    Short(String),
    Full {
        name: String,
        types: Option<Vec<String>>,
        #[serde(rename = "idPrefixes")]
        id_prefixes: Option<Vec<String>>,
    },
}

impl ResourceDescriptor {
    pub fn name(&self) -> &str {
        match self {
            ResourceDescriptor::Short(name) => name,
            ResourceDescriptor::Full { name, .. } => name,
        }
    }
}

#[derive(Serialize, Debug, Deserialize, Clone)]
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
    pub description: String,
    #[serde(rename = "type")]
    pub item_type: String,
    pub year: String,
    pub runtime: String,
    #[serde(rename = "cast")]
    pub casts: Vec<String>,
    #[serde(rename = "genre")]
    pub genres: Vec<String>,
    pub poster: String,
    pub background: String,
    pub logo: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct StreamResponse {
    pub streams: Vec<Stream>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Stream {
    pub name: Option<String>,
    pub title: Option<String>,
    pub url: Option<String>,
    pub info_hash: Option<String>,
    pub file_idx: Option<u32>,
}
