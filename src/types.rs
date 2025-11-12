use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RatatuiDependency {
    pub version: String,
    pub optional: bool,
    pub dev_dependency: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CratePackage {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub created_at: String,
    pub updated_at: String,
    pub downloads: u64,
    pub recent_downloads: u64,
    pub categories: Option<Vec<String>>,
    pub repository: Option<String>,
    pub homepage: Option<String>,
    pub documentation: Option<String>,
    pub ratatui_dependency: RatatuiDependency,
    pub is_core_library: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Statistics {
    pub etag_cache_hits: usize,
    pub etag_cache_misses: usize,
    pub cache_hit_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    pub version: String,
    pub generated_at: String,
    pub total_crates: usize,
    pub core_libraries: usize,
    pub community_packages: usize,
    pub data_sources: Vec<String>,
    pub statistics: Statistics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CratesData {
    pub metadata: Metadata,
    pub crates: Vec<CratePackage>,
}
