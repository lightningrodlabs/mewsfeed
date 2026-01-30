use activitypub_s2s::*;

const MASTODON_WEBFINGER_JSON: &str = include_str!("fixtures/mastodon_webfinger.json");

#[test]
fn parse_mastodon_webfinger_fixture() {
    let wf: WebFingerResponse =
        serde_json::from_str(MASTODON_WEBFINGER_JSON).expect("parse webfinger");
    assert_eq!(wf.subject, "acct:Gargron@mastodon.social");
    assert_eq!(wf.aliases.len(), 2);
    assert!(wf
        .aliases
        .contains(&"https://mastodon.social/@Gargron".to_string()));

    let self_link = wf
        .links
        .iter()
        .find(|l| l.rel == "self")
        .expect("self link");
    assert_eq!(
        self_link.link_type.as_deref(),
        Some("application/activity+json")
    );
    assert_eq!(
        self_link.href.as_deref(),
        Some("https://mastodon.social/users/Gargron")
    );

    let subscribe = wf
        .links
        .iter()
        .find(|l| l.rel == "http://ostatus.org/schema/1.0/subscribe")
        .expect("subscribe link");
    assert!(subscribe.template.is_some());
    assert!(subscribe.href.is_none());
}

#[test]
fn serialize_mewsfeed_webfinger() {
    let wf = WebFingerResponse {
        subject: "acct:alice@cats.mewsfeed.example".to_string(),
        aliases: vec![
            "https://cats.mewsfeed.example/users/alice".to_string(),
            "https://cats.mewsfeed.example/@alice".to_string(),
        ],
        links: vec![
            WebFingerLink {
                rel: "self".to_string(),
                link_type: Some("application/activity+json".to_string()),
                href: Some("https://cats.mewsfeed.example/users/alice".to_string()),
                template: None,
                properties: Default::default(),
            },
            WebFingerLink {
                rel: "http://webfinger.net/rel/profile-page".to_string(),
                link_type: Some("text/html".to_string()),
                href: Some("https://cats.mewsfeed.example/@alice".to_string()),
                template: None,
                properties: Default::default(),
            },
        ],
    };

    let json = serde_json::to_string_pretty(&wf).expect("serialize");
    assert!(json.contains(r#""type": "application/activity+json""#));
    assert!(!json.contains("link_type"));

    let restored: WebFingerResponse = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored, wf);
}

#[test]
fn webfinger_empty_aliases_omitted() {
    let wf = WebFingerResponse {
        subject: "acct:test@example.com".to_string(),
        aliases: vec![],
        links: vec![],
    };

    let json = serde_json::to_string(&wf).expect("serialize");
    assert!(!json.contains("aliases"));
}
