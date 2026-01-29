# Review: FediMew Prime Challenges and First Deliverables

This document reviews the ActivityPub S2S integration plan (`project-management/fedimew.md`), focusing on the prime challenges and Phase 1 deliverables.

## Key Decisions

| Decision | Choice |
|----------|--------|
| JSON-LD Processing | S2S module only; zome handles state, not protocol serialization |
| Phase 1 Scope | Types + Unit Tests |
| Gateway Routing | S2S module connects out; bidirectional channel for inbound relay |
| Instance Model | Collective: hApp network shares responsibility for serving one fediverse instance |

---

## Prime Challenges

### 1. WASM Sandbox Limitation (Critical)

**Challenge:** Holochain zomes run in a WASM sandbox and cannot make HTTP requests. All ActivityPub protocol operations requiring network access must live outside the DNA.

**Impact:** This forces a split architecture:
- **activitypub zome** - Federation *state* only (what must be consistent across nodes)
- **activitypub-s2s module** - All protocol logic: HTTP, JSON-LD, signatures, WebFinger, delivery

**Zome Responsibilities (state that requires DHT consensus):**
- Federation visibility per mew (Public, Unlisted, FollowersOnly, etc.)
- Delivery status tracking (which inboxes received which content)
- Remote actor references (not full AP Actor JSON, just IDs and minimal metadata)
- Remote follow/follower relationships
- Mapping between local ActionHashes and AP URIs

**S2S Module Responsibilities (protocol, no DHT needed):**
- JSON-LD parsing and serialization
- ActivityPub type construction (Actor, Note, Activity)
- HTTP Signatures (sign/verify)
- WebFinger resolution
- Inbox delivery with retries
- Actor fetching and caching (local cache, not DHT)

**Mitigation:** The plan correctly places all protocol logic in a Rust crate embedded in Tauri/Electron, communicating with the conductor via AppWebsocket (same pattern as the UI).

---

### 2. Shared Types Dual-Target Compilation

**Challenge:** The `federation_types` crate must compile to both:
- **WASM** (`wasm32-unknown-unknown`) for use in zomes
- **Native** for use in the S2S module

**Simplified by Architecture:**

Since the shared crate only contains simple state types (not AP protocol types), the risk is minimal:
- No JSON-LD library needed in shared crate
- No HTTP/crypto dependencies in shared crate
- Only `hdk`, `serde`, and `holochain_serialized_bytes`

**Mitigation:**
- Keep shared crate minimal: only state types that need DHT consensus
- All protocol complexity (JSON-LD, signatures) stays in S2S module
- Follow existing pattern from `mews_types`: `crate-type = ["cdylib", "rlib"]`

---

### 3. HTTP Signatures Key Management

**Challenge:** ActivityPub S2S requires HTTP Signatures for authentication:
- Private keys must be stored securely
- Signing and verification happen in the S2S module
- Remote actors' public keys must be fetched and cached

**Solution:** Keypair lives entirely within the S2S module.

**Architecture:**
- S2S module generates and stores its own keypair (e.g., in OS keychain or encrypted local storage)
- Private key never traverses the AppWebsocket boundary
- Public key is provided to the zome for inclusion in Actor JSON responses
- Zome only stores the public key (for serving to remote actors)

**Benefits:**
- Simplified security model (no private key in DHT, even as private entry)
- Faster signing (no round-trip to conductor)
- S2S module has full control over key lifecycle
- Aligns with principle of keeping crypto operations in the S2S module

---

### 4. JSON-LD and Protocol Serialization

**Challenge:** ActivityPub uses JSON-LD with:
- `@context` declarations
- Compaction/expansion algorithms
- Type coercion

**Decision:** JSON-LD processing happens exclusively in the S2S module.

**Separation of Concerns:**
- **S2S module** handles all ActivityPub protocol serialization (JSON-LD, Actor, Note, Activity types)
- **Zome** manages federation *state* that must be known across nodes (visibility, delivery status, remote actor references), and AP↔Mewsfeed mapping
- Communication between S2S module and zome uses simple state types, not AP protocol types

**Benefits:**
- No WASM-compatible JSON-LD library needed
- Zome stays focused on distributed state coordination and AP↔Mewsfeed mapping
- Protocol changes don't require DNA updates
- Cleaner separation: S2S owns protocol, zome owns consensus state

