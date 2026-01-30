use serde::{Deserialize, Serialize};
use serde_json::Value;

/// An ActivityPub Activity.
///
/// Uses a flat struct with a `type` discriminant and `serde_json::Value` for the
/// object field. This matches the real AP wire format where the object's shape
/// varies by activity type (string URI, nested object, or array).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct APActivity {
    #[serde(rename = "@context")]
    pub context: serde_json::Value,

    /// Activity URI.
    pub id: String,

    /// Activity type: "Create", "Follow", "Accept", "Like",
    /// "Announce", "Undo", "Delete", etc.
    #[serde(rename = "type")]
    pub activity_type: String,

    /// The actor performing this activity.
    pub actor: String,

    /// The object of the activity. Shape depends on `activity_type`:
    /// - Create: an APNote object
    /// - Follow: actor URI string
    /// - Accept: the original Follow activity object
    /// - Like: note URI string
    /// - Announce: note URI string
    /// - Undo: the original activity object
    /// - Delete: object URI string or Tombstone object
    pub object: Value,

    /// Primary audience.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub to: Vec<String>,

    /// Secondary audience.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cc: Vec<String>,

    /// ISO 8601 timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published: Option<String>,
}

/// Well-known activity type constants.
pub mod activity_type {
    pub const CREATE: &str = "Create";
    pub const FOLLOW: &str = "Follow";
    pub const ACCEPT: &str = "Accept";
    pub const REJECT: &str = "Reject";
    pub const LIKE: &str = "Like";
    pub const ANNOUNCE: &str = "Announce";
    pub const UNDO: &str = "Undo";
    pub const DELETE: &str = "Delete";
}
