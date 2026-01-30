# ActivityPub S2S Integration for Mewsfeed

## Overview

This plan implements ActivityPub Server-to-Server (S2S) protocol integration for Mewsfeed, enabling every agent to federate with the broader ActivityPub universe (Mastodon, Pleroma, etc.) while preserving Holochain's agent-centric model.

### Key Principles
- **Agent as Actor**: Each Holochain agent becomes an ActivityPub Actor
- **Per-post federation**: Users choose which mews to federate (not all-or-nothing)
- **Minimal infrastructure**: Only a generic HTTP gateway required outside the hApp
- **Private by default**: Posts can remain Holochain-only
- **Shared types crate**: Federation state types (`federation_types`) shared between S2S module and zome; AP protocol types internal to `activitypub-s2s`

### Addressing Pattern
```
@{username}@{network-id}.mewsfeed.example
Example: @alice@main.mewsfeed.example
```

---

## Architecture

```
                        INTERNET
                           │
              ┌────────────┴────────────┐
              │   Generic HTTP Gateway  │
              │ *.mewsfeed.example:443  │
              └────────────┬────────────┘
                           │
              ┌────────────┴────────────┐
              │   Tauri/Electron App    │
              │  ┌───────────────────┐  │
              │  │ activitypub-s2s│  │
              │  │ crate (HTTP, AP)  │  │
              │  └─────────┬─────────┘  │
              │            │AppWebsocket│
              │  ┌─────────┴─────────┐  │
              │  │ Holochain Conductor│  │
              │  │  activitypub zome │  │
              │  │  (data + mapping) │  │
              │  └───────────────────┘  │
              └─────────────────────────┘

              ┌─────────────────────────┐
              │   federation_types      │
              │   (shared crate)        │
              │   imported by both      │
              │   S2S module + zome     │
              └─────────────────────────┘
```

### Design Principles

1. **Generic HTTP Gateway** - A thin layer that:
   - Receives HTTP requests from the internet
   - Routes based on subdomain to the appropriate S2S module endpoint
   - Forwards requests to `activitypub-s2s` crate running in Tauri/Electron
   - Returns responses from S2S module
   - **Has no ActivityPub knowledge**

2. **ActivityPub S2S Module** (in Tauri/Electron) handles:
   - HTTP Signature verification and signing
   - ActivityPub JSON-LD parsing
   - WebFinger resolution
   - Outbound HTTP requests (reqwest)
   - Calls zome via AppWebsocket
   - Uses shared `federation_types` crate for state types
   - AP protocol types (`APActor`, `APNote`, `APActivity`, etc.) are internal to this crate

3. **ActivityPub Zome** handles:
   - Data model (entries, links, validation)
   - AP ↔ Mewsfeed mapping
   - Federation state tracking
   - Emits signals for UI
   - Uses shared `federation_types` crate

4. **Benefits**:
   - Gateway never needs redeployment for protocol changes
   - Same gateway serves all Mewsfeed networks
   - S2S module has full Rust capabilities (HTTP, crypto)
   - Zome stays pure (data only)
   - Shared types ensure consistency between S2S module + zome

---

## Components to Implement

### 1. Shared and Protocol Type Crates

Types are split across two crates:

**`crates/federation_types/`** — DHT-safe shared state types (compiles to both WASM and native). Imported by **both** the activitypub-s2s crate and the activitypub zome.

```
crates/federation_types/src/
├── lib.rs                    # Re-exports
├── state.rs                  # MewFederationState, FederationVisibility
├── references.rs             # RemoteActorRef, RemoteFollow, RemoteFollower, RemoteInteraction
└── config.rs                 # InstanceConfig, InviteUrlParams
```

**`crates/activitypub-s2s/`** — AP protocol types, native-only (no WASM, no `hdk`). These types handle JSON-LD `@context`, complex nested objects, and protocol-specific serialization.