**Libraries (S2S module only):**
- `json-ld` crate for full JSON-LD processing
- Native Rust crypto libraries (no WASM constraints)

---

### 5. Bidirectional Federation State Management

**Challenge:** Federation requires tracking:
- **Outbound:** Pending deliveries, retry queues, delivery status per inbox
- **Inbound:** Processed activities, deduplication, actor caching

**Complexity:**
- Multiple delivery targets per activity (all follower inboxes)
- Retry logic with backoff
- Race conditions between S2S module and zome state

**Mitigation:** The plan defines clear entry types (`MewFederationState`, `RemoteActor`, etc.) and separates concerns: zome tracks state, S2S module handles delivery mechanics.

---

### 6. Gateway Routing Architecture

**Challenge:** The generic HTTP gateway must:
- Route based on subdomain (`network-a.mewsfeed.example`)
- Forward to the correct S2S module instance
- Handle TLS termination

**Solution:** S2S module initiates outbound connection to gateway.

**Architecture:**
```
S2S Module (in Tauri) ──outbound──► Gateway ◄── Fediverse
         │                              │
         └──────── bidirectional ───────┘
```

**Key Design:**
- S2S module connects OUT to the gateway (NAT traversal is a non-issue)
- Gateway maintains persistent bidirectional connection per agent
- Inbound fediverse requests are relayed through this channel
- Protocol encapsulates keep-alive and timeouts (TCP-like semantics)
- Future: unified push service may reduce need for persistent connections

**Benefits:**
- No port forwarding or public IP required for users
- Gateway is the only public-facing component
- S2S module controls connection lifecycle

---

### 7. Collective Instance Bootstrapping

**Concept:** The hApp network collectively serves a single fediverse instance.

Unlike traditional ActivityPub servers where one server = one instance, in FediMew:
- All nodes in the Holochain network share responsibility for serving the fediverse instance
- The instance has one subdomain (e.g., `cats.mewsfeed.example`)
- Any online node can respond to inbound federation requests via the gateway

**Invite URL Configuration:**

On hApp installation, users may optionally provide an **invite URL** that configures:

1. **Keypair** - The shared signing keypair for the collective instance
2. **Subdomain** - The fediverse instance subdomain (e.g., `cats` for `cats.mewsfeed.example`)
3. **Gateway endpoint** - Where to establish the bidirectional connection

**Invite URL Structure:**
```
mewsfeed://join?
  gateway=wss://gateway.mewsfeed.example
  &subdomain=cats
  &key=<encrypted-keypair-or-derivation-seed>
```

**Bootstrapping Flow:**
1. User installs hApp
2. User enters invite URL (or creates new instance if none provided)
3. S2S module extracts configuration from URL
4. S2S module derives/decrypts keypair and stores locally
5. S2S module connects to gateway, registers for the subdomain
6. Node is now part of the collective instance, ready to serve federation requests

**Benefits:**
- Easy onboarding: share a link to join the network's fediverse presence
- Distributed responsibility: any node can handle inbound requests
- Single identity: the network appears as one fediverse instance to the outside world
- No single point of failure: instance survives as long as any node is online

**Security Considerations:**
- Invite URL contains sensitive key material (must be shared securely)
- Consider key derivation from a seed + network DNA hash
- Keypair rotation requires coordinated update across nodes

---

### 8. Testing Infrastructure

**Challenge:** End-to-end testing requires:
- Local Mastodon/Pleroma instances
- DNS/TLS setup for federation
- Multiple Holochain agents

**Recommendation:**
- Use Docker Compose for local Mastodon
- Mock HTTP responses for unit/integration tests
- Reserve E2E federation tests for CI with proper infrastructure

---

## First Deliverables Analysis (Phase 1: Foundation)

Phase 1 focuses on two distinct type locations:
1. **`federation_types` shared crate** - State types needed by both S2S module and zome (for DHT consensus)
2. **S2S module internal types** - AP protocol types (Actor, Note, Activity, etc.) that never touch the zome

### Deliverable 1.1: Federation State Crate Scaffolding

**Files to create:**
```
crates/federation_types/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── state.rs
    ├── references.rs
    └── config.rs
```

**Cargo.toml pattern** (following `mews_types`):
```toml
[package]
name = "federation_types"
version = "0.0.1"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
hdk = { workspace = true }
serde = { workspace = true }
holochain_serialized_bytes = { workspace = true }
```

