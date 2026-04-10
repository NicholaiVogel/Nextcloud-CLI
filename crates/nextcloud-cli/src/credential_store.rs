use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use nextcloud::{CredentialRef, Error as CoreError};
use serde::{Deserialize, Serialize};

use crate::error::{CliError, CliResult};

const CREDENTIAL_FILE_NAME: &str = "credentials.json";

#[derive(Debug, Clone)]
pub struct CredentialStore {
    path: PathBuf,
}

impl CredentialStore {
    pub fn new(config_dir: &Path) -> Self {
        Self {
            path: config_dir.join(CREDENTIAL_FILE_NAME),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn put_app_password(
        &self,
        credential: &CredentialRef,
        username: &str,
        app_password: &str,
    ) -> CliResult<()> {
        let mut file = self.load()?;
        let now = Utc::now();
        let existing_created_at = file
            .credentials
            .get(&credential.id)
            .map(|item| item.created_at)
            .unwrap_or(now);
        file.credentials.insert(
            credential.id.clone(),
            StoredCredential {
                id: credential.id.clone(),
                service: credential.service.clone(),
                username: username.to_owned(),
                app_password: app_password.to_owned(),
                created_at: existing_created_at,
                updated_at: now,
            },
        );
        self.save(&file)
    }

    pub fn get_app_password(&self, credential: &CredentialRef) -> CliResult<String> {
        let file = self.load()?;
        file.credentials
            .get(&credential.id)
            .map(|item| item.app_password.clone())
            .ok_or_else(|| {
                CoreError::CredentialNotFound {
                    id: credential.id.clone(),
                }
                .into()
            })
    }

    pub fn has_credential(&self, credential: &CredentialRef) -> CliResult<bool> {
        let file = self.load()?;
        Ok(file.credentials.contains_key(&credential.id))
    }

    fn load(&self) -> CliResult<CredentialFile> {
        if !self.path.exists() {
            return Ok(CredentialFile::default());
        }
        let raw = fs::read_to_string(&self.path).map_err(|source| CliError::CredentialRead {
            path: self.path.clone(),
            source,
        })?;
        serde_json::from_str(&raw).map_err(|source| CliError::CredentialParse {
            path: self.path.clone(),
            source,
        })
    }

    fn save(&self, file: &CredentialFile) -> CliResult<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|source| CliError::CredentialWrite {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        let raw = serde_json::to_string_pretty(file)?;
        fs::write(&self.path, format!("{raw}\n")).map_err(|source| CliError::CredentialWrite {
            path: self.path.clone(),
            source,
        })?;
        set_owner_only_permissions(&self.path)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct CredentialFile {
    #[serde(default)]
    credentials: BTreeMap<String, StoredCredential>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredCredential {
    id: String,
    service: String,
    username: String,
    app_password: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[cfg(unix)]
fn set_owner_only_permissions(path: &Path) -> CliResult<()> {
    use std::os::unix::fs::PermissionsExt;

    let permissions = fs::Permissions::from_mode(0o600);
    fs::set_permissions(path, permissions).map_err(|source| CliError::CredentialWrite {
        path: path.to_path_buf(),
        source,
    })
}

#[cfg(not(unix))]
fn set_owner_only_permissions(_path: &Path) -> CliResult<()> {
    Ok(())
}