```
crates/activitypub-s2s/src/
├── lib.rs                    # Re-exports, AS_CONTEXT / SECURITY_CONTEXT / AS_PUBLIC constants
├── actor.rs                  # APActor, APPublicKey, APImage
├── note.rs                   # APNote, APTag
├── activity.rs               # APActivity struct, activity_type constants module
├── collections.rs            # OrderedCollection, OrderedCollectionPage
├── webfinger.rs              # WebFingerResponse, WebFingerLink
└── signatures.rs             # SignatureHeaders, SignatureParseError
```

**Key Types**:

```rust
// Actor
pub struct APActor {
    pub id: String,
    pub actor_type: String,  // "Person"
    pub preferred_username: String,
    pub name: Option<String>,
    pub summary: Option<String>,
    pub inbox: String,
    pub outbox: String,
    pub followers: Option<String>,
    pub following: Option<String>,
    pub public_key: APPublicKey,
    pub icon: Option<APImage>,
}

// Note (post)
pub struct APNote {
    pub id: String,
    pub attributed_to: String,
    pub content: String,
    pub published: String,
    pub in_reply_to: Option<String>,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub tag: Vec<APTag>,
    pub sensitive: bool,
}

// Activities — flat struct, not an enum.
// Real AP JSON uses a "type" discriminant with a polymorphic "object" field
// (sometimes a string URI, sometimes a nested object, sometimes an array).
// A flat struct with `object: serde_json::Value` avoids fragile
// `#[serde(untagged)]` enum issues.
pub struct APActivity {
    pub context: serde_json::Value,
    pub id: String,
    pub activity_type: String,    // "Create", "Follow", "Accept", etc.
    pub actor: String,
    pub object: serde_json::Value, // Shape varies by activity_type
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub published: Option<String>,
}

pub mod activity_type {
    pub const CREATE: &str = "Create";
    pub const FOLLOW: &str = "Follow";
    pub const ACCEPT: &str = "Accept";
    pub const REJECT: &str = "Reject";
    pub const LIKE: &str = "Like";
    pub const ANNOUNCE: &str = "Announce";
    pub const UNDO: &str = "Undo";
    pub const DELETE: &str = "Delete";
}

