use serde::{Deserialize, Serialize};
use url::Url;

use crate::client::NextcloudClient;
use crate::error::Result;
use crate::ocs::OcsEnvelope;

#[derive(Debug, Clone)]
pub struct AppPasswordClient {
    client: NextcloudClient,
}

impl AppPasswordClient {
    pub fn new(client: NextcloudClient) -> Self {
        Self { client }
    }

    pub async fn create_app_password(&self) -> Result<AppPasswordCredentials> {
        let data: OcsAppPasswordData = self
            .client
            .get_ocs_json::<OcsEnvelope<OcsAppPasswordData>>(
                "ocs/v2.php/core/getapppassword?format=json",
            )
            .await?
            .into_data()?;
        Ok(AppPasswordCredentials {
            app_password: data.app_password,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppPasswordCredentials {
    #[serde(rename = "appPassword", alias = "apppassword")]
    pub app_password: String,
}

#[derive(Debug, Clone)]
pub struct LoginFlowV2Client {
    client: NextcloudClient,
}

impl LoginFlowV2Client {
    pub fn new(client: NextcloudClient) -> Self {
        Self { client }
    }

    pub async fn start(&self) -> Result<LoginFlowV2Start> {
        self.client.post_empty_json("index.php/login/v2").await
    }

    pub async fn poll(&self, poll: &LoginFlowV2Poll) -> Result<LoginFlowV2Credentials> {
        self.client
            .post_form_json(
                poll.endpoint.clone(),
                &[("token".to_string(), poll.token.clone())],
            )
            .await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LoginFlowV2Start {
    pub poll: LoginFlowV2Poll,
    pub login: Url,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LoginFlowV2Poll {
    pub token: String,
    pub endpoint: Url,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LoginFlowV2Credentials {
    pub server: Url,
    #[serde(alias = "loginName")]
    pub login_name: String,
    #[serde(alias = "appPassword")]
    pub app_password: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
struct OcsAppPasswordData {
    #[serde(rename = "apppassword")]
    app_password: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_login_flow_start() -> Result<()> {
        let raw = r#"{
            "poll": {
                "token": "abc123",
                "endpoint": "https://cloud.example.com/index.php/login/v2/poll"
            },
            "login": "https://cloud.example.com/index.php/login/v2/flow/abc123"
        }"#;

        let parsed: LoginFlowV2Start = serde_json::from_str(raw).expect("valid login flow JSON");
        assert_eq!(parsed.poll.token, "abc123");
        assert_eq!(
            parsed.login.as_str(),
            "https://cloud.example.com/index.php/login/v2/flow/abc123"
        );
        Ok(())
    }

    #[test]
    fn parses_poll_credentials_with_nextcloud_login_name_alias() -> Result<()> {
        let raw = r#"{
            "server": "https://cloud.example.com",
            "loginName": "nicholai",
            "appPassword": "secret"
        }"#;

        let parsed: LoginFlowV2Credentials =
            serde_json::from_str(raw).expect("valid credentials JSON");
        assert_eq!(parsed.login_name, "nicholai");
        assert_eq!(parsed.app_password, "secret");
        Ok(())
    }

    #[test]
    fn parses_ocs_app_password_response() -> Result<()> {
        let raw = r#"{
            "ocs": {
                "meta": {
                    "status": "ok",
                    "statuscode": 200,
                    "message": "OK"
                },
                "data": {
                    "apppassword": "generated-secret"
                }
            }
        }"#;

        let parsed: OcsEnvelope<OcsAppPasswordData> =
            serde_json::from_str(raw).expect("valid OCS app password JSON");
        assert_eq!(parsed.into_data()?.app_password, "generated-secret");
        Ok(())
    }
}
