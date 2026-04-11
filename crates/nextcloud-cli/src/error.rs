use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CliError {
    #[error(transparent)]
    Core(#[from] nextcloud::Error),

    #[error("app password is required; pass --app-password or set NEXTCLOUD_APP_PASSWORD")]
    MissingAppPassword,

    #[error("account password is required; pass --password-stdin or --password-env <NAME>")]
    MissingAccountPassword,

    #[error("password environment variable {name} is not set")]
    PasswordEnvMissing { name: String },

    #[error("failed to read account password from stdin: {0}")]
    PasswordStdinRead(#[source] std::io::Error),

    #[error("local file already exists at {path}; pass --overwrite to replace it")]
    LocalFileExists { path: PathBuf },

    #[error("local upload path {path} is not a file")]
    LocalUploadNotFile { path: PathBuf },

    #[error("remote path {path} already exists; pass --overwrite to replace it")]
    RemotePathExists { path: String },

    #[error("unsupported search mode `{mode}`; supported mode: name")]
    UnsupportedSearchMode { mode: String },

    #[error(
        "destructive command requires confirmation; pass --yes to continue or --dry-run to preview"
    )]
    ConfirmationRequired,

    #[error(
        "sensitive command `{command}` requires confirmation; pass --yes to continue or --dry-run to preview"
    )]
    SensitiveConfirmationRequired { command: String },

    #[error("profile `{profile}` does not allow `{command}`: {policy}")]
    PolicyDenied {
        profile: String,
        command: String,
        policy: String,
    },

    #[error("unsupported share creation mode; currently supported: --public")]
    UnsupportedShareCreateMode,

    #[error("invalid expire date `{value}`; expected YYYY-MM-DD")]
    InvalidExpireDate { value: String },

    #[error("invalid share id `{value}`; share id must not be empty")]
    InvalidShareId { value: String },

    #[error("policy set did not include any changes")]
    NoPolicyChanges,

    #[error("invalid calendar range `{value}`; expected values like 7d")]
    InvalidCalendarRange { value: String },

    #[error("invalid calendar bound `{value}`; expected YYYY-MM-DD or RFC3339 datetime")]
    InvalidCalendarBound { value: String },

    #[error("invalid limit `{value}`; expected a value from 1 to 100")]
    InvalidLimit { value: u32 },

    #[error("calendar event end must be after start")]
    InvalidCalendarEventRange,

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

    #[error("keyring credential backend failed: {0}")]
    Keyring(String),

    #[error("failed to read capability cache from {path}: {source}")]
    CacheRead {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to write capability cache at {path}: {source}")]
    CacheWrite {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to parse capability cache from {path}: {source}")]
    CacheParse {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    #[error("login flow timed out after {timeout_seconds} seconds")]
    LoginTimeout { timeout_seconds: u64 },

    #[error("failed to open browser for login URL {url}: {message}")]
    BrowserOpen { url: String, message: String },

    #[error("failed to serialize response JSON: {0}")]
    SerializeResponse(#[from] serde_json::Error),
}

impl CliError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Core(error) => error.code(),
            Self::MissingAppPassword => "missing_app_password",
            Self::MissingAccountPassword => "missing_account_password",
            Self::PasswordEnvMissing { .. } => "password_env_missing",
            Self::PasswordStdinRead(_) => "password_stdin_read_failed",
            Self::LocalFileExists { .. } => "local_file_exists",
            Self::LocalUploadNotFile { .. } => "local_upload_not_file",
            Self::RemotePathExists { .. } => "remote_path_exists",
            Self::UnsupportedSearchMode { .. } => "unsupported_search_mode",
            Self::ConfirmationRequired => "confirmation_required",
            Self::SensitiveConfirmationRequired { .. } => "confirmation_required",
            Self::PolicyDenied { .. } => "policy_denied",
            Self::UnsupportedShareCreateMode => "unsupported_share_create_mode",
            Self::InvalidExpireDate { .. } => "invalid_expire_date",
            Self::InvalidShareId { .. } => "invalid_share_id",
            Self::NoPolicyChanges => "no_policy_changes",
            Self::InvalidCalendarRange { .. } => "invalid_calendar_range",
            Self::InvalidCalendarBound { .. } => "invalid_calendar_bound",
            Self::InvalidLimit { .. } => "invalid_limit",
            Self::InvalidCalendarEventRange => "invalid_calendar_event_range",
            Self::CredentialRead { .. } => "credential_read_failed",
            Self::CredentialWrite { .. } => "credential_write_failed",
            Self::CredentialParse { .. } => "credential_parse_failed",
            Self::Keyring(_) => "keyring_failed",
            Self::CacheRead { .. } => "cache_read_failed",
            Self::CacheWrite { .. } => "cache_write_failed",
            Self::CacheParse { .. } => "cache_parse_failed",
            Self::LoginTimeout { .. } => "login_timeout",
            Self::BrowserOpen { .. } => "browser_open_failed",
            Self::SerializeResponse(_) => "serialize_response_failed",
        }
    }

    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Core(nextcloud::Error::NoProfileSelected) => 2,
            Self::Core(nextcloud::Error::ProfileNotFound { .. }) => 2,
            Self::Core(nextcloud::Error::CredentialNotFound { .. }) => 2,
            Self::Core(nextcloud::Error::InvalidRemotePath { .. }) => 2,
            Self::MissingAppPassword => 2,
            Self::MissingAccountPassword => 2,
            Self::PasswordEnvMissing { .. } => 2,
            Self::LocalFileExists { .. } => 2,
            Self::LocalUploadNotFile { .. } => 2,
            Self::RemotePathExists { .. } => 2,
            Self::UnsupportedSearchMode { .. } => 2,
            Self::ConfirmationRequired => 2,
            Self::SensitiveConfirmationRequired { .. } => 2,
            Self::PolicyDenied { .. } => 2,
            Self::UnsupportedShareCreateMode => 2,
            Self::InvalidExpireDate { .. } => 2,
            Self::InvalidShareId { .. } => 2,
            Self::NoPolicyChanges => 2,
            Self::InvalidCalendarRange { .. } => 2,
            Self::InvalidCalendarBound { .. } => 2,
            Self::InvalidLimit { .. } => 2,
            Self::InvalidCalendarEventRange => 2,
            Self::LoginTimeout { .. } => 10,
            Self::Core(nextcloud::Error::OcsStatus { .. }) => 6,
            Self::Core(nextcloud::Error::HttpStatus { .. }) => 3,
            Self::Core(nextcloud::Error::Http(_)) => 3,
            _ => 1,
        }
    }
}

pub type CliResult<T> = std::result::Result<T, CliError>;
