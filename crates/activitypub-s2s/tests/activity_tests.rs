use activitypub_s2s::*;

#[test]
fn create_activity_with_note() {
    let activity = APActivity {
        context: serde_json::json!(AS_CONTEXT),
        id: "https://cats.mewsfeed.example/activities/create-1".to_string(),
        activity_type: activity_type::CREATE.to_string(),
        actor: "https://cats.mewsfeed.example/users/alice".to_string(),
        object: serde_json::json!({
            "id": "https://cats.mewsfeed.example/notes/abc123",
            "type": "Note",
            "attributedTo": "https://cats.mewsfeed.example/users/alice",
            "content": "<p>Hello!</p>",
            "published": "2024-01-15T12:00:00Z",
            "to": [AS_PUBLIC]
        }),
        to: vec![AS_PUBLIC.to_string()],
        cc: vec![],
        published: Some("2024-01-15T12:00:00Z".to_string()),
    };

    let json = serde_json::to_string(&activity).expect("serialize");
    let restored: APActivity = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored.activity_type, "Create");
    assert_eq!(restored.actor, activity.actor);
}

#[test]
fn follow_activity() {
    let activity = APActivity {
        context: serde_json::json!(AS_CONTEXT),
        id: "https://cats.mewsfeed.example/activities/follow-1".to_string(),
        activity_type: activity_type::FOLLOW.to_string(),
        actor: "https://cats.mewsfeed.example/users/alice".to_string(),
        object: serde_json::json!("https://mastodon.social/users/bob"),
        to: vec![],
        cc: vec![],
        published: None,
    };

    let json = serde_json::to_string(&activity).expect("serialize");
    assert!(json.contains(r#""type":"Follow""#));

    let value: serde_json::Value = serde_json::from_str(&json).expect("parse");
    assert!(value["object"].is_string());
}

#[test]
fn accept_follow_activity() {
    let original_follow = serde_json::json!({
        "id": "https://cats.mewsfeed.example/activities/follow-1",
        "type": "Follow",
        "actor": "https://cats.mewsfeed.example/users/alice",
        "object": "https://mastodon.social/users/bob"
    });

    let accept = APActivity {
        context: serde_json::json!(AS_CONTEXT),
        id: "https://mastodon.social/activities/accept-1".to_string(),
        activity_type: activity_type::ACCEPT.to_string(),
        actor: "https://mastodon.social/users/bob".to_string(),
        object: original_follow,
        to: vec!["https://cats.mewsfeed.example/users/alice".to_string()],
        cc: vec![],
        published: None,
    };

    let json = serde_json::to_string(&accept).expect("serialize");
    let restored: APActivity = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored.activity_type, "Accept");
    assert!(restored.object.is_object());
}

#[test]
fn like_activity() {
    let like = APActivity {
        context: serde_json::json!(AS_CONTEXT),
        id: "https://mastodon.social/activities/like-1".to_string(),
        activity_type: activity_type::LIKE.to_string(),
        actor: "https://mastodon.social/users/bob".to_string(),
        object: serde_json::json!("https://cats.mewsfeed.example/notes/abc123"),
        to: vec!["https://cats.mewsfeed.example/users/alice".to_string()],
        cc: vec![],
        published: None,
    };

    let json = serde_json::to_string(&like).expect("serialize");
    let restored: APActivity = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored.activity_type, "Like");
    assert_eq!(
        restored.object.as_str().expect("string object"),
        "https://cats.mewsfeed.example/notes/abc123"
    );
}

#[test]
fn undo_like_activity() {
    let undo = APActivity {
        context: serde_json::json!(AS_CONTEXT),
        id: "https://mastodon.social/activities/undo-1".to_string(),
        activity_type: activity_type::UNDO.to_string(),
        actor: "https://mastodon.social/users/bob".to_string(),
        object: serde_json::json!({
            "id": "https://mastodon.social/activities/like-1",
            "type": "Like",
            "actor": "https://mastodon.social/users/bob",
            "object": "https://cats.mewsfeed.example/notes/abc123"
        }),
        to: vec!["https://cats.mewsfeed.example/users/alice".to_string()],
        cc: vec![],
        published: None,
    };

    let json = serde_json::to_string(&undo).expect("serialize");
    let restored: APActivity = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored.activity_type, "Undo");
    assert_eq!(
        restored.object["type"].as_str().expect("nested type"),
        "Like"
    );
}

#[test]
fn announce_activity() {
    let announce = APActivity {
        context: serde_json::json!(AS_CONTEXT),
        id: "https://mastodon.social/activities/announce-1".to_string(),
        activity_type: activity_type::ANNOUNCE.to_string(),
        actor: "https://mastodon.social/users/bob".to_string(),
        object: serde_json::json!("https://cats.mewsfeed.example/notes/abc123"),
        to: vec![AS_PUBLIC.to_string()],
        cc: vec!["https://cats.mewsfeed.example/users/alice".to_string()],
        published: Some("2024-01-15T13:00:00Z".to_string()),
    };

    let json = serde_json::to_string(&announce).expect("serialize");
    let restored: APActivity = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored.activity_type, "Announce");
}

#[test]
fn delete_activity() {
    let delete = APActivity {
        context: serde_json::json!(AS_CONTEXT),
        id: "https://cats.mewsfeed.example/activities/delete-1".to_string(),
        activity_type: activity_type::DELETE.to_string(),
        actor: "https://cats.mewsfeed.example/users/alice".to_string(),
        object: serde_json::json!("https://cats.mewsfeed.example/notes/abc123"),
        to: vec![AS_PUBLIC.to_string()],
        cc: vec![],
        published: None,
    };

    let json = serde_json::to_string(&delete).expect("serialize");
    let restored: APActivity = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored.activity_type, "Delete");
}
