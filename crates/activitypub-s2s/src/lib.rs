//! ActivityPub S2S protocol types for MewsFeed federation.
//!
//! This crate defines the full ActivityPub protocol types used by the S2S module
//! (running natively in Tauri/Electron). These types handle JSON-LD serialization
//! and are NOT used by the Holochain zome (which uses `federation_types` instead).
//!
//! The S2S module converts between these protocol types and the simpler state types
//! in `federation_types` when communicating with the zome over AppWebsocket.

pub mod activity;
pub mod actor;
pub mod client;
pub mod collections;
pub mod crypto;
pub mod error;
pub mod handlers;
pub mod mock_data;
pub mod note;
pub mod server;
pub mod signatures;
pub mod webfinger;

pub use activity::*;
pub use actor::*;
pub use collections::*;
pub use note::*;
pub use signatures::*;
pub use webfinger::*;

/// The standard ActivityStreams context URI.
pub const AS_CONTEXT: &str = "https://www.w3.org/ns/activitystreams";

/// The W3ID Security vocabulary context URI (used in Actor objects for publicKey).
pub const SECURITY_CONTEXT: &str = "https://w3id.org/security/v1";

/// The public addressing target for ActivityPub.
pub const AS_PUBLIC: &str = "https://www.w3.org/ns/activitystreams#Public";
