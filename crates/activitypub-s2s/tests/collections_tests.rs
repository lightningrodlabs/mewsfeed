use activitypub_s2s::*;

#[test]
fn ordered_collection_roundtrip() {
    let collection = OrderedCollection {
        context: serde_json::json!(AS_CONTEXT),
        id: "https://cats.mewsfeed.example/users/alice/outbox".to_string(),
        collection_type: "OrderedCollection".to_string(),
        total_items: 42,
        first: Some("https://cats.mewsfeed.example/users/alice/outbox?page=true".to_string()),
        last: None,
    };

    let json = serde_json::to_string(&collection).expect("serialize");
    let restored: OrderedCollection = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored, collection);

    let value: serde_json::Value = serde_json::from_str(&json).expect("parse");
    assert!(value.get("totalItems").is_some());
}

#[test]
fn ordered_collection_page_roundtrip() {
    let page = OrderedCollectionPage {
        context: serde_json::json!(AS_CONTEXT),
        id: "https://cats.mewsfeed.example/users/alice/outbox?page=true".to_string(),
        page_type: "OrderedCollectionPage".to_string(),
        part_of: "https://cats.mewsfeed.example/users/alice/outbox".to_string(),
        ordered_items: vec![serde_json::json!({
            "type": "Create",
            "id": "https://cats.mewsfeed.example/activities/1"
        })],
        next: Some("https://cats.mewsfeed.example/users/alice/outbox?page=2".to_string()),
        prev: None,
    };

    let json = serde_json::to_string(&page).expect("serialize");
    let restored: OrderedCollectionPage = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored, page);

    let value: serde_json::Value = serde_json::from_str(&json).expect("parse");
    assert!(value.get("orderedItems").is_some());
    assert!(value.get("partOf").is_some());
}
