use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

/// Unified error type for the activitypub-s2s crate.
#[derive(thiserror::Error, Debug)]
pub enum S2SError {
    #[error("bad request: {0}")]
    BadRequest(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("unauthorized: {0}")]
    Unauthorized(String),

    #[error("internal error: {0}")]
    Internal(String),

    #[error("crypto error: {0}")]
    Crypto(String),

    #[error("http client error: {0}")]
    HttpClient(#[from] reqwest::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("signature parse error: {0}")]
    SignatureParse(#[from] crate::SignatureParseError),
}

impl IntoResponse for S2SError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            S2SError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            S2SError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            S2SError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg.clone()),
            S2SError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
            S2SError::Crypto(msg) => (StatusCode::INTERNAL_SERVER_ERROR, format!("crypto: {msg}")),
            S2SError::HttpClient(e) => (StatusCode::BAD_GATEWAY, format!("upstream: {e}")),
            S2SError::Json(e) => (StatusCode::BAD_REQUEST, format!("json: {e}")),
            S2SError::SignatureParse(e) => (StatusCode::BAD_REQUEST, format!("signature: {e}")),
        };
        let body = serde_json::json!({ "error": message });
        (status, axum::Json(body)).into_response()
    }
}