// Federation state (used by both S2S module and zome, defined in federation_types crate)
// Delivery tracking is handled by the S2S module outside the DHT.
pub struct MewFederationState {
    pub mew_hash: ActionHash,
    pub ap_uri: Option<String>,
    pub visibility: FederationVisibility,
}
```

**Type Mappings** (Mewsfeed ↔ ActivityPub):
| Mewsfeed | ActivityPub |
|----------|-------------|
| `Mew` (Original) | `Note` |
| `Mew` (Reply) | `Note` with `inReplyTo` |
| `Mew` (Quote) | `Note` with embedded quote |
| `Mewmew` | `Announce` activity |
| `Profile` | `Person` actor |
| Follow link | `Follow` activity |
| Like link | `Like` activity |

---

### 2. ActivityPub Zomes

**Location**: `dnas/mewsfeed/zomes/integrity/activitypub/` and `dnas/mewsfeed/zomes/coordinator/activitypub/`

#### Integrity Zome Entry Types

```rust
// Entry Types
FederationConfig        // Agent's AP settings (enabled, username, public_key_pem)
RemoteActor             // Cached remote AP actors
MewFederationState      // Per-mew federation state and visibility
IncomingActivity        // Received AP activities            **(not an entry)
OutgoingActivity        // Queued AP activities for delivery **(not an entry)
RemoteFollow            // Agent -> remote actor follows
RemoteFollower          // Remote actor -> agent follows
RemoteInteraction       // Likes/boosts/replies from fediverse
```

```rust
// Link Types
AgentToFederationConfig   // Agent -> their config
UsernameToAgent           // Username path -> Agent (webfinger)zAXj
AgentToRemoteFollows      // Agent -> remote follows
AgentToRemoteFollowers    // Agent -> remote followers
MewToFederationState      // Mew -> its AP state
MewToRemoteInteractions   // Mew -> fediverse interactions
DeliveryQueueToActivity   // Queue -> pending deliveries
ActivityIdToMew           // AP URL -> local mew hash
```

#### Coordinator Zome Functions

The zome **models ActivityPub state** and **maps between AP and Mewsfeed**. All HTTP and protocol logic is in the S2S module.

**Configuration**:
- `enable_federation(username)` - Create FederationConfig, link username
- `disable_federation()` - Mark config disabled
- `get_federation_config(agent)` - Get agent's AP config
- `store_public_key(public_key_pem)` - Store the public key provided by S2S module (private key stays in S2S module)

**AP ↔ Mewsfeed Mapping**:
- `mew_to_ap_note(mew_hash)` - Generate AP Note JSON from Mew
- `ap_note_to_mew(note_json)` - Parse AP Note, create Mew entry (for remote replies)
- `profile_to_ap_actor(agent)` - Generate AP Actor JSON from Profile
- `get_webfinger_response(username)` - Generate WebFinger JRD

**Federation State**:
- `federate_mew(mew_hash, visibility)` - Mark mew for federation
- `get_pending_federations()` - List mews awaiting delivery
- `record_delivery(mew_hash, inbox_url, success)` - Track delivery status
- `unfederate_mew(mew_hash)` - Mark as deleted

**Remote Actors**:
- `cache_remote_actor(actor_json)` - Store/update remote actor
- `get_remote_actor(actor_id)` - Retrieve cached actor
- `get_remote_actor_by_handle(handle, domain)` - Lookup by @user@domain

**Remote Relationships**:
- `create_remote_follow(remote_actor_id)` - Pending follow
- `confirm_remote_follow(follow_id)` - Mark accepted
- `create_remote_follower(remote_actor_id)` - Someone followed us
- `get_remote_followers()` / `get_remote_following()`

**Incoming Activities** (S2S module calls these after processing HTTP):
- `record_incoming_activity(activity_json)` - Store for audit
- `process_follow(actor_id)` - Create RemoteFollower entry
- `process_like(actor_id, mew_hash)` - Create RemoteInteraction
- `process_announce(actor_id, mew_hash)` - Create boost record
- `process_mention(actor_id, note_json)` - Handle remote mention

**Signals** (emitted for UI):
- `NewRemoteFollower { actor_id }`
- `RemoteLike { actor_id, mew_hash }`
- `RemoteMention { actor_id, note_id }`
- `FederationDelivered { mew_hash }`

#### Visibility Enum

```rust
pub enum FederationVisibility {
    Public,           // To: as:Public, CC: followers
    Unlisted,         // To: followers, CC: as:Public
    FollowersOnly,    // To: followers only
    Direct(Vec<String>), // To: specific actors
    HolochainOnly,    // Not federated at all
}
```

---

### 3. Generic HTTP Gateway

**Location**: Separate lightweight service (can be shared infrastructure or self-hosted)

The gateway is a **protocol-agnostic** HTTP-to-Holochain translator with NO protocol knowledge:

```
hc-http-gateway/
├── Cargo.toml
└── src/
    ├── main.rs               # HTTP server
    ├── router.rs             # Subdomain -> network routing
    ├── inbound.rs            # HTTP request -> zome call
    ├── outbound.rs           # Poll zome queue -> HTTP request
    └── config.rs             # Network registry
```

**Gateway Responsibilities** (protocol-agnostic):
1. Receive HTTP requests from the internet
2. Extract subdomain (network ID) from Host header
3. Forward request to S2S module's local HTTP endpoint
4. Return S2S module's response to caller

**Gateway Does NOT**:
- Parse JSON or understand content structure
- Call Holochain zomes directly
- Know about ActivityPub, Solid, or any protocol
- Need updates when protocols change

```
┌─────────────────────────────────────────────────────────────────┐
│                     Tauri/Electron Runtime                       │
│  ┌─────────────────┐    ┌──────────────────────────────────┐   │
│  │ activitypub     │    │   activitypub-s2s crate       │   │
│  │ zome            │◄──►│   (handles HTTP + protocol)      │   │
│  │ (pure data)     │    │                                  │   │
│  │                 │    │ - Exposes local HTTP endpoint    │   │
│  │ - entries       │    │ - Receives from gateway          │   │
│  │ - links         │    │ - Makes outbound HTTP            │   │
│  │ - validation    │    │ - Calls zome via AppWebsocket    │   │
│  │ - AP mapping    │    │                                  │   │
│  └─────────────────┘    └──────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

