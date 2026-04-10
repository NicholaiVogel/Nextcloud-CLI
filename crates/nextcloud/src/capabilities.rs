use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::client::NextcloudClient;
use crate::error::Result;

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
        let envelope: OcsCapabilitiesEnvelope = self
            .client
            .get_ocs_json("ocs/v2.php/cloud/capabilities?format=json")
            .await?;
        Ok(ServerCapabilities {
            version: envelope.ocs.data.version,
            capabilities: envelope.ocs.data.capabilities,
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
struct OcsCapabilitiesEnvelope {
    ocs: OcsCapabilitiesBody,
}

#[derive(Debug, Clone, Deserialize)]
struct OcsCapabilitiesBody {
    data: OcsCapabilitiesData,
}

#[derive(Debug, Clone, Deserialize)]
struct OcsCapabilitiesData {
    version: Value,
    capabilities: Value,
}
