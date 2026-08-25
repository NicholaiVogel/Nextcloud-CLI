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

    #[error("failed to read CA bundle {path}: {source}")]
    TlsCaBundleRead {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("invalid CA bundle {path}: {message}")]
    TlsCaBundleInvalid { path: PathBuf, message: String },

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

    #[error("OCS request failed with status {status_code}: {message}")]
    OcsStatus {
        status: String,
        status_code: i64,
        message: String,
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
            Self::TlsCaBundleRead { .. } => "tls_ca_bundle_read_failed",
            Self::TlsCaBundleInvalid { .. } => "tls_ca_bundle_invalid",
            Self::ProfileNotFound { .. } => "profile_not_found",
            Self::NoProfileSelected => "no_profile_selected",
            Self::CredentialNotFound { .. } => "credential_not_found",
            Self::InvalidRemotePath { .. } => "invalid_remote_path",
            Self::Http(error) => classify_tls_error(error).code(),
            Self::HttpStatus { .. } => "http_status_failed",
            Self::OcsStatus { .. } => "ocs_status_failed",
            Self::DownloadSizeMismatch { .. } => "download_size_mismatch",
        }
    }

    /// Returns an actionable, secret-free hint for errors that benefit from
    /// operator guidance. The raw reqwest error is intentionally not exposed
    /// here because it may contain a server URL or other transport detail.
    pub fn hint(&self) -> Option<&'static str> {
        match self {
            Self::Http(error) => Some(classify_tls_error(error).hint()),
            _ => None,
        }
    }

    /// Returns the stable user-facing message for transport failures without
    /// echoing reqwest's potentially sensitive error text.
    pub fn safe_message(&self) -> String {
        match self {
            Self::Http(error) => classify_tls_error(error).message().to_owned(),
            _ => self.to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TlsErrorKind {
    CertificateUntrusted,
    HostnameMismatch,
    HandshakeFailed,
    Other,
}

impl TlsErrorKind {
    fn code(self) -> &'static str {
        match self {
            Self::CertificateUntrusted => "tls_certificate_untrusted",
            Self::HostnameMismatch => "tls_hostname_mismatch",
            Self::HandshakeFailed => "tls_handshake_failed",
            Self::Other => "http_request_failed",
        }
    }

    fn message(self) -> &'static str {
        match self {
            Self::CertificateUntrusted => {
                "TLS certificate verification failed because the issuer is not trusted"
            }
            Self::HostnameMismatch => "TLS hostname verification failed for the server certificate",
            Self::HandshakeFailed => "TLS handshake failed while connecting to the server",
            Self::Other => "network request failed",
        }
    }

    fn hint(self) -> &'static str {
        match self {
            Self::CertificateUntrusted => {
                "Provide the issuing CA with --ca-bundle <path> or NEXTCLOUD_CLI_CA_BUNDLE; do not use --insecure unless you explicitly accept disabling TLS verification."
            }
            Self::HostnameMismatch => {
                "Use a server URL whose hostname appears in the certificate SAN; a custom CA bundle does not disable hostname verification."
            }
            Self::HandshakeFailed => {
                "Check the server certificate and TLS configuration; use --ca-bundle <path> for a private CA."
            }
            Self::Other => "Check the server URL, network, proxy, and TLS configuration.",
        }
    }
}

fn classify_tls_error(error: &reqwest::Error) -> TlsErrorKind {
    let message = error.to_string().to_ascii_lowercase();
    classify_tls_error_message(&message)
}

fn classify_tls_error_message(message: &str) -> TlsErrorKind {
    let hostname_mismatch = [
        "hostname mismatch",
        "certificate name mismatch",
        "not valid for",
        "notvalidforname",
        "not valid for name",
        "no matching subject alternative name",
        "doesn't match certificate",
        "does not match certificate",
    ];
    if hostname_mismatch
        .iter()
        .any(|needle| message.contains(needle))
    {
        return TlsErrorKind::HostnameMismatch;
    }

    let untrusted = [
        "unknown issuer",
        "unknownissuer",
        "unknown ca",
        "unknownca",
        "self signed",
        "self-signed",
        "certificate verify failed",
        "certificate not trusted",
        "invalid peer certificate",
        "unable to get local issuer certificate",
        "webpki error",
        "expired",
        "invalid certificate",
        "bad certificate",
    ];
    if untrusted.iter().any(|needle| message.contains(needle)) {
        return TlsErrorKind::CertificateUntrusted;
    }

    let handshake_failure = [
        "tls handshake",
        "tls error",
        "tls alert",
        "ssl handshake",
        "ssl error",
        "handshake",
        "fatal alert",
        "protocol_version",
        "protocol version",
        "wrong version number",
    ];
    if handshake_failure
        .iter()
        .any(|needle| message.contains(needle))
    {
        return TlsErrorKind::HandshakeFailed;
    }

    TlsErrorKind::Other
}

#[cfg(test)]
mod tests {
    use super::{TlsErrorKind, classify_tls_error_message};

    #[test]
    fn classifies_certificate_diagnostics_without_exposing_transport_details() {
        assert_eq!(
            classify_tls_error_message("certificate verify failed: unknown issuer"),
            TlsErrorKind::CertificateUntrusted
        );
        assert_eq!(
            classify_tls_error_message("certificate name mismatch"),
            TlsErrorKind::HostnameMismatch
        );
        assert_eq!(
            classify_tls_error_message("tls handshake failure"),
            TlsErrorKind::HandshakeFailed
        );
        assert_eq!(
            classify_tls_error_message("connection reset by peer"),
            TlsErrorKind::Other
        );
    }
}
