use activitypub_s2s::*;

const MASTODON_ACTOR_JSON: &str = include_str!("fixtures/mastodon_actor.json");

#[test]
fn parse_mastodon_actor_fixture() {
    let actor: APActor = serde_json::from_str(MASTODON_ACTOR_JSON).expect("parse actor");
    assert_eq!(actor.id, "https://mastodon.social/users/Gargron");
    assert_eq!(actor.actor_type, "Person");
    assert_eq!(actor.preferred_username, "Gargron");
    assert_eq!(actor.name, Some("Eugen Rochko".to_string()));
    assert!(actor.summary.is_some());
    assert_eq!(actor.inbox, "https://mastodon.social/users/Gargron/inbox");
    assert_eq!(actor.outbox, "https://mastodon.social/users/Gargron/outbox");
    assert!(actor.followers.is_some());
    assert!(actor.following.is_some());

    assert_eq!(
        actor.public_key.id,
        "https://mastodon.social/users/Gargron#main-key"
    );
    assert_eq!(
        actor.public_key.owner,
        "https://mastodon.social/users/Gargron"
    );
    assert!(actor.public_key.public_key_pem.contains("BEGIN PUBLIC KEY"));

    let icon = actor.icon.expect("icon present");
    assert_eq!(icon.image_type, "Image");
    assert!(icon.url.contains("gargron.jpeg"));
}

#[test]
fn serialize_actor_roundtrip() {
    let actor = APActor {
        context: serde_json::json!([AS_CONTEXT, SECURITY_CONTEXT]),
        id: "https://cats.mewsfeed.example/users/alice".to_string(),
        actor_type: "Person".to_string(),
        preferred_username: "alice".to_string(),
        name: Some("Alice".to_string()),
        summary: Some("<p>Meow!</p>".to_string()),
        inbox: "https://cats.mewsfeed.example/users/alice/inbox".to_string(),
        outbox: "https://cats.mewsfeed.example/users/alice/outbox".to_string(),
        followers: Some("https://cats.mewsfeed.example/users/alice/followers".to_string()),
        following: Some("https://cats.mewsfeed.example/users/alice/following".to_string()),
        public_key: APPublicKey {
            id: "https://cats.mewsfeed.example/users/alice#main-key".to_string(),
            owner: "https://cats.mewsfeed.example/users/alice".to_string(),
            public_key_pem: "-----BEGIN PUBLIC KEY-----\ntest\n-----END PUBLIC KEY-----\n"
                .to_string(),
        },
        icon: None,
        url: None,
        manually_approves_followers: false,
        discoverable: false,
    };

    let json = serde_json::to_string(&actor).expect("serialize");
    assert!(json.contains(r#""@context""#));
    assert!(json.contains(r#""type":"Person""#));
    assert!(json.contains(r#""preferredUsername":"alice""#));

    let restored: APActor = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored, actor);
}

#[test]
fn actor_json_field_names() {
    let actor = APActor {
        context: serde_json::json!(AS_CONTEXT),
        id: "https://example.com/users/test".to_string(),
        actor_type: "Person".to_string(),
        preferred_username: "test".to_string(),
        name: None,
        summary: None,
        inbox: "https://example.com/users/test/inbox".to_string(),
        outbox: "https://example.com/users/test/outbox".to_string(),
        followers: None,
        following: None,
        public_key: APPublicKey {
            id: "https://example.com/users/test#main-key".to_string(),
            owner: "https://example.com/users/test".to_string(),
            public_key_pem: "pem".to_string(),
        },
        icon: None,
        url: None,
        manually_approves_followers: false,
        discoverable: false,
    };

    let value: serde_json::Value = serde_json::to_value(&actor).expect("to value");
    let obj = value.as_object().expect("is object");

    assert!(obj.contains_key("@context"));
    assert!(obj.contains_key("type"));
    assert!(obj.contains_key("preferredUsername"));
    assert!(obj.contains_key("publicKey"));
    // Verify None fields are omitted
    assert!(!obj.contains_key("name"));
    assert!(!obj.contains_key("summary"));
    assert!(!obj.contains_key("icon"));
}
