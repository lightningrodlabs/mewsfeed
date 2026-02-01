use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;

use crate::crypto;
use crate::error::S2SError;
use crate::mock_data::FederationDataSource;
use crate::signatures::SignatureHeaders;

/// Shared application state passed to all handlers.
pub struct AppState {
    pub data_source: Arc<dyn FederationDataSource>,
    pub domain: String,
}

/// ActivityPub JSON content type.
const AP_CONTENT_TYPE: &str = "application/activity+json";
/// JRD content type for WebFinger.
const JRD_CONTENT_TYPE: &str = "application/jrd+json";

// ---------------------------------------------------------------------------
// WebFinger
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct WebFingerQuery {
    resource: Option<String>,
}

/// GET /.well-known/webfinger?resource=acct:{user}@{domain}
pub async fn webfinger_handler(
    State(state): State<Arc<AppState>>,
    Query(query): Query<WebFingerQuery>,
) -> Result<Response, S2SError> {
    let resource = query
        .resource
        .ok_or_else(|| S2SError::BadRequest("missing 'resource' query parameter".into()))?;

    let acct = resource
        .strip_prefix("acct:")
        .ok_or_else(|| S2SError::BadRequest("resource must start with 'acct:'".into()))?;

    let (username, domain) = acct
        .split_once('@')
        .ok_or_else(|| S2SError::BadRequest("invalid acct URI format".into()))?;

    if domain != state.domain {
        return Err(S2SError::BadRequest(format!(
            "domain mismatch: expected {}, got {domain}",
            state.domain
        )));
    }

    let response = state
        .data_source
        .get_webfinger(username, domain)
        .await?
        .ok_or_else(|| S2SError::NotFound(format!("user: {username}")))?;

    Ok((
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, JRD_CONTENT_TYPE)],
        Json(response),
    )
        .into_response())
}

// ---------------------------------------------------------------------------
// Actor
// ---------------------------------------------------------------------------

/// GET /users/{username}
pub async fn actor_handler(
    State(state): State<Arc<AppState>>,
    Path(username): Path<String>,
    headers: HeaderMap,
) -> Result<Response, S2SError> {
    // Check Accept header contains activity+json (lenient: also accept json)
    let accept = headers
        .get(axum::http::header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if !accept.contains("application/activity+json")
        && !accept.contains("application/ld+json")
        && !accept.contains("application/json")
    {
        return Err(S2SError::BadRequest(
            "Accept header must include application/activity+json".into(),
        ));
    }

    let actor = state
        .data_source
        .get_actor(&username)
        .await?
        .ok_or_else(|| S2SError::NotFound(format!("user: {username}")))?;

    Ok((
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, AP_CONTENT_TYPE)],
        Json(actor),
    )
        .into_response())
}

// ---------------------------------------------------------------------------
// Inbox
// ---------------------------------------------------------------------------

/// POST /users/{username}/inbox
///
/// Verifies HTTP Signature, then processes the activity via the data source.
pub async fn inbox_handler(
    State(state): State<Arc<AppState>>,
    Path(username): Path<String>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Result<Response, S2SError> {
    // 1. Ensure user exists
    state
        .data_source
        .get_actor(&username)
        .await?
        .ok_or_else(|| S2SError::NotFound(format!("user: {username}")))?;

    // 2. Extract and parse the Signature header
    let sig_header_value = headers
        .get("signature")
        .ok_or_else(|| S2SError::Unauthorized("missing Signature header".into()))?
        .to_str()
        .map_err(|_| S2SError::BadRequest("invalid Signature header encoding".into()))?;

    let sig_parts = SignatureHeaders::parse(sig_header_value)?;

    // 3. Extract actor URL from keyId (strip #main-key fragment)
    let actor_url = sig_parts
        .key_id
        .split('#')
        .next()
        .unwrap_or(&sig_parts.key_id);

    // 4. Fetch sender's actor to get their public key
    let sender_actor = crate::client::fetch_actor(actor_url).await?;
    let public_key_pem = &sender_actor.public_key.public_key_pem;

    // 5. Reconstruct the signing string from received headers
    let path = format!("/users/{username}/inbox");
    let signed_header_names: Vec<&str> = sig_parts.headers.split_whitespace().collect();

    let header_pairs: Vec<(String, String)> = headers
        .iter()
        .filter_map(|(name, value)| {
            value
                .to_str()
                .ok()
                .map(|v| (name.as_str().to_string(), v.to_string()))
        })
        .collect();

    let signing_string =
        crypto::build_signing_string("POST", &path, &header_pairs, &signed_header_names);

    // 6. Verify the signature
    let valid = crypto::verify(&signing_string, &sig_parts.signature, public_key_pem)?;
    if !valid {
        return Err(S2SError::Unauthorized("invalid HTTP Signature".into()));
    }

    // 7. Parse body as activity
    let activity: crate::APActivity =
        serde_json::from_slice(&body).map_err(|e| S2SError::BadRequest(format!("json: {e}")))?;

    // 8. Validate activity.actor matches the signature's key owner
    if activity.actor != actor_url {
        return Err(S2SError::Unauthorized(format!(
            "activity actor ({}) does not match signature key owner ({actor_url})",
            activity.actor
        )));
    }

    // 9. Process the activity
    state.data_source.process_inbox(&username, activity).await?;

    Ok(StatusCode::ACCEPTED.into_response())
}

// ---------------------------------------------------------------------------
// Outbox
// ---------------------------------------------------------------------------

/// GET /users/{username}/outbox
pub async fn outbox_handler(
    State(state): State<Arc<AppState>>,
    Path(username): Path<String>,
) -> Result<Response, S2SError> {
    let outbox = state
        .data_source
        .get_outbox(&username)
        .await?
        .ok_or_else(|| S2SError::NotFound(format!("user: {username}")))?;

    Ok((
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, AP_CONTENT_TYPE)],
        Json(outbox),
    )
        .into_response())
}

// ---------------------------------------------------------------------------
// Debug endpoints (Phase 2 only)
// ---------------------------------------------------------------------------

/// GET /__debug/inbox/{username} -- return received activities as JSON array.
pub async fn debug_inbox_handler(
    State(state): State<Arc<AppState>>,
    Path(username): Path<String>,
) -> Result<Json<Vec<crate::APActivity>>, S2SError> {
    let activities = state.data_source.get_received_activities(&username).await?;
    Ok(Json(activities))
}

/// GET /__debug/key/{username} -- return the user's private key as PEM.
pub async fn debug_key_handler(
    State(state): State<Arc<AppState>>,
    Path(username): Path<String>,
) -> Result<Response, S2SError> {
    let pem = state
        .data_source
        .get_private_key_pem(&username)
        .await?
        .ok_or_else(|| S2SError::NotFound(format!("user: {username}")))?;

    Ok((
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "application/x-pem-file")],
        pem,
    )
        .into_response())
}
