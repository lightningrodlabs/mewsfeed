use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// WebFinger JRD (JSON Resource Descriptor) response per RFC 7033.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WebFingerResponse {
    /// The queried resource (e.g., "acct:alice@cats.mewsfeed.example").
    pub subject: String,

    /// Alternative identifiers for the resource.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,

    /// Typed links to related resources.
    pub links: Vec<WebFingerLink>,
}

/// A single link in a WebFinger JRD response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WebFingerLink {
    /// Link relation type (e.g., "self", "http://webfinger.net/rel/profile-page").
    pub rel: String,

    /// MIME type (e.g., "application/activity+json").
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub link_type: Option<String>,

    /// Target URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub href: Option<String>,

    /// URI template (used by OStatus subscribe relation).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,

    /// Additional properties.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub properties: HashMap<String, Option<String>>,
}