**Note:** No `serde_json` needed - these are simple state types, not JSON-LD.

**Workspace updates** (`Cargo.toml`):
```toml
[workspace.dependencies.federation_types]
path = "crates/federation_types"
```

---

### Deliverable 1.2: Federation State Types

**File:** `crates/federation_types/src/state.rs`

These types track federation state that must be consistent across DHT nodes:

```rust
use hdk::prelude::*;

/// Per-mew federation state (stored in zome)
#[derive(Serialize, Deserialize, SerializedBytes, Debug, Clone)]
pub struct MewFederationState {
    pub mew_hash: ActionHash,
    pub ap_uri: Option<String>,  // e.g., "https://cats.mewsfeed.example/notes/abc123"
    pub visibility: FederationVisibility,
}

/// How a mew should be federated
#[derive(Serialize, Deserialize, SerializedBytes, Debug, Clone, PartialEq)]
pub enum FederationVisibility {
    Public,              // To: as:Public
    Unlisted,            // CC: as:Public (not in public timelines)
    FollowersOnly,       // To: followers collection only
    Direct(Vec<String>), // To: specific actor URIs
    HolochainOnly,       // Not federated
}

/// Delivery tracking (S2S module reports back to zome)
#[derive(Serialize, Deserialize, SerializedBytes, Debug, Clone)]
pub struct DeliveryRecord {
    pub mew_hash: ActionHash,
    pub inbox_uri: String,
    pub status: DeliveryStatus,
    pub last_attempt: Timestamp,
}

#[derive(Serialize, Deserialize, SerializedBytes, Debug, Clone, PartialEq)]
pub enum DeliveryStatus {
    Pending,
    Delivered,
    Failed { error: String, attempts: u32 },
}
```

---

### Deliverable 1.3: Remote Actor References

**File:** `crates/federation_types/src/references.rs`

Minimal references to remote actors (not full AP Actor JSON):

```rust
use hdk::prelude::*;

/// Lightweight reference to a remote fediverse actor
/// Full actor data cached in S2S module, not DHT
#[derive(Serialize, Deserialize, SerializedBytes, Debug, Clone)]
pub struct RemoteActorRef {
    pub actor_uri: String,       // e.g., "https://mastodon.social/users/alice"
    pub handle: String,          // e.g., "@alice@mastodon.social"
    pub display_name: Option<String>,
}

/// Remote follow relationship (stored in zome for DHT consistency)
#[derive(Serialize, Deserialize, SerializedBytes, Debug, Clone)]
pub struct RemoteFollow {
    pub local_agent: AgentPubKey,
    pub remote_actor_uri: String,
    pub status: RemoteFollowStatus,
    pub created_at: Timestamp,
}

#[derive(Serialize, Deserialize, SerializedBytes, Debug, Clone, PartialEq)]
pub enum RemoteFollowStatus {
    Pending,   // Follow sent, awaiting Accept
    Accepted,  // Remote actor accepted
    Rejected,  // Remote actor rejected
}

/// Remote follower (someone from fediverse following a local agent)
#[derive(Serialize, Deserialize, SerializedBytes, Debug, Clone)]
pub struct RemoteFollower {
    pub local_agent: AgentPubKey,
    pub remote_actor_uri: String,
    pub followed_at: Timestamp,
}

/// Remote interaction on a local mew
#[derive(Serialize, Deserialize, SerializedBytes, Debug, Clone)]
pub struct RemoteInteraction {
    pub mew_hash: ActionHash,
    pub actor_uri: String,
    pub interaction_type: RemoteInteractionType,
    pub activity_uri: String,  // For Undo operations
    pub timestamp: Timestamp,
}

#[derive(Serialize, Deserialize, SerializedBytes, Debug, Clone, PartialEq)]
pub enum RemoteInteractionType {
    Like,
    Announce,  // Boost/reblog
    Reply { note_uri: String },
}
```

---

### Deliverable 1.4: Instance Configuration Types

**File:** `crates/federation_types/src/config.rs`

Types for the collective instance configuration:

