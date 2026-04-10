use serde::{Deserialize, Serialize};
use url::Url;

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
    pub login_name: String,
    pub app_password: String,
}
