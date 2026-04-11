use std::path::PathBuf;

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("could not determine a configuration directory")]
    ConfigDirUnavailable,

    #[error("failed to create directory {path}: {source}")]
    CreateDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to read {path}: {source}")]
    ReadFile {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to write {path}: {source}")]
    WriteFile {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to write response body: {source}")]
    WriteResponse {
        #[source]
        source: std::io::Error,
    },

    #[error("failed to parse JSON from {path}: {source}")]
    ParseJson {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    #[error("failed to parse XML from {context}: {message}")]
    ParseXml { context: String, message: String },

    #[error("failed to serialize JSON for {path}: {source}")]
    SerializeJson {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    #[error("invalid server URL `{value}`: {source}")]
    InvalidServerUrl {
        value: String,
        #[source]
        source: url::ParseError,
    },

    #[error("profile `{name}` was not found")]
    ProfileNotFound { name: String },

    #[error("no profile selected; pass --profile or set a default profile")]
    NoProfileSelected,

    #[error("credential `{id}` was not found")]
    CredentialNotFound { id: String },

    #[error("invalid remote path `{path}`: {reason}")]
    InvalidRemotePath { path: String, reason: String },

    #[error("network request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("server returned HTTP {status}: {body}")]
    HttpStatus {
        status: reqwest::StatusCode,
        body: String,
    },

    #[error("download byte count mismatch: expected {expected} bytes, wrote {actual} bytes")]
    DownloadSizeMismatch { expected: u64, actual: u64 },
}

impl Error {
    pub fn code(&self) -> &'static str {
        match self {
            Self::ConfigDirUnavailable => "config_dir_unavailable",
            Self::CreateDir { .. } => "create_dir_failed",
            Self::ReadFile { .. } => "read_file_failed",
            Self::WriteFile { .. } => "write_file_failed",
            Self::WriteResponse { .. } => "write_response_failed",
            Self::ParseJson { .. } => "parse_json_failed",
            Self::ParseXml { .. } => "parse_xml_failed",
            Self::SerializeJson { .. } => "serialize_json_failed",
            Self::InvalidServerUrl { .. } => "invalid_server_url",
            Self::ProfileNotFound { .. } => "profile_not_found",
            Self::NoProfileSelected => "no_profile_selected",
            Self::CredentialNotFound { .. } => "credential_not_found",
            Self::InvalidRemotePath { .. } => "invalid_remote_path",
            Self::Http(_) => "http_request_failed",
            Self::HttpStatus { .. } => "http_status_failed",
            Self::DownloadSizeMismatch { .. } => "download_size_mismatch",
        }
    }
}