```rust
use hdk::prelude::*;

/// Configuration for the collective fediverse instance
/// Derived from invite URL on hApp installation
#[derive(Serialize, Deserialize, SerializedBytes, Debug, Clone)]
pub struct InstanceConfig {
    pub subdomain: String,           // e.g., "cats"
    pub gateway_url: String,         // e.g., "wss://gateway.mewsfeed.example"
    pub instance_uri: String,        // e.g., "https://cats.mewsfeed.example"
    pub public_key_id: String,       // Key ID for HTTP Signatures
}

/// Parsed invite URL components (used during bootstrapping)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InviteUrlParams {
    pub gateway: String,
    pub subdomain: String,
    pub key_material: String,  // Encrypted or derivation seed
}
```

---

### Deliverable 1.5: AP Protocol Types (S2S Module Only)

**Location:** `crates/activitypub-s2s/src/types/` (NOT shared with zome)

These types exist only in the S2S module for protocol handling:

```rust
// actor.rs - Full AP Actor for JSON-LD serialization
pub struct APActor { /* full AP fields */ }

// note.rs - AP Note for JSON-LD serialization
pub struct APNote { /* full AP fields with @context */ }

// activity.rs - AP Activities
pub enum APActivity { Create, Follow, Like, Announce, Undo, Delete, Accept, Reject }

// webfinger.rs - WebFinger JRD
pub struct WebFingerResponse { /* JRD fields */ }

// signatures.rs - HTTP Signature types
pub struct SignatureHeaders { /* signature fields */ }
```

**Key Point:** These types handle JSON-LD `@context`, complex nested objects, and protocol-specific serialization. They never cross the AppWebsocket boundary to the zome.

---

## Recommended Phase 1 Implementation Order

1. **`federation_types` crate scaffolding** - Create directory, Cargo.toml, update workspace
2. **Federation state types** - MewFederationState, FederationVisibility, DeliveryRecord
3. **Remote actor references** - RemoteActorRef, RemoteFollow, RemoteFollower, RemoteInteraction
4. **Instance configuration types** - InstanceConfig, InviteUrlParams
5. **Unit tests for shared types** - Serialization roundtrips, enum coverage
6. **S2S module type scaffolding** - AP protocol types (Actor, Note, Activity, WebFinger, Signatures)
7. **S2S module JSON-LD tests** - Mastodon/Pleroma interoperability

---

## Phase 1 Testing Strategy

**Scope:** Unit tests for both shared types and S2S module types.

### Shared Crate Tests (`federation_types`)

**Test Categories:**

1. **Serialization Roundtrips:**
   - Serialize to bytes, deserialize back, compare
   - Test all enum variants

2. **ActionHash Handling:**
   - Verify ActionHash serialization works correctly
   - Test with mock hashes

**Test File Structure:**
```
crates/federation_types/
├── src/
│   └── ...
└── tests/
    ├── state_tests.rs
    ├── references_tests.rs
    └── config_tests.rs
```

### S2S Module Tests (`activitypub-s2s`)

**Test Categories:**

1. **JSON-LD Processing:**
   - Context expansion tests
   - Compaction tests
   - Known context handling

2. **Interoperability Tests:**
   - Parse actual Mastodon Actor JSON
   - Parse actual Mastodon Note JSON
   - Generate JSON that Mastodon accepts

3. **HTTP Signature Tests:**
   - Signature generation
   - Signature verification
   - Known test vectors

**Test File Structure:**
```
crates/activitypub-s2s/
├── src/
│   └── types/
└── tests/
    ├── actor_tests.rs
    ├── note_tests.rs
    ├── activity_tests.rs
    ├── signature_tests.rs
    └── fixtures/
        ├── mastodon_actor.json
        └── mastodon_note.json
```

---

## Verification Plan

After Phase 1 implementation:

1. **Shared crate build verification:**
   ```bash
   cargo build -p federation_types
   cargo build -p federation_types --target wasm32-unknown-unknown
   ```

2. **S2S module build verification:**
   ```bash
   cargo build -p activitypub-s2s
   ```

3. **Test execution:**
   ```bash
   cargo test -p federation_types
   cargo test -p activitypub-s2s
   ```

4. **Integration check:**
   - Create a test zome that imports `federation_types`
   - Verify it compiles to WASM
   - Verify types serialize through Holochain's `SerializedBytes`

5. **Interop check:**
   - Fetch a real Mastodon actor and parse with S2S types
   - Verify WebFinger resolution works against mastodon.social
