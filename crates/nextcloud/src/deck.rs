use reqwest::Method;
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

    pub async fn create_board(&self, options: &DeckBoardCreateOptions) -> Result<DeckBoard> {
        let raw: RawDeckBoard = self
            .client
            .request_json_with_ocs_header(
                Method::POST,
                DECK_BOARDS_ENDPOINT,
                &DeckBoardCreateRequest {
                    title: options.title.as_str(),
                    color: options.color.as_str(),
                },
            )
            .await?;
        Ok(DeckBoard::from(raw))
    }

    pub async fn create_stack(&self, options: &DeckStackCreateOptions) -> Result<DeckStack> {
        let path = format!(
            "index.php/apps/deck/api/v1.0/boards/{}/stacks",
            options.board_id
        );
        let raw: RawDeckStack = self
            .client
            .request_json_with_ocs_header(
                Method::POST,
                &path,
                &DeckStackCreateRequest {
                    title: options.title.as_str(),
                    order: options.order,
                },
            )
            .await?;
        Ok(DeckStack::from(raw))
    }

    pub async fn cards(&self, options: &DeckCardsOptions) -> Result<Vec<DeckCard>> {
        let path = format!(
            "index.php/apps/deck/api/v1.0/boards/{}/stacks",
            options.board_id
        );
        let stacks = self.client.get_json::<Vec<RawDeckStack>>(&path).await?;
        let mut cards = Vec::new();
        for stack in stacks {
            let stack_id = stack.string("id").unwrap_or_default();
            let stack_title = stack.string("title").and_then(non_empty);
            for raw_card in stack.cards() {
                let card = DeckCard::from_raw(raw_card, stack_id.clone(), stack_title.clone());
                if options.include_archived || (!card.archived && !card.deleted) {
                    cards.push(card);
                }
            }
        }
        Ok(cards)
    }

    pub async fn create_card(&self, options: &DeckCardCreateOptions) -> Result<DeckCard> {
        let path = format!(
            "index.php/apps/deck/api/v1.0/boards/{}/stacks/{}/cards",
            options.board_id, options.stack_id
        );
        let raw: RawDeckCard = self
            .client
            .request_json_with_ocs_header(
                Method::POST,
                &path,
                &DeckCardCreateRequest {
                    title: options.title.as_str(),
                    card_type: "plain",
                    order: options.order,
                    description: options.description.as_deref(),
                    duedate: options.due_at.as_deref(),
                },
            )
            .await?;
        let mut card = DeckCard::from_raw(raw, options.stack_id.clone(), None);
        if card.board_id.is_empty() {
            card.board_id = options.board_id.clone();
        }
        Ok(card)
    }

    pub async fn update_card(&self, options: &DeckCardUpdateOptions) -> Result<DeckCard> {
        let path = format!(
            "index.php/apps/deck/api/v1.0/boards/{}/stacks/{}/cards/{}",
            options.board_id, options.stack_id, options.card_id
        );
        let raw: RawDeckCard = self
            .client
            .request_json_with_ocs_header(
                Method::PUT,
                &path,
                &DeckCardUpdateRequest {
                    title: options.title.as_deref(),
                    card_type: Some("plain"),
                    order: options.order,
                    description: options.description.as_deref(),
                    duedate: options.due_at.as_deref(),
                },
            )
            .await?;
        let mut card = DeckCard::from_raw(raw, options.stack_id.clone(), None);
        if card.board_id.is_empty() {
            card.board_id = options.board_id.clone();
        }
        Ok(card)
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeckBoardCreateOptions {
    pub title: String,
    pub color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeckStack {
    pub id: String,
    pub board_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<i64>,
    pub deleted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeckStackCreateOptions {
    pub board_id: String,
    pub title: String,
    pub order: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeckCardsOptions {
    pub board_id: String,
    pub include_archived: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeckCard {
    pub id: String,
    pub board_id: String,
    pub stack_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub archived: bool,
    pub deleted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeckCardCreateOptions {
    pub board_id: String,
    pub stack_id: String,
    pub title: String,
    pub description: Option<String>,
    pub due_at: Option<String>,
    pub order: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeckCardUpdateOptions {
    pub board_id: String,
    pub stack_id: String,
    pub card_id: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub due_at: Option<String>,
    pub order: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
struct RawDeckBoard {
    #[serde(flatten)]
    fields: Map<String, Value>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
struct RawDeckStack {
    #[serde(flatten)]
    fields: Map<String, Value>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
struct RawDeckCard {
    #[serde(flatten)]
    fields: Map<String, Value>,
}

#[derive(Debug, Serialize)]
struct DeckBoardCreateRequest<'a> {
    title: &'a str,
    color: &'a str,
}

#[derive(Debug, Serialize)]
struct DeckStackCreateRequest<'a> {
    title: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    order: Option<i64>,
}

#[derive(Debug, Serialize)]
struct DeckCardCreateRequest<'a> {
    title: &'a str,
    #[serde(rename = "type")]
    card_type: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    order: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    duedate: Option<&'a str>,
}

#[derive(Debug, Serialize)]
struct DeckCardUpdateRequest<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<&'a str>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    card_type: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    order: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    duedate: Option<&'a str>,
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

impl From<RawDeckStack> for DeckStack {
    fn from(raw: RawDeckStack) -> Self {
        let deleted_at = raw.i64("deletedAt").or_else(|| raw.i64("deleted_at"));
        Self {
            id: raw.string("id").unwrap_or_default(),
            board_id: raw
                .string("boardId")
                .or_else(|| raw.string("board_id"))
                .unwrap_or_default(),
            title: raw.string("title").and_then(non_empty),
            order: raw.i64("order"),
            deleted: raw.bool("deleted").unwrap_or(false) || deleted_at.unwrap_or(0) != 0,
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

impl RawDeckStack {
    fn value(&self, key: &str) -> Option<&Value> {
        self.fields.get(key)
    }

    fn string(&self, key: &str) -> Option<String> {
        value_to_string(self.value(key)?)
    }

    fn i64(&self, key: &str) -> Option<i64> {
        value_to_i64(self.value(key)?)
    }

    fn bool(&self, key: &str) -> Option<bool> {
        value_to_bool(self.value(key)?)
    }

    fn cards(&self) -> Vec<RawDeckCard> {
        self.value("cards")
            .and_then(Value::as_array)
            .map(|cards| {
                cards
                    .iter()
                    .filter_map(|card| serde_json::from_value(card.clone()).ok())
                    .collect()
            })
            .unwrap_or_default()
    }
}

impl RawDeckCard {
    fn value(&self, key: &str) -> Option<&Value> {
        self.fields.get(key)
    }

    fn string(&self, key: &str) -> Option<String> {
        value_to_string(self.value(key)?)
    }

    fn bool(&self, key: &str) -> Option<bool> {
        value_to_bool(self.value(key)?)
    }
}

impl DeckCard {
    fn from_raw(raw: RawDeckCard, stack_id: String, stack_title: Option<String>) -> Self {
        Self {
            id: raw.string("id").unwrap_or_default(),
            board_id: raw
                .string("boardId")
                .or_else(|| raw.string("board_id"))
                .unwrap_or_default(),
            stack_id,
            stack_title,
            title: raw.string("title").and_then(non_empty),
            description: raw.string("description").and_then(non_empty),
            archived: raw.bool("archived").unwrap_or(false),
            deleted: raw.bool("deleted").unwrap_or(false),
        }
    }
}

fn value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        Value::Null | Value::Array(_) | Value::Object(_) => None,
    }
}

fn value_to_bool(value: &Value) -> Option<bool> {
    match value {
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

fn value_to_i64(value: &Value) -> Option<i64> {
    match value {
        Value::Number(value) => value.as_i64().or_else(|| value.as_u64().map(|n| n as i64)),
        Value::String(value) => value.parse().ok(),
        Value::Bool(value) => Some(i64::from(*value)),
        Value::Null | Value::Array(_) | Value::Object(_) => None,
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

    #[test]
    fn flattens_cards_from_stacks() {
        let raw = r#"[
            {
                "id": 1,
                "title": "Doing",
                "cards": [
                    {
                        "id": 99,
                        "boardId": 10,
                        "title": "Ship CLI",
                        "description": "work",
                        "archived": false,
                        "deleted": false
                    },
                    {
                        "id": 100,
                        "boardId": 10,
                        "title": "Archived",
                        "archived": true,
                        "deleted": false
                    }
                ]
            }
        ]"#;
        let stacks = serde_json::from_str::<Vec<RawDeckStack>>(raw).expect("valid stacks");
        let mut cards = Vec::new();
        for stack in stacks {
            let stack_id = stack.string("id").unwrap_or_default();
            let stack_title = stack.string("title").and_then(non_empty);
            for raw_card in stack.cards() {
                let card = DeckCard::from_raw(raw_card, stack_id.clone(), stack_title.clone());
                if !card.archived && !card.deleted {
                    cards.push(card);
                }
            }
        }

        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].id, "99");
        assert_eq!(cards[0].stack_id, "1");
        assert_eq!(cards[0].stack_title.as_deref(), Some("Doing"));
    }

    #[test]
    fn serializes_board_create_request() {
        let request = DeckBoardCreateRequest {
            title: "Roadmap",
            color: "0082c9",
        };
        let value = serde_json::to_value(request).expect("serializes");
        assert_eq!(value["title"], "Roadmap");
        assert_eq!(value["color"], "0082c9");
    }

    #[test]
    fn normalizes_stack() {
        let raw = r#"{
            "id": 4,
            "boardId": 2,
            "title": "Doing",
            "order": 999,
            "deletedAt": 0
        }"#;
        let stack = DeckStack::from(serde_json::from_str::<RawDeckStack>(raw).expect("stack"));
        assert_eq!(stack.id, "4");
        assert_eq!(stack.board_id, "2");
        assert_eq!(stack.title.as_deref(), Some("Doing"));
        assert_eq!(stack.order, Some(999));
        assert!(!stack.deleted);
    }

    #[test]
    fn serializes_stack_create_request() {
        let request = DeckStackCreateRequest {
            title: "Doing",
            order: Some(100),
        };
        let value = serde_json::to_value(request).expect("serializes");
        assert_eq!(value["title"], "Doing");
        assert_eq!(value["order"], 100);
    }

    #[test]
    fn serializes_card_create_request() {
        let request = DeckCardCreateRequest {
            title: "Ship CLI",
            card_type: "plain",
            order: Some(999),
            description: Some("private body"),
            duedate: Some("2026-04-10T12:00:00+00:00"),
        };
        let value = serde_json::to_value(request).expect("serializes");
        assert_eq!(value["title"], "Ship CLI");
        assert_eq!(value["type"], "plain");
        assert_eq!(value["order"], 999);
        assert_eq!(value["description"], "private body");
        assert_eq!(value["duedate"], "2026-04-10T12:00:00+00:00");
    }

    #[test]
    fn serializes_card_update_request_with_partial_fields() {
        let request = DeckCardUpdateRequest {
            title: Some("Updated"),
            card_type: Some("plain"),
            order: None,
            description: None,
            duedate: None,
        };
        let value = serde_json::to_value(request).expect("serializes");
        assert_eq!(value["title"], "Updated");
        assert_eq!(value["type"], "plain");
        assert!(value.get("description").is_none());
    }
}
