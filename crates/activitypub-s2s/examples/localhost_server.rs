//! Localhost ActivityPub server for manual testing.
//!
//! Run with:
//!   cargo run -p activitypub-s2s --example localhost_server
//!
//! Or with a custom port:
//!   PORT=3001 cargo run -p activitypub-s2s --example localhost_server

use activitypub_s2s::server::{start_server, ServerConfig};

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "activitypub_s2s=debug,tower_http=debug".parse().unwrap()),
        )
        .init();

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);

    let domain = format!("localhost:{port}");

    let config = ServerConfig {
        port,
        domain: domain.clone(),
    };

    match start_server(config).await {
        Ok((addr, handle)) => {
            println!("ActivityPub localhost server running on http://{addr}");
            println!();
            println!("Mock users: alice, bob");
            println!();
            println!("Try these curl commands:");
            println!();
            println!("  # WebFinger discovery");
            println!(
                "  curl -s 'http://{addr}/.well-known/webfinger?resource=acct:alice@{domain}' | jq ."
            );
            println!();
            println!("  # Fetch actor");
            println!(
                "  curl -s -H 'Accept: application/activity+json' 'http://{addr}/users/alice' | jq ."
            );
            println!();
            println!("  # Fetch outbox");
            println!(
                "  curl -s -H 'Accept: application/activity+json' 'http://{addr}/users/alice/outbox' | jq ."
            );
            println!();
            println!("  # Debug: list received inbox activities");
            println!("  curl -s 'http://{addr}/__debug/inbox/alice' | jq .");
            println!();
            println!("  # Debug: export private key (PEM)");
            println!("  curl -s 'http://{addr}/__debug/key/alice'");
            println!();

            handle.await.ok();
        }
        Err(e) => {
            eprintln!("Failed to start server: {e}");
            std::process::exit(1);
        }
    }
}
