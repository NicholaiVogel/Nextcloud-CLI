//! Reusable Nextcloud client primitives for `nextcloud-cli`.
//!
//! The CLI crate owns terminal UX and command parsing. This crate owns stable
//! data models, configuration layout, API clients, and protocol-specific helpers.

pub mod activity;
pub mod auth;
pub mod calendar;
pub mod capabilities;
pub mod client;
pub mod config_schema;
pub mod contacts;
pub mod deck;
pub mod error;
pub mod models;
pub mod notes;
pub mod ocs;
pub mod shares;
pub mod webdav;

pub use activity::{ActivityClient, ActivityItem, ActivityRecentOptions};
pub use auth::{
    AppPasswordClient, AppPasswordCredentials, LoginFlowV2Client, LoginFlowV2Credentials,
    LoginFlowV2Poll, LoginFlowV2Start,
};
pub use calendar::{CalendarClient, CalendarCreateOptions, CalendarEvent, CalendarEventsOptions};
pub use capabilities::{CapabilitiesClient, ServerCapabilities, ServerStatus};
pub use client::{ClientAuth, DownloadedBytes, NextcloudClient};
pub use config_schema::{
    CliConfig, ConfigPaths, ConfigStore, CredentialRef, Profile, ProfilePolicy,
};
pub use contacts::{Contact, ContactCreateOptions, ContactSearchOptions, ContactsClient};
pub use deck::{
    DeckBoard, DeckBoardCreateOptions, DeckCard, DeckCardCreateOptions, DeckCardMoveOptions,
    DeckCardRefOptions, DeckCardUpdateOptions, DeckCardsOptions, DeckClient, DeckStack,
    DeckStackCreateOptions,
};
pub use error::{Error, Result};
pub use notes::{Note, NotesClient, NotesCreateOptions, NotesListOptions, NotesUpdateOptions};
pub use ocs::{OcsEnvelope, OcsMeta};
pub use shares::{Share, ShareCreatePublicOptions, ShareListOptions, SharesClient};
pub use webdav::{WebDavClient, WebDavEntry};
