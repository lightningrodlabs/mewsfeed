use serde::{Deserialize, Serialize};

/// An ActivityPub OrderedCollection (used for outbox, followers, following).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OrderedCollection {
    #[serde(rename = "@context")]
    pub context: serde_json::Value,

    pub id: String,

    #[serde(rename = "type")]
    pub collection_type: String,

    pub total_items: u64,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub first: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub last: Option<String>,
}

/// A page within an OrderedCollection.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OrderedCollectionPage {
    #[serde(rename = "@context")]
    pub context: serde_json::Value,

    pub id: String,

    #[serde(rename = "type")]
    pub page_type: String,

    pub part_of: String,

    #[serde(default)]
    pub ordered_items: Vec<serde_json::Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub next: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev: Option<String>,
}