---

## ActivityPub S2S Module

The ActivityPub protocol logic lives in a **Rust crate embedded in the Tauri/Electron runtime**. This crate:

- Handles all HTTP requests/responses
- Implements ActivityPub S2S protocol
- Creates/verifies HTTP Signatures
- Communicates with Holochain conductor via the **app interface** (same as UI)

```
┌─────────────────────────────────────────────────────────────────────┐
│  Tauri/Electron Runtime                                             │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  activitypub-s2s crate (Rust)                             │   │
│  │                                                              │   │
│  │  - HTTP client (reqwest)                                     │   │
│  │  - HTTP Signatures (sign/verify)                             │   │
│  │  - ActivityPub JSON-LD parsing                               │   │
│  │  - WebFinger resolution                                      │   │
│  │  - Activity delivery with retries                            │   │
│  │  - Inbox polling/processing                                  │   │
│  │                                                              │   │
│  │  Communicates with conductor via AppWebsocket                │   │
│  │  (same interface as TypeScript UI code)                      │   │
│  └──────────────────────────────────────────────────────────────┘   │
│                              │                                       │
│                    AppWebsocket (same as UI)                        │
│                              │                                       │
│  ┌───────────────────────────▼──────────────────────────────────┐   │
│  │  Holochain Conductor                                          │   │
│  │                                                               │   │
│  │  activitypub zome ◄──► mews zome                              │   │
│  │  (AP model/state)      (Mewsfeed model)                       │   │
│  │                                                               │   │
│  └───────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
```

### Separation of Concerns

| Component | Responsibility |
|-----------|----------------|
| **Gateway** | Inbound HTTP relay (subdomain routing → S2S module) |
| **activitypub-s2s crate** | HTTP, signatures, AP protocol, calls zome via AppWebsocket |
| **activitypub zome** | AP state entries, AP ↔ Mewsfeed mapping |
| **mews/follows/etc zomes** | Core Mewsfeed data model |

### Why This Architecture?

1. **Holochain zomes can't do HTTP** - WASM sandbox limitation
2. **S2S module in Tauri** - Has full Rust capabilities (reqwest, crypto, etc.)
3. **AppWebsocket interface** - Same API the TypeScript UI uses
4. **Gateway is generic** - Just routes HTTP, no protocol knowledge
5. **Zome stays pure** - Entries, links, validation, mapping logic only

---

### 4. Reverse Proxy / TLS Termination

The generic HTTP gateway can either:
1. Handle TLS directly (simpler deployment)
2. Sit behind a reverse proxy for TLS termination (more flexible)

**Option A: Gateway handles TLS** (recommended for simplicity):
- Gateway listens on 443 with wildcard cert
- No separate reverse proxy needed
- Gateway extracts subdomain from Host header

**Option B: Caddy in front** (for additional features):

```caddyfile
*.mewsfeed.example {
    reverse_proxy localhost:3000
}
```

**DNS**: Wildcard A/AAAA record for `*.mewsfeed.example`

The gateway is completely generic - it just needs the subdomain to route to the correct Holochain network.

---

## Data Flow Examples

### Federate a Mew

```
1. User creates mew with federate=true
2. mews zome creates Mew entry
3. S2S module calls activitypub_zome.federate_mew(hash, Public)
4. Zome creates MewFederationState entry, returns pending state

5. S2S module handles delivery:
   - Calls zome.mew_to_ap_note(hash) → gets AP Note JSON
   - Wraps in Create activity
   - Gets keypair from zome, creates HTTP Signature
   - Gets follower inboxes (local + remote)
   - POSTs to each inbox
   - Calls zome.record_delivery(hash, inbox, success)
```

