use serde::{Deserialize, Serialize};

/// An ActivityPub Note object (equivalent to a Mew/tweet).
///
/// This is the content object wrapped in Create activities.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct APNote {
    #[serde(rename = "@context")]
    pub context: serde_json::Value,

    /// Canonical URI for this note.
    pub id: String,

    /// Always "Note".
    #[serde(rename = "type")]
    pub note_type: String,

    /// The actor who created this note.
    pub attributed_to: String,

    /// HTML content of the note.
    pub content: String,

    /// ISO 8601 timestamp.
    pub published: String,

    /// URI of the note this is replying to, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub in_reply_to: Option<String>,

    /// Primary audience (e.g., as:Public, specific actors).
    #[serde(default)]
    pub to: Vec<String>,

    /// Secondary audience (e.g., followers collection).
    #[serde(default)]
    pub cc: Vec<String>,

    /// Mentions and hashtags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tag: Vec<APTag>,

    /// Content warning flag.
    #[serde(default)]
    pub sensitive: bool,

    /// The conversation this note belongs to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation: Option<String>,

    /// URL to the HTML representation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// A tag on a Note (mention, hashtag, or emoji).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct APTag {
    /// Tag type: "Mention", "Hashtag", or "Emoji".
    #[serde(rename = "type")]
    pub tag_type: String,

    /// The target URI.
    /// For Mention: the actor URI.
    /// For Hashtag: the tag search URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub href: Option<String>,

    /// Display name.
    /// For Mention: "@alice@example.com".
    /// For Hashtag: "#cats".
    pub name: String,
}
