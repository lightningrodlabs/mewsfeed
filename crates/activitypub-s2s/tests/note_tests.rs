use activitypub_s2s::*;

const MASTODON_NOTE_JSON: &str = include_str!("fixtures/mastodon_note.json");

#[test]
fn parse_mastodon_note_fixture() {
    let note: APNote = serde_json::from_str(MASTODON_NOTE_JSON).expect("parse note");
    assert_eq!(
        note.id,
        "https://mastodon.social/users/Gargron/statuses/103270115826048975"
    );
    assert_eq!(note.note_type, "Note");
    assert_eq!(note.attributed_to, "https://mastodon.social/users/Gargron");
    assert!(note.content.contains("Hello from"));
    assert_eq!(note.published, "2019-12-08T20:30:00Z");
    assert!(note.in_reply_to.is_none());
    assert_eq!(note.to, vec![AS_PUBLIC]);
    assert!(!note.sensitive);

    assert_eq!(note.tag.len(), 2);
    assert_eq!(note.tag[0].tag_type, "Hashtag");
    assert_eq!(note.tag[0].name, "#Mastodon");
    assert_eq!(note.tag[1].tag_type, "Mention");
    assert_eq!(note.tag[1].name, "@alice@mastodon.social");
}

#[test]
fn serialize_note_roundtrip() {
    let note = APNote {
        context: serde_json::json!(AS_CONTEXT),
        id: "https://cats.mewsfeed.example/notes/abc123".to_string(),
        note_type: "Note".to_string(),
        attributed_to: "https://cats.mewsfeed.example/users/alice".to_string(),
        content: "<p>Hello from Holochain!</p>".to_string(),
        published: "2024-01-15T12:00:00Z".to_string(),
        in_reply_to: None,
        to: vec![AS_PUBLIC.to_string()],
        cc: vec!["https://cats.mewsfeed.example/users/alice/followers".to_string()],
        tag: vec![],
        sensitive: false,
        conversation: None,
        url: None,
    };

    let json = serde_json::to_string(&note).expect("serialize");
    let restored: APNote = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored, note);
}

#[test]
fn note_json_field_names() {
    let note = APNote {
        context: serde_json::json!(AS_CONTEXT),
        id: "https://example.com/notes/1".to_string(),
        note_type: "Note".to_string(),
        attributed_to: "https://example.com/users/test".to_string(),
        content: "<p>test</p>".to_string(),
        published: "2024-01-01T00:00:00Z".to_string(),
        in_reply_to: Some("https://example.com/notes/0".to_string()),
        to: vec![AS_PUBLIC.to_string()],
        cc: vec![],
        tag: vec![APTag {
            tag_type: "Hashtag".to_string(),
            href: Some("https://example.com/tags/test".to_string()),
            name: "#test".to_string(),
        }],
        sensitive: true,
        conversation: None,
        url: None,
    };

    let value: serde_json::Value = serde_json::to_value(&note).expect("to value");
    let obj = value.as_object().expect("is object");

    assert!(obj.contains_key("@context"));
    assert!(obj.contains_key("type"));
    assert!(obj.contains_key("attributedTo"));
    assert!(obj.contains_key("inReplyTo"));
}