### Receive Remote Mention

```
1. Remote Mastodon user @bob@mastodon.social mentions @alice@cats.mewsfeed.example
2. Mastodon POSTs to https://cats.mewsfeed.example/users/alice/inbox
3. Gateway forwards to S2S module's local HTTP endpoint

4. S2S module processes:
   - Parses ActivityPub JSON
   - Extracts actor ID from activity
   - Fetches actor if not cached (HTTP GET)
   - Verifies HTTP Signature using actor's public key
   - Calls zome.cache_remote_actor(actor_json) if new
   - Calls zome.process_mention(actor_id, note_json)

5. Zome:
   - Records the activity
   - Emits RemoteMention signal

6. UI receives signal, shows notification
7. S2S module returns 202 Accepted to gateway
```

### Follow Remote Actor

```
1. Alice wants to follow @bob@mastodon.social
2. S2S module resolves via WebFinger + actor fetch
3. S2S module calls zome.cache_remote_actor(bob_json)
4. S2S module calls zome.create_remote_follow(bob_actor_id)
5. Zome creates RemoteFollow entry (status: Pending)

6. S2S module:
   - Builds Follow activity JSON
   - Signs with agent's keypair
   - POSTs to bob's inbox

7. Later, Mastodon sends Accept{Follow}:
   - Gateway forwards to S2S module
   - S2S module verifies signature
   - S2S module calls zome.confirm_remote_follow(follow_id)
   - Zome updates status, emits signal

8. UI shows "Now following @bob@mastodon.social"
```

### WebFinger Discovery

```
1. Remote server: GET /.well-known/webfinger?resource=acct:alice@cats.mewsfeed.example
2. Gateway forwards to S2S module

3. S2S module:
   - Parses acct: URI, extracts username
   - Calls zome.get_webfinger_response("alice")
   - Zome looks up agent, builds JRD
   - Returns WebFinger JSON

4. S2S module returns HTTP 200 with JRD to gateway
5. Gateway returns response to caller
```

### Actor Profile Request

```
1. Remote server: GET /users/alice (Accept: application/activity+json)
2. Gateway forwards to S2S module

3. S2S module:
   - Extracts agent ID from path
   - Calls zome.profile_to_ap_actor(agent)
   - Zome fetches Profile, builds AP Actor JSON
   - Returns Actor JSON

4. S2S module returns HTTP 200 with Actor JSON
```

---

## File Changes Summary

### New Files

```
crates/
├── federation_types/             # Shared state types (BOTH S2S module AND zome, WASM+native)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                # Re-exports
│       ├── state.rs              # MewFederationState, FederationVisibility
│       ├── references.rs         # RemoteActorRef, RemoteFollow, RemoteFollower
│       └── config.rs             # InstanceConfig, InviteUrlParams
│
└── activitypub-s2s/              # AP protocol types + S2S logic (native-only)
    ├── Cargo.toml
    └── src/
        ├── lib.rs                # Re-exports, constants
        ├── actor.rs              # APActor, APPublicKey, APImage
        ├── note.rs               # APNote, APTag
        ├── activity.rs           # APActivity flat struct, activity_type constants
        ├── collections.rs        # OrderedCollection, OrderedCollectionPage
        ├── webfinger.rs          # WebFingerResponse, WebFingerLink
        └── signatures.rs         # SignatureHeaders, SignatureParseError

dnas/mewsfeed/zomes/
├── integrity/activitypub/        # New integrity zome
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                # Entry types, link types
│       └── validation.rs
│
└── coordinator/activitypub/      # New coordinator zome (AP model + mapping)
    ├── Cargo.toml
    └── src/
        ├── lib.rs                # Module exports, init
        ├── config.rs             # Federation config (enabled, keypair ref)
        ├── actor.rs              # AP Actor model, map to/from Profile
        ├── note.rs               # AP Note model, map to/from Mew
        ├── activity.rs           # AP Activity types (Create, Follow, Like, etc.)
        ├── remote_actors.rs      # Cached remote AP actors
        ├── federation_state.rs   # Per-mew federation state
        ├── remote_follows.rs     # Remote follow relationships
        ├── remote_interactions.rs # Remote likes/boosts/replies
        └── signals.rs            # Signals for S2S module

crates/activitypub-s2s/        # Rust crate embedded in Tauri/Electron
├── Cargo.toml
└── src/
    ├── lib.rs                    # Public API for Tauri commands
    ├── http.rs                   # HTTP client (reqwest)
    ├── signatures.rs             # HTTP Signature sign/verify
    ├── webfinger.rs              # WebFinger resolution
    ├── actor.rs                  # Fetch/cache remote actors
    ├── delivery.rs               # Deliver activities to inboxes
    ├── inbox.rs                  # Process incoming activities
    ├── conductor.rs              # AppWebsocket client to Holochain
    └── gateway.rs                # Interface to HTTP gateway for inbound
```

