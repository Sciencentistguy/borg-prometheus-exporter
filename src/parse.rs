use serde::de::IgnoredAny;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct BorgResponse {
    pub cache: BorgResponseCache,
    encryption: IgnoredAny,
    pub repository: BorgResponseRepository,
    security_dir: IgnoredAny,
}

#[derive(Debug, Deserialize)]
pub struct BorgResponseCache {
    path: IgnoredAny,
    pub stats: BorgResponseCacheStats,
}

#[derive(Debug, Deserialize)]
pub struct BorgResponseCacheStats {
    pub total_chunks: usize,
    pub total_csize: usize,
    pub total_size: usize,
    pub total_unique_chunks: usize,
    pub unique_csize: usize,
    pub unique_size: usize,
}

#[derive(Debug, Deserialize)]
pub struct BorgResponseRepository {
    id: IgnoredAny,
    pub last_modified: String,
    location: IgnoredAny,
}
