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
    backend: CredentialBackendKind,
}

impl CredentialStore {
    pub fn new(config_dir: &Path) -> Self {
        let file = FileCredentialBackend::new(config_dir);
        let backend = match std::env::var("NEXTCLOUD_CLI_KEYRING_BACKEND") {
            Ok(value) if value.eq_ignore_ascii_case("file") => CredentialBackendKind::File(file),
            Ok(value) if value.eq_ignore_ascii_case("keyring") => {
                CredentialBackendKind::Keyring(KeyringCredentialBackend::new(file, false))
            }
            _ => CredentialBackendKind::Keyring(KeyringCredentialBackend::new(file, true)),
        };
        Self { backend }
    }

    pub fn path(&self) -> &Path {
        self.backend.path()
    }

    pub fn backend_name(&self) -> &'static str {
        self.backend.name()
    }

    pub fn put_app_password(
        &self,
        credential: &CredentialRef,
        username: &str,
        app_password: &str,
    ) -> CliResult<()> {
        self.backend
            .put_app_password(credential, username, app_password)
    }

    pub fn get_app_password(&self, credential: &CredentialRef) -> CliResult<String> {
        self.backend.get_app_password(credential)
    }

    pub fn has_credential(&self, credential: &CredentialRef) -> CliResult<bool> {
        self.backend.has_credential(credential)
    }
}

#[derive(Debug, Clone)]
enum CredentialBackendKind {
    File(FileCredentialBackend),
    Keyring(KeyringCredentialBackend),
}

impl CredentialBackendKind {
    fn name(&self) -> &'static str {
        match self {
            Self::File(backend) => backend.name(),
            Self::Keyring(backend) => backend.name(),
        }
    }

    fn path(&self) -> &Path {
        match self {
            Self::File(backend) => backend.path(),
            Self::Keyring(backend) => backend.path(),
        }
    }

    fn put_app_password(
        &self,
        credential: &CredentialRef,
        username: &str,
        app_password: &str,
    ) -> CliResult<()> {
        match self {
            Self::File(backend) => backend.put_app_password(credential, username, app_password),
            Self::Keyring(backend) => backend.put_app_password(credential, username, app_password),
        }
    }

    fn get_app_password(&self, credential: &CredentialRef) -> CliResult<String> {
        match self {
            Self::File(backend) => backend.get_app_password(credential),
            Self::Keyring(backend) => backend.get_app_password(credential),
        }
    }

    fn has_credential(&self, credential: &CredentialRef) -> CliResult<bool> {
        match self {
            Self::File(backend) => backend.has_credential(credential),
            Self::Keyring(backend) => backend.has_credential(credential),
        }
    }
}

trait CredentialBackend {
    fn name(&self) -> &'static str;
    fn path(&self) -> &Path;
    fn put_app_password(
        &self,
        credential: &CredentialRef,
        username: &str,
        app_password: &str,
    ) -> CliResult<()>;
    fn get_app_password(&self, credential: &CredentialRef) -> CliResult<String>;
    fn has_credential(&self, credential: &CredentialRef) -> CliResult<bool>;
}

#[derive(Debug, Clone)]
struct KeyringCredentialBackend {
    file_fallback: FileCredentialBackend,
    allow_fallback: bool,
}

impl KeyringCredentialBackend {
    fn new(file_fallback: FileCredentialBackend, allow_fallback: bool) -> Self {
        Self {
            file_fallback,
            allow_fallback,
        }
    }

    fn entry(&self, credential: &CredentialRef) -> CliResult<keyring::Entry> {
        keyring::Entry::new(&credential.service, &credential.id)
            .map_err(|source| CliError::Keyring(source.to_string()))
    }
}

impl CredentialBackend for KeyringCredentialBackend {
    fn name(&self) -> &'static str {
        if self.allow_fallback {
            "keyring-auto"
        } else {
            "keyring"
        }
    }

    fn path(&self) -> &Path {
        self.file_fallback.path()
    }

    fn put_app_password(
        &self,
        credential: &CredentialRef,
        username: &str,
        app_password: &str,
    ) -> CliResult<()> {
        match self.entry(credential).and_then(|entry| {
            entry
                .set_password(app_password)
                .map_err(|source| CliError::Keyring(source.to_string()))
        }) {
            Ok(()) => Ok(()),
            Err(error) if self.allow_fallback => {
                tracing::warn!(error = %error, "falling back to local credential file");
                self.file_fallback
                    .put_app_password(credential, username, app_password)
            }
            Err(error) => Err(error),
        }
    }

    fn get_app_password(&self, credential: &CredentialRef) -> CliResult<String> {
        match self.entry(credential).and_then(|entry| {
            entry
                .get_password()
                .map_err(|source| CliError::Keyring(source.to_string()))
        }) {
            Ok(password) => Ok(password),
            Err(error) if self.allow_fallback => {
                tracing::debug!(error = %error, "falling back to local credential file");
                self.file_fallback.get_app_password(credential)
            }
            Err(error) => Err(error),
        }
    }

    fn has_credential(&self, credential: &CredentialRef) -> CliResult<bool> {
        match self.get_app_password(credential) {
            Ok(_) => Ok(true),
            Err(CliError::Core(CoreError::CredentialNotFound { .. })) => Ok(false),
            Err(CliError::Keyring(_)) if self.allow_fallback => {
                self.file_fallback.has_credential(credential)
            }
            Err(error) => Err(error),
        }
    }
}

#[derive(Debug, Clone)]
struct FileCredentialBackend {
    path: PathBuf,
}

impl FileCredentialBackend {
    fn new(config_dir: &Path) -> Self {
        Self {
            path: config_dir.join(CREDENTIAL_FILE_NAME),
        }
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

impl CredentialBackend for FileCredentialBackend {
    fn name(&self) -> &'static str {
        "local-file-0600"
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn put_app_password(
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

    fn get_app_password(&self, credential: &CredentialRef) -> CliResult<String> {
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

    fn has_credential(&self, credential: &CredentialRef) -> CliResult<bool> {
        let file = self.load()?;
        Ok(file.credentials.contains_key(&credential.id))
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