### External (not in mewsfeed repo)

```
hc-http-gateway/                  # Generic, protocol-agnostic gateway
├── Cargo.toml                    # Can be shared infrastructure
└── src/
    ├── main.rs                   # HTTP server
    ├── router.rs                 # Subdomain -> network/agent routing
    └── forward.rs                # Forward to configured endpoint

Note: Gateway routes INBOUND HTTP to the activitypub-s2s
crate's local HTTP endpoint (running in Tauri/Electron).
The S2S module processes the request and calls zome via AppWebsocket.
```

### Inbound HTTP Flow

```
Internet                    Gateway                 S2S Module              Conductor
   │                           │                        │                       │
   │  POST /inbox              │                        │                       │
   │──────────────────────────►│                        │                       │
   │                           │  forward to            │                       │
   │                           │  localhost:PORT        │                       │
   │                           │───────────────────────►│                       │
   │                           │                        │  zome.cache_actor()   │
   │                           │                        │──────────────────────►│
   │                           │                        │                       │
   │                           │                        │  zome.process_...()   │
   │                           │                        │──────────────────────►│
   │                           │                        │◄──────────────────────│
   │                           │◄───────────────────────│                       │
   │  202 Accepted             │                        │                       │
   │◄──────────────────────────│                        │                       │
```

The activitypub-s2s crate exposes a local HTTP endpoint that the gateway forwards to.
This keeps the gateway completely generic - it just forwards HTTP to a configured target.

### Modified Files

```
Cargo.toml                        # Add new workspace members
dnas/mewsfeed/workdir/dna.yaml   # Add activitypub zomes
crates/mews_types/src/lib.rs      # Add FederationVisibility to FeedMew
```

---

## Implementation Phases

### Phase 1: Foundation
1. Create `federation_types` shared crate (DHT-safe, WASM+native)
2. Create `activitypub-s2s` crate with AP protocol types (native-only)
3. Define federation state types (shared between S2S module + zome)
4. Define AP types: Actor, Note, Activity, WebFinger, Signatures
5. Unit tests and Mastodon JSON fixture interoperability

### Phase 2: Local Protocol I/O (S2S Module)
Prove the S2S module works against real ActivityPub traffic before building the zome.
1. WebFinger/Actor GET: Return correct JRD and Actor JSON
2. Inbox POST: Accept a signed Follow or Create activity, validate HTTP signature, return 202
3. Integration verification with `curl -H "Accept: application/activity+json"`

### Phase 3: ActivityPub Zome
Only after Phase 2 is verified. Creates the Holochain zome pair to manage federation state on the DHT.
1. Create `activitypub_integrity` zome with entry/link types
2. Implement validation rules
3. Create `activitypub` coordinator zome
4. Implement AP ↔ Mewsfeed mapping functions
5. Implement federation state management

