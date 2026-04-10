use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CliError {
    #[error(transparent)]
    Core(#[from] nextcloud::Error),

    #[error("app password is required; pass --app-password or set NEXTCLOUD_APP_PASSWORD")]
    MissingAppPassword,

    #[error("failed to read credentials from {path}: {source}")]
    CredentialRead {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to write credentials to {path}: {source}")]
    CredentialWrite {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to parse credentials from {path}: {source}")]
    CredentialParse {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    #[error("failed to serialize response JSON: {0}")]
    SerializeResponse(#[from] serde_json::Error),
}

impl CliError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Core(error) => error.code(),
            Self::MissingAppPassword => "missing_app_password",
            Self::CredentialRead { .. } => "credential_read_failed",
            Self::CredentialWrite { .. } => "credential_write_failed",
            Self::CredentialParse { .. } => "credential_parse_failed",
            Self::SerializeResponse(_) => "serialize_response_failed",
        }
    }

    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Core(nextcloud::Error::NoProfileSelected) => 2,
            Self::Core(nextcloud::Error::ProfileNotFound { .. }) => 2,
            Self::Core(nextcloud::Error::CredentialNotFound { .. }) => 2,
            Self::MissingAppPassword => 2,
            Self::Core(nextcloud::Error::HttpStatus { .. }) => 3,
            Self::Core(nextcloud::Error::Http(_)) => 3,
            _ => 1,
        }
    }
}

pub type CliResult<T> = std::result::Result<T, CliError>;
