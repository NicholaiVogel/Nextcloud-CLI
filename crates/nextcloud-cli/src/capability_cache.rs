use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Duration, Utc};
use nextcloud::ServerCapabilities;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::error::{CliError, CliResult};

const CAPABILITY_CACHE_DIR: &str = "capabilities";
const DEFAULT_TTL_SECONDS: i64 = 3600;

#[derive(Debug, Clone)]
pub struct CapabilityCache {
    dir: PathBuf,
    ttl: Duration,
}

impl CapabilityCache {
    pub fn new(cache_dir: &Path) -> Self {
        Self {
            dir: cache_dir.join(CAPABILITY_CACHE_DIR),
            ttl: Duration::seconds(DEFAULT_TTL_SECONDS),
        }
    }

    pub fn ttl_seconds(&self) -> i64 {
        self.ttl.num_seconds()
    }

    pub fn load_fresh(&self, profile: &str, server: &Url) -> CliResult<Option<CachedCapabilities>> {
        let Some(cached) = self.load(profile)? else {
            return Ok(None);
        };
        if cached.server != server.as_str() {
            return Ok(None);
        }
        if Utc::now() - cached.cached_at > self.ttl {
            return Ok(None);
        }
        Ok(Some(cached))
    }

    pub fn save(
        &self,
        profile: &str,
        server: &Url,
        capabilities: ServerCapabilities,
    ) -> CliResult<CachedCapabilities> {
        fs::create_dir_all(&self.dir).map_err(|source| CliError::CacheWrite {
            path: self.dir.clone(),
            source,
        })?;
        let cached = CachedCapabilities {
            profile: profile.to_owned(),
            server: server.to_string(),
            cached_at: Utc::now(),
            ttl_seconds: self.ttl_seconds(),
            capabilities,
        };
        let path = self.path_for(profile);
        let raw = serde_json::to_string_pretty(&cached)?;
        fs::write(&path, format!("{raw}\n"))
            .map_err(|source| CliError::CacheWrite { path, source })?;
        Ok(cached)
    }

    pub fn invalidate(&self, profile: &str) -> CliResult<bool> {
        let path = self.path_for(profile);
        if !path.exists() {
            return Ok(false);
        }
        fs::remove_file(&path).map_err(|source| CliError::CacheWrite { path, source })?;
        Ok(true)
    }

    fn load(&self, profile: &str) -> CliResult<Option<CachedCapabilities>> {
        let path = self.path_for(profile);
        if !path.exists() {
            return Ok(None);
        }
        let raw = fs::read_to_string(&path).map_err(|source| CliError::CacheRead {
            path: path.clone(),
            source,
        })?;
        let cached =
            serde_json::from_str(&raw).map_err(|source| CliError::CacheParse { path, source })?;
        Ok(Some(cached))
    }

    fn path_for(&self, profile: &str) -> PathBuf {
        self.dir.join(format!("{}.json", safe_file_name(profile)))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedCapabilities {
    pub profile: String,
    pub server: String,
    pub cached_at: DateTime<Utc>,
    pub ttl_seconds: i64,
    pub capabilities: ServerCapabilities,
}

fn safe_file_name(value: &str) -> String {
    value
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' => ch,
            _ => '_',
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_file_name_replaces_path_separators() {
        assert_eq!(safe_file_name("personal/main"), "personal_main");
    }
}
