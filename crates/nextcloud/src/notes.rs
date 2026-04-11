use chrono::{DateTime, SecondsFormat, Utc};
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use url::form_urlencoded;

use crate::client::NextcloudClient;
use crate::error::Result;

const NOTES_ENDPOINT: &str = "index.php/apps/notes/api/v1/notes";

#[derive(Debug, Clone)]
pub struct NotesClient {
    client: NextcloudClient,
}

impl NotesClient {
    pub fn new(client: NextcloudClient) -> Self {
        Self { client }
    }

    pub async fn list(&self, options: &NotesListOptions) -> Result<Vec<Note>> {
        let mut serializer = form_urlencoded::Serializer::new(String::new());
        if let Some(category) = &options.category {
            serializer.append_pair("category", category);
        }
        if options.exclude_content {
            serializer.append_pair("exclude", "content");
        }
        let query = serializer.finish();
        let path = if query.is_empty() {
            NOTES_ENDPOINT.to_owned()
        } else {
            format!("{NOTES_ENDPOINT}?{query}")
        };

        let mut notes = self
            .client
            .get_json::<Vec<RawNote>>(&path)
            .await?
            .into_iter()
            .map(Note::from)
            .collect::<Vec<_>>();
        notes.truncate(options.limit.clamp(1, 100) as usize);
        Ok(notes)
    }

    pub async fn create(&self, options: &NotesCreateOptions) -> Result<Note> {
        let raw: RawNote = self
            .client
            .request_json(
                Method::POST,
                NOTES_ENDPOINT,
                &NotesCreateRequest {
                    title: Some(options.title.as_str()),
                    content: options.content.as_deref(),
                    category: options.category.as_deref(),
                },
            )
            .await?;
        Ok(Note::from(raw))
    }

    pub async fn delete(&self, id: &str) -> Result<()> {
        let path = format!("{NOTES_ENDPOINT}/{id}");
        self.client
            .request_text(Method::DELETE, &path, None)
            .await?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotesListOptions {
    pub category: Option<String>,
    pub exclude_content: bool,
    pub limit: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotesCreateOptions {
    pub title: String,
    pub content: Option<String>,
    pub category: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Note {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub etag: Option<String>,
    pub readonly: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified_unix: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    pub favorite: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
struct RawNote {
    #[serde(flatten)]
    fields: Map<String, Value>,
}

#[derive(Debug, Serialize)]
struct NotesCreateRequest<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    category: Option<&'a str>,
}

impl From<RawNote> for Note {
    fn from(raw: RawNote) -> Self {
        let modified_unix = raw.i64("modified").or_else(|| raw.i64("modified_unix"));
        Self {
            id: raw
                .string("id")
                .or_else(|| raw.string("note_id"))
                .unwrap_or_default(),
            etag: raw.string("etag").and_then(non_empty),
            readonly: raw.bool("readonly").unwrap_or(false),
            modified_at: raw
                .string("modified_at")
                .and_then(non_empty)
                .or_else(|| modified_unix.and_then(timestamp_to_rfc3339)),
            modified_unix,
            title: raw.string("title").and_then(non_empty),
            category: raw.string("category").and_then(non_empty),
            favorite: raw.bool("favorite").unwrap_or(false),
            content: raw.string("content").and_then(non_empty),
        }
    }
}

impl RawNote {
    fn value(&self, key: &str) -> Option<&Value> {
        self.fields.get(key)
    }

    fn string(&self, key: &str) -> Option<String> {
        match self.value(key)? {
            Value::String(value) => Some(value.clone()),
            Value::Number(value) => Some(value.to_string()),
            Value::Bool(value) => Some(value.to_string()),
            Value::Null | Value::Array(_) | Value::Object(_) => None,
        }
    }

    fn i64(&self, key: &str) -> Option<i64> {
        match self.value(key)? {
            Value::Number(value) => value.as_i64().or_else(|| value.as_u64().map(|n| n as i64)),
            Value::String(value) => value.parse().ok(),
            Value::Bool(value) => Some(i64::from(*value)),
            Value::Null | Value::Array(_) | Value::Object(_) => None,
        }
    }

    fn bool(&self, key: &str) -> Option<bool> {
        match self.value(key)? {
            Value::Bool(value) => Some(*value),
            Value::Number(value) => value.as_i64().map(|number| number != 0),
            Value::String(value) => match value.as_str() {
                "true" | "1" => Some(true),
                "false" | "0" | "" => Some(false),
                _ => None,
            },
            Value::Null | Value::Array(_) | Value::Object(_) => None,
        }
    }
}

fn timestamp_to_rfc3339(value: i64) -> Option<String> {
    DateTime::<Utc>::from_timestamp(value, 0)
        .map(|timestamp| timestamp.to_rfc3339_opts(SecondsFormat::Secs, true))
}

fn non_empty(value: String) -> Option<String> {
    if value.is_empty() { None } else { Some(value) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_notes() -> Result<()> {
        let raw = r#"[
            {
                "id": 76,
                "etag": "abc",
                "readonly": false,
                "modified": 1775833445,
                "title": "New note",
                "category": "work",
                "favorite": true,
                "content": "hello"
            }
        ]"#;
        let notes = serde_json::from_str::<Vec<RawNote>>(raw)
            .expect("valid notes JSON")
            .into_iter()
            .map(Note::from)
            .collect::<Vec<_>>();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].id, "76");
        assert_eq!(
            notes[0].modified_at.as_deref(),
            Some("2026-04-10T15:04:05Z")
        );
        assert!(notes[0].favorite);
        Ok(())
    }

    #[test]
    fn serializes_create_request_without_missing_fields() -> Result<()> {
        let body = NotesCreateRequest {
            title: Some("Plan"),
            content: None,
            category: Some("work"),
        };
        let value = serde_json::to_value(body).expect("serializes");
        assert_eq!(value["title"], "Plan");
        assert_eq!(value["category"], "work");
        assert!(value.get("content").is_none());
        Ok(())
    }
}
