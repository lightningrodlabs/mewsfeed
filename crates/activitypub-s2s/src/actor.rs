use serde::{Deserialize, Serialize};

/// A full ActivityPub Actor (Person) for JSON-LD serialization.
///
/// This is the wire format sent to/received from remote servers.
/// The S2S module constructs this from profile data fetched via the zome.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct APActor {
    /// JSON-LD context. Usually an array:
    /// `["https://www.w3.org/ns/activitystreams", "https://w3id.org/security/v1"]`
    #[serde(rename = "@context")]
    pub context: serde_json::Value,

    /// The actor's canonical URI (also serves as the AP id).
    pub id: String,

    /// Always "Person" for user actors.
    #[serde(rename = "type")]
    pub actor_type: String,

    /// The username portion (e.g., "alice").
    pub preferred_username: String,

    /// Display name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Bio / profile description (may contain HTML).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,

    /// The actor's inbox URL (POST activities here).
    pub inbox: String,

    /// The actor's outbox URL.
    pub outbox: String,

    /// URL of the followers collection.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followers: Option<String>,

    /// URL of the following collection.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub following: Option<String>,

    /// The actor's public key for HTTP Signature verification.
    pub public_key: APPublicKey,

    /// Avatar image.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<APImage>,

    /// Profile URL (HTML page).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,

    /// Whether this account requires follow approval.
    #[serde(default, skip_serializing_if = "is_false")]
    pub manually_approves_followers: bool,

    /// Whether this account is discoverable.
    #[serde(default, skip_serializing_if = "is_false")]
    pub discoverable: bool,
}

fn is_false(v: &bool) -> bool {
    !v
}

/// An actor's public key, embedded in the Actor object.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct APPublicKey {
    /// Key identifier, e.g., "https://cats.mewsfeed.example/users/alice#main-key"
    pub id: String,

    /// The actor this key belongs to.
    pub owner: String,

    /// PEM-encoded RSA public key.
    pub public_key_pem: String,
}

/// An image object (used for actor icons/avatars).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct APImage {
    /// Always "Image".
    #[serde(rename = "type")]
    pub image_type: String,

    /// URL of the image.
    pub url: String,

    /// MIME type, e.g., "image/png".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
}
