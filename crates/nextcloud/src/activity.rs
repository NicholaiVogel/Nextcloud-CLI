use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use url::form_urlencoded;

use crate::client::NextcloudClient;
use crate::error::Result;
use crate::ocs::OcsEnvelope;

const ACTIVITY_ENDPOINT: &str = "ocs/v2.php/apps/activity/api/v2/activity";

#[derive(Debug, Clone)]
pub struct ActivityClient {
    client: NextcloudClient,
}

impl ActivityClient {
    pub fn new(client: NextcloudClient) -> Self {
        Self { client }
    }

    pub async fn recent(&self, options: &ActivityRecentOptions) -> Result<Vec<ActivityItem>> {
        let mut serializer = form_urlencoded::Serializer::new(String::new());
        serializer.append_pair("format", "json");
        serializer.append_pair("limit", &options.limit.clamp(1, 100).to_string());
        let path = format!("{ACTIVITY_ENDPOINT}?{}", serializer.finish());

        let raw = self
            .client
            .get_ocs_json::<OcsEnvelope<Vec<RawActivity>>>(&path)
            .await?
            .into_data()?;
        Ok(raw.into_iter().map(ActivityItem::from).collect())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityRecentOptions {
    pub limit: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActivityItem {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activity_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub datetime: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
struct RawActivity {
    #[serde(flatten)]
    fields: Map<String, Value>,
}

impl From<RawActivity> for ActivityItem {
    fn from(raw: RawActivity) -> Self {
        Self {
            id: raw
                .string("activity_id")
                .or_else(|| raw.string("id"))
                .unwrap_or_default(),
            app: raw.string("app").and_then(non_empty),
            activity_type: raw
                .string("type")
                .or_else(|| raw.string("activity_type"))
                .and_then(non_empty),
            subject: raw
                .string("subject")
                .or_else(|| raw.string("subject_prepared"))
                .and_then(non_empty),
            message: raw
                .string("message")
                .or_else(|| raw.string("message_prepared"))
                .and_then(non_empty),
            object_type: raw.string("object_type").and_then(non_empty),
            object_id: raw.string("object_id").and_then(non_empty),
            link: raw.string("link").and_then(non_empty),
            datetime: raw
                .string("datetime")
                .or_else(|| raw.string("timestamp"))
                .and_then(non_empty),
        }
    }
}

impl RawActivity {
    fn string(&self, key: &str) -> Option<String> {
        match self.fields.get(key)? {
            Value::String(value) => Some(value.clone()),
            Value::Number(value) => Some(value.to_string()),
            Value::Bool(value) => Some(value.to_string()),
            Value::Null | Value::Array(_) | Value::Object(_) => None,
        }
    }
}

fn non_empty(value: String) -> Option<String> {
    if value.is_empty() { None } else { Some(value) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_activity_items() -> Result<()> {
        let raw = r#"{
            "ocs": {
                "meta": {
                    "status": "ok",
                    "statuscode": 200,
                    "message": "OK"
                },
                "data": [{
                    "activity_id": 42,
                    "app": "files",
                    "type": "file_created",
                    "subject": "You created report.md",
                    "object_type": "files",
                    "object_id": 99,
                    "link": "https://cloud.example.com/f/99",
                    "datetime": "2026-04-10T16:00:00+00:00"
                }]
            }
        }"#;

        let items = serde_json::from_str::<OcsEnvelope<Vec<RawActivity>>>(raw)
            .expect("valid activity response")
            .into_data()?
            .into_iter()
            .map(ActivityItem::from)
            .collect::<Vec<_>>();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, "42");
        assert_eq!(items[0].app.as_deref(), Some("files"));
        assert_eq!(items[0].activity_type.as_deref(), Some("file_created"));
        Ok(())
    }
}
