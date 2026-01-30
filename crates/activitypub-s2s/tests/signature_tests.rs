use activitypub_s2s::*;

#[test]
fn parse_signature_header() {
    let header = r#"keyId="https://mastodon.social/users/Gargron#main-key",algorithm="rsa-sha256",headers="(request-target) host date digest",signature="base64encodedvalue==""#;

    let sig = SignatureHeaders::parse(header).expect("parse signature");
    assert_eq!(sig.key_id, "https://mastodon.social/users/Gargron#main-key");
    assert_eq!(sig.algorithm, "rsa-sha256");
    assert_eq!(sig.headers, "(request-target) host date digest");
    assert_eq!(sig.signature, "base64encodedvalue==");
}

#[test]
fn signature_header_roundtrip() {
    let original = SignatureHeaders {
        key_id: "https://example.com/users/test#main-key".to_string(),
        algorithm: "rsa-sha256".to_string(),
        headers: "(request-target) host date".to_string(),
        signature: "abc123==".to_string(),
    };

    let header_str = original.to_header_string();
    let parsed = SignatureHeaders::parse(&header_str).expect("parse roundtrip");
    assert_eq!(parsed, original);
}

#[test]
fn parse_signature_missing_key_id() {
    let header = r#"algorithm="rsa-sha256",headers="(request-target)",signature="abc==""#;
    let result = SignatureHeaders::parse(header);
    assert!(result.is_err());
}

#[test]
fn parse_signature_missing_headers() {
    let header = r#"keyId="https://example.com/key",signature="abc==""#;
    let result = SignatureHeaders::parse(header);
    assert!(result.is_err());
}

#[test]
fn parse_signature_missing_signature() {
    let header = r#"keyId="https://example.com/key",headers="(request-target)""#;
    let result = SignatureHeaders::parse(header);
    assert!(result.is_err());
}

#[test]
fn parse_signature_default_algorithm() {
    let header = r#"keyId="https://example.com/key",headers="(request-target)",signature="abc==""#;
    let sig = SignatureHeaders::parse(header).expect("parse without algorithm");
    assert_eq!(sig.algorithm, "rsa-sha256");
}

#[test]
fn signature_json_roundtrip() {
    let sig = SignatureHeaders {
        key_id: "https://example.com/key".to_string(),
        algorithm: "rsa-sha256".to_string(),
        headers: "(request-target) host date".to_string(),
        signature: "sig123".to_string(),
    };

    let json = serde_json::to_string(&sig).expect("serialize");
    let restored: SignatureHeaders = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored, sig);

    assert!(json.contains("keyId"));
    assert!(!json.contains("key_id"));
}
