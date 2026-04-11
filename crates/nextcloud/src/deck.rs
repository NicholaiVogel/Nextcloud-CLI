use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::client::NextcloudClient;
use crate::error::Result;

const DECK_BOARDS_ENDPOINT: &str = "index.php/apps/deck/api/v1.0/boards";

#[derive(Debug, Clone)]
pub struct DeckClient {
    client: NextcloudClient,
}

impl DeckClient {
    pub fn new(client: NextcloudClient) -> Self {
        Self { client }
    }

    pub async fn boards(&self) -> Result<Vec<DeckBoard>> {
        let raw = self
            .client
            .get_json::<Vec<RawDeckBoard>>(DECK_BOARDS_ENDPOINT)
            .await?;
        Ok(raw.into_iter().map(DeckBoard::from).collect())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeckBoard {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    pub archived: bool,
    pub deleted: bool,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
struct RawDeckBoard {
    #[serde(flatten)]
    fields: Map<String, Value>,
}

impl From<RawDeckBoard> for DeckBoard {
    fn from(raw: RawDeckBoard) -> Self {
        Self {
            id: raw.string("id").unwrap_or_default(),
            title: raw.string("title").and_then(non_empty),
            owner: raw
                .string("owner")
                .or_else(|| raw.string("ownerUid"))
                .and_then(non_empty),
            color: raw.string("color").and_then(non_empty),
            archived: raw.bool("archived").unwrap_or(false),
            deleted: raw.bool("deleted").unwrap_or(false),
        }
    }
}

impl RawDeckBoard {
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

fn non_empty(value: String) -> Option<String> {
    if value.is_empty() { None } else { Some(value) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_boards() {
        let raw = r#"[
            {
                "id": 10,
                "title": "Roadmap",
                "owner": "nicholai",
                "color": "0082c9",
                "archived": false,
                "deleted": 0
            }
        ]"#;
        let boards = serde_json::from_str::<Vec<RawDeckBoard>>(raw)
            .expect("valid deck board JSON")
            .into_iter()
            .map(DeckBoard::from)
            .collect::<Vec<_>>();
        assert_eq!(boards.len(), 1);
        assert_eq!(boards[0].id, "10");
        assert_eq!(boards[0].title.as_deref(), Some("Roadmap"));
        assert_eq!(boards[0].owner.as_deref(), Some("nicholai"));
        assert!(!boards[0].archived);
    }
}