### Phase 4: Generic HTTP Gateway (Week 7-8)
1. Scaffold `hc-http-gateway` (separate repo, reusable)
2. Implement subdomain -> network/agent routing
3. Implement forwarding to S2S module's local endpoint
4. Test with S2S module echo handler

### Phase 5: Inbound Federation (Week 9-10)
1. S2S module: inbox endpoint handler
2. S2S module: actor fetching and caching
3. S2S module: signature verification
4. Zome: process incoming Follow/Like/Announce/Create
5. Signals for UI notifications

### Phase 6: Outbound Federation (Week 11-12)
1. S2S module: activity delivery with retries
2. Zome: federation state tracking
3. S2S module: poll and deliver pending federations
4. End-to-end: federate mew, verify on Mastodon

### Phase 7: Integration & Testing (Week 13-14)
1. Integrate with mews zome via zome-to-zome calls
2. Add remote interaction counts to FeedMew
3. UI integration for federation controls
4. Test with Mastodon, Pleroma, Misskey
5. Documentation

---

## Verification Plan

### Unit Tests
- Trait implementations for type conversions
- Validation rules for entry types
- HTTP Signature generation/verification

### Integration Tests
- Zome function roundtrips (create config, federate mew, etc.)
- Bridge service endpoints (WebFinger, Actor JSON)
- Delivery queue processing

### End-to-End Tests
1. Set up local Mastodon instance
2. Create Mewsfeed agent with federation enabled
3. Federate a mew, verify it appears on Mastodon
4. Follow Mastodon user from Mewsfeed
5. Receive mention from Mastodon, verify notification
6. Like/boost Mewsfeed post from Mastodon, verify counts

### Manual Testing
- Test with production Mastodon instances (mastodon.social)
- Test with Pleroma, Misskey, other ActivityPub implementations
- Verify WebFinger resolution works with various clients

---

## Critical Files Reference

| File | Purpose |
|------|---------|
| `crates/mews_types/src/lib.rs` | Mew, Profile, FeedMew - types to map to AP |
| `crates/federation_types/src/lib.rs` | **NEW** Shared state types (S2S module + zome) |
| `crates/activitypub-s2s/src/lib.rs` | **NEW** AP protocol types (S2S module only) |
| `dnas/mewsfeed/workdir/dna.yaml` | DNA manifest - add activitypub zomes |
| `Cargo.toml` | Workspace config - add new crates |
| `dnas/mewsfeed/zomes/integrity/mews/src/lib.rs` | Pattern for entry/link types |
| `dnas/mewsfeed/zomes/coordinator/follows/src/follower_to_creators.rs` | Follow pattern to extend |

---

## Gateway Reusability

The `hc-http-gateway` is protocol-agnostic and reusable:

### Multi-Network Support
- Single gateway instance serves unlimited Mewsfeed networks
- Subdomain routing: `network-a.mewsfeed.example`, `network-b.mewsfeed.example`
- Each network can run different hApp versions
- No gateway changes needed when hApp upgrades

### For Other hApps (Future)
- Any Holochain app needing HTTP endpoints can use this gateway
- Gateway just forwards HTTP to a configured local endpoint
- Protocol logic stays in the hApp's S2S module

---

## Security Considerations

1. **HTTP Signatures**: Generated and verified in the **S2S module** (not zome or gateway)
2. **Private keys**: Stored in S2S module (OS keychain or encrypted local storage); private key never traverses the AppWebsocket boundary. Zome stores only the public key for serving to remote actors.
3. **Rate limiting**: Gateway can implement basic IP-based limits
4. **Content sanitization**: HTML-escape mew text before AP conversion (in zome mapping)
5. **Author verification**: Only mew author can set federation state (zome validation)
6. **Gateway trust**: Gateway is trusted to faithfully relay HTTP; cannot forge signatures
7. **S2S module trust**: Has access to private keys for signing; runs in same Tauri process as UI
