use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::error::{Error, Result};

const CONFIG_FILE_NAME: &str = "config.json";
const CACHE_DIR_NAME: &str = "cache";
const AUDIT_DIR_NAME: &str = "audit";
const CREDENTIAL_SERVICE: &str = "nextcloud-cli";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConfigPaths {
    pub config_dir: PathBuf,
    pub config_file: PathBuf,
    pub cache_dir: PathBuf,
    pub audit_dir: PathBuf,
}

impl ConfigPaths {
    pub fn from_override(config_dir: Option<PathBuf>) -> Result<Self> {
        let config_dir = match config_dir {
            Some(path) => path,
            None => dirs::config_dir()
                .map(|path| path.join("nextcloud-cli"))
                .ok_or(Error::ConfigDirUnavailable)?,
        };

        Ok(Self {
            config_file: config_dir.join(CONFIG_FILE_NAME),
            cache_dir: config_dir.join(CACHE_DIR_NAME),
            audit_dir: config_dir.join(AUDIT_DIR_NAME),
            config_dir,
        })
    }

    pub fn ensure(&self) -> Result<()> {
        create_dir(&self.config_dir)?;
        create_dir(&self.cache_dir)?;
        create_dir(&self.audit_dir)?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ConfigStore {
    paths: ConfigPaths,
}

impl ConfigStore {
    pub fn new(paths: ConfigPaths) -> Self {
        Self { paths }
    }

    pub fn paths(&self) -> &ConfigPaths {
        &self.paths
    }

    pub fn load(&self) -> Result<CliConfig> {
        self.paths.ensure()?;
        if !self.paths.config_file.exists() {
            return Ok(CliConfig::default());
        }

        let raw =
            fs::read_to_string(&self.paths.config_file).map_err(|source| Error::ReadFile {
                path: self.paths.config_file.clone(),
                source,
            })?;

        serde_json::from_str(&raw).map_err(|source| Error::ParseJson {
            path: self.paths.config_file.clone(),
            source,
        })
    }

    pub fn save(&self, config: &CliConfig) -> Result<()> {
        self.paths.ensure()?;
        let raw = serde_json::to_string_pretty(config).map_err(|source| Error::SerializeJson {
            path: self.paths.config_file.clone(),
            source,
        })?;
        fs::write(&self.paths.config_file, format!("{raw}\n")).map_err(|source| {
            Error::WriteFile {
                path: self.paths.config_file.clone(),
                source,
            }
        })?;
        set_owner_only_permissions(&self.paths.config_file)?;
        Ok(())
    }

    pub fn selected_profile(&self, requested: Option<&str>) -> Result<Profile> {
        let config = self.load()?;
        config.selected_profile(requested).cloned()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CliConfig {
    pub schema_version: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_profile: Option<String>,
    #[serde(default)]
    pub profiles: BTreeMap<String, Profile>,
}

impl Default for CliConfig {
    fn default() -> Self {
        Self {
            schema_version: 1,
            default_profile: None,
            profiles: BTreeMap::new(),
        }
    }
}

impl CliConfig {
    pub fn selected_profile(&self, requested: Option<&str>) -> Result<&Profile> {
        let name = requested
            .map(str::to_owned)
            .or_else(|| self.default_profile.clone())
            .ok_or(Error::NoProfileSelected)?;

        self.profiles
            .get(&name)
            .ok_or(Error::ProfileNotFound { name })
    }

    pub fn upsert_profile(&mut self, profile: Profile, set_default: bool) {
        let name = profile.name.clone();
        self.profiles.insert(name.clone(), profile);
        if set_default || self.default_profile.is_none() {
            self.default_profile = Some(name);
        }
    }

    pub fn set_default_profile(&mut self, name: &str) -> Result<()> {
        if !self.profiles.contains_key(name) {
            return Err(Error::ProfileNotFound {
                name: name.to_owned(),
            });
        }
        self.default_profile = Some(name.to_owned());
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Profile {
    pub name: String,
    pub server: Url,
    pub username: String,
    pub credential: CredentialRef,
    pub policy: ProfilePolicy,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Profile {
    pub fn new(name: String, server: Url, username: String) -> Self {
        let now = Utc::now();
        let credential = CredentialRef::for_profile(&name);
        Self {
            name,
            server,
            username,
            credential,
            policy: ProfilePolicy::default(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn parse_server(value: &str) -> Result<Url> {
        let mut url = Url::parse(value).map_err(|source| Error::InvalidServerUrl {
            value: value.to_owned(),
            source,
        })?;
        if url.scheme() != "https" && url.scheme() != "http" {
            return Err(Error::InvalidServerUrl {
                value: value.to_owned(),
                source: url::ParseError::RelativeUrlWithoutBase,
            });
        }
        url.set_query(None);
        url.set_fragment(None);
        Ok(url)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CredentialRef {
    pub service: String,
    pub id: String,
}

impl CredentialRef {
    pub fn for_profile(profile_name: &str) -> Self {
        Self {
            service: CREDENTIAL_SERVICE.to_owned(),
            id: format!("profile:{profile_name}:app-password"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProfilePolicy {
    pub agent_mode: bool,
    pub default_dry_run: bool,
    pub allow_destructive: bool,
    pub allow_public_shares: bool,
}

impl Default for ProfilePolicy {
    fn default() -> Self {
        Self {
            agent_mode: false,
            default_dry_run: true,
            allow_destructive: false,
            allow_public_shares: false,
        }
    }
}

fn create_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path).map_err(|source| Error::CreateDir {
        path: path.to_path_buf(),
        source,
    })
}

#[cfg(unix)]
fn set_owner_only_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let permissions = fs::Permissions::from_mode(0o600);
    fs::set_permissions(path, permissions).map_err(|source| Error::WriteFile {
        path: path.to_path_buf(),
        source,
    })
}

#[cfg(not(unix))]
fn set_owner_only_permissions(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_schema_version() -> Result<()> {
        let config = CliConfig::default();
        assert_eq!(config.schema_version, 1);
        assert!(config.profiles.is_empty());
        Ok(())
    }

    #[test]
    fn profile_server_strips_query_and_fragment() -> Result<()> {
        let url = Profile::parse_server("https://cloud.example.com/index.php?x=1#frag")?;
        assert_eq!(url.as_str(), "https://cloud.example.com/index.php");
        Ok(())
    }
}
