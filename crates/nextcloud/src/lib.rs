//! Reusable Nextcloud client primitives for `nextcloud-cli`.
//!
//! The CLI crate owns terminal UX and command parsing. This crate owns stable
//! data models, configuration layout, API clients, and protocol-specific helpers.

pub mod auth;
pub mod capabilities;
pub mod client;
pub mod config_schema;
pub mod error;
pub mod models;
pub mod ocs;
pub mod shares;
pub mod webdav;

pub use auth::{
    AppPasswordClient, AppPasswordCredentials, LoginFlowV2Client, LoginFlowV2Credentials,
    LoginFlowV2Poll, LoginFlowV2Start,
};
pub use capabilities::{CapabilitiesClient, ServerCapabilities, ServerStatus};
pub use client::{ClientAuth, DownloadedBytes, NextcloudClient};
pub use config_schema::{
    CliConfig, ConfigPaths, ConfigStore, CredentialRef, Profile, ProfilePolicy,
};
pub use error::{Error, Result};
pub use ocs::{OcsEnvelope, OcsMeta};
pub use shares::{Share, ShareListOptions, SharesClient};
pub use webdav::{WebDavClient, WebDavEntry};
