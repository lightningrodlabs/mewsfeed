use std::net::SocketAddr;
use std::sync::Arc;

use axum::routing::{get, post};
use axum::Router;
use tokio::task::JoinHandle;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::error::S2SError;
use crate::handlers::{self, AppState};
use crate::mock_data::FederationDataSource;

/// Server configuration.
pub struct ServerConfig {
    pub port: u16,
    pub domain: String,
}

/// Build the axum router with all ActivityPub endpoints.
pub fn build_router(data_source: Arc<dyn FederationDataSource>, domain: String) -> Router {
    let state = Arc::new(AppState {
        data_source,
        domain,
    });

    Router::new()
        // ActivityPub protocol endpoints
        .route("/.well-known/webfinger", get(handlers::webfinger_handler))
        .route("/users/{username}", get(handlers::actor_handler))
        .route("/users/{username}/inbox", post(handlers::inbox_handler))
        .route("/users/{username}/outbox", get(handlers::outbox_handler))
        // Debug endpoints (Phase 2 only)
        .route(
            "/__debug/inbox/{username}",
            get(handlers::debug_inbox_handler),
        )
        .route("/__debug/key/{username}", get(handlers::debug_key_handler))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// Start the server, returning the bound address and a join handle.
pub async fn start_server(config: ServerConfig) -> Result<(SocketAddr, JoinHandle<()>), S2SError> {
    let domain = config.domain.clone();
    let data_source = Arc::new(
        crate::mock_data::MockDataSource::new(&domain)
            .map_err(|e| S2SError::Internal(format!("mock data init: {e}")))?,
    );

    let router = build_router(data_source, domain);

    let addr = SocketAddr::from(([127, 0, 0, 1], config.port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| S2SError::Internal(format!("bind: {e}")))?;
    let bound_addr = listener
        .local_addr()
        .map_err(|e| S2SError::Internal(format!("local addr: {e}")))?;

    let handle = tokio::spawn(async move {
        axum::serve(listener, router).await.ok();
    });

    Ok((bound_addr, handle))
}
