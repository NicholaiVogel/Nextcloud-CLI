use serde::{Deserialize, Serialize};
use url::Url;

use crate::client::NextcloudClient;
use crate::error::Result;

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
}
