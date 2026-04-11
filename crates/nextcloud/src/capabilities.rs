use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::client::NextcloudClient;
use crate::error::Result;
use crate::ocs::OcsEnvelope;

#[derive(Debug, Clone)]
pub struct CapabilitiesClient {
    client: NextcloudClient,
}

impl CapabilitiesClient {
    pub fn new(client: NextcloudClient) -> Self {
        Self { client }
    }

    pub async fn server_status(&self) -> Result<ServerStatus> {
        self.client.get_json("status.php").await
    }

    pub async fn capabilities(&self) -> Result<ServerCapabilities> {
        let data: OcsCapabilitiesData = self
            .client
            .get_ocs_json::<OcsEnvelope<OcsCapabilitiesData>>(
                "ocs/v2.php/cloud/capabilities?format=json",
            )
            .await?
            .into_data()?;
        Ok(ServerCapabilities {
            version: data.version,
            capabilities: data.capabilities,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServerStatus {
    pub installed: bool,
    pub maintenance: bool,
    #[serde(default)]
    pub needs_db_upgrade: bool,
    pub version: String,
    pub versionstring: String,
    #[serde(default)]
    pub edition: String,
    #[serde(default)]
    pub productname: String,
    #[serde(default)]
    pub extended_support: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServerCapabilities {
    pub version: Value,
    pub capabilities: Value,
}

#[derive(Debug, Clone, Deserialize)]
struct OcsCapabilitiesData {
    version: Value,
    capabilities: Value,
}
