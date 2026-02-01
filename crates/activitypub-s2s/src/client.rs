//! Outbound HTTP client for ActivityPub activity delivery and resource fetching.
//!
//! Uses reqwest for HTTP and the `crypto` module for HTTP Signature construction.

use rsa::RsaPrivateKey;

use crate::actor::APActor;
use crate::crypto;
use crate::error::S2SError;
use crate::webfinger::WebFingerResponse;
use crate::APActivity;

/// Deliver an activity to a remote inbox with HTTP Signature authentication.
///
/// Returns the HTTP status code from the remote server.
pub async fn deliver_activity(
    inbox_url: &str,
    activity: &APActivity,
    key_id: &str,
    private_key: &RsaPrivateKey,
) -> Result<u16, S2SError> {
    let body = serde_json::to_vec(activity).map_err(S2SError::Json)?;

    let url: reqwest::Url = inbox_url
        .parse()
        .map_err(|e| S2SError::BadRequest(format!("invalid inbox URL: {e}")))?;

    let host = match url.port() {
        Some(port) => format!("{}:{}", url.host_str().unwrap_or(""), port),
        None => url.host_str().unwrap_or("").to_string(),
    };
    let path = url.path().to_string();

    let date = chrono::Utc::now()
        .format("%a, %d %b %Y %H:%M:%S GMT")
        .to_string();
    let digest = crypto::compute_digest(&body);

    let headers = vec![
        ("host".to_string(), host.clone()),
        ("date".to_string(), date.clone()),
        ("digest".to_string(), digest.clone()),
    ];

    let signed_header_names = ["(request-target)", "host", "date", "digest"];
    let signing_string =
        crypto::build_signing_string("POST", &path, &headers, &signed_header_names);
    let signature_b64 = crypto::sign(&signing_string, private_key)?;
    let signature_header =
        crypto::build_signature_header(key_id, &signed_header_names, &signature_b64);

    let client = reqwest::Client::new();
    let response = client
        .post(inbox_url)
        .header("Content-Type", "application/activity+json")
        .header("Host", &host)
        .header("Date", &date)
        .header("Digest", &digest)
        .header("Signature", &signature_header)
        .body(body)
        .send()
        .await
        .map_err(S2SError::HttpClient)?;

    Ok(response.status().as_u16())
}

/// Fetch a remote actor by its ActivityPub actor URL.
pub async fn fetch_actor(actor_url: &str) -> Result<APActor, S2SError> {
    let client = reqwest::Client::new();
    let response = client
        .get(actor_url)
        .header("Accept", "application/activity+json")
        .send()
        .await
        .map_err(S2SError::HttpClient)?;

    if !response.status().is_success() {
        return Err(S2SError::NotFound(format!(
            "actor fetch returned HTTP {}",
            response.status()
        )));
    }

    response
        .json::<APActor>()
        .await
        .map_err(|e| S2SError::BadRequest(format!("failed to parse actor response: {e}")))
}

/// Perform a WebFinger lookup for a user at a given domain.
pub async fn fetch_webfinger(domain: &str, username: &str) -> Result<WebFingerResponse, S2SError> {
    let scheme = if domain.starts_with("localhost") {
        "http"
    } else {
        "https"
    };
    let url =
        format!("{scheme}://{domain}/.well-known/webfinger?resource=acct:{username}@{domain}",);

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header("Accept", "application/jrd+json")
        .send()
        .await
        .map_err(S2SError::HttpClient)?;

    if !response.status().is_success() {
        return Err(S2SError::NotFound(format!(
            "WebFinger lookup returned HTTP {}",
            response.status()
        )));
    }

    response
        .json::<WebFingerResponse>()
        .await
        .map_err(|e| S2SError::BadRequest(format!("failed to parse WebFinger response: {e}")))
}
