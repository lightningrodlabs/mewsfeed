# 0697A: `federation_types` Crate Initialization

Phase 1, Step 1 of the ActivityPub (FediMew) integration.

## What Was Done

Created the `federation_types` shared crate at `crates/federation_types/`. This crate contains federation state types that compile to both WASM (for use in Holochain zomes) and native Rust (for use in the S2S module). It does **not** contain ActivityPub protocol types (Actor, Note, Activity, etc.) — those will live exclusively in the `activitypub-s2s` module per the phase1 review.

### Files Created

| File | Purpose |
|------|---------|
| `crates/federation_types/Cargo.toml` | Crate manifest following `mews_types` conventions |
| `crates/federation_types/src/lib.rs` | Module re-exports (`state`, `references`, `config`) |
| `crates/federation_types/src/state.rs` | Per-mew federation state and visibility types |
| `crates/federation_types/src/references.rs` | Remote actor references and interaction types |
| `crates/federation_types/src/config.rs` | Collective instance configuration types |

### Files Modified

| File | Change |
|------|--------|
| `Cargo.toml` (workspace root) | Added `[workspace.dependencies.federation_types]` entry |

No other existing files were modified. The `crates/*` glob in workspace `members` auto-discovers the new crate.

## Types Defined

### `state.rs` — Federation State

- `MewFederationState` — per-mew state: `mew_hash`, `ap_uri`, `visibility`
- `FederationVisibility` — enum: `Public`, `Unlisted`, `FollowersOnly`, `Direct(Vec<String>)`, `HolochainOnly`

Delivery tracking is intentionally excluded from the DHT. Only *intent to federate* (via `visibility`) is recorded here. Each agent's S2S module takes primary responsibility for federating that agent's posts, and uses heuristics to infer federation status for mews from other agents.

### `references.rs` — Remote Actor References

- `RemoteActorRef` — lightweight reference: `actor_uri`, `handle`, `display_name`
- `RemoteFollow` — local agent following remote actor: `local_agent`, `remote_actor_uri`, `status`, `created_at`
- `RemoteFollowStatus` — enum: `Pending`, `Accepted`, `Rejected`
- `RemoteFollower` — remote actor following local agent: `local_agent`, `remote_actor_uri`, `followed_at`
- `RemoteInteraction` — remote interaction on local mew: `mew_hash`, `actor_uri`, `interaction_type`, `activity_uri`, `timestamp`
- `RemoteInteractionType` — enum: `Like`, `Announce`, `Reply { note_uri }`

### `config.rs` — Instance Configuration

- `InstanceConfig` — collective instance config: `subdomain`, `gateway_url`, `instance_uri`, `public_key_id` (derives `SerializedBytes`)
- `InviteUrlParams` — parsed invite URL: `gateway`, `subdomain`, `key_material` (does **not** derive `SerializedBytes`; used only by the S2S module)

## Conventions Followed

- Derive pattern: `#[derive(Serialize, Deserialize, SerializedBytes, Debug, Clone)]` for DHT types; enums also derive `PartialEq, Eq`
- Dependencies via workspace: `hdk`, `serde`, `holochain_serialized_bytes`
- `crate-type = ["cdylib", "rlib"]` for dual WASM/native compilation
- `serde_json` as dev-dependency only (for `InviteUrlParams` roundtrip test)
- 100-char line width per `rustfmt.toml`

## Unit Tests

13 tests covering serialization roundtrips for all types and all enum variants, plus edge cases (empty vectors, `None` optionals):

| Module | Tests |
|--------|-------|
| `state` | `mew_federation_state_roundtrip`, `mew_federation_state_no_uri_roundtrip`, `federation_visibility_all_variants_roundtrip`, `federation_visibility_direct_empty_vec` |
| `references` | `remote_actor_ref_roundtrip`, `remote_actor_ref_no_display_name`, `remote_follow_roundtrip`, `remote_follow_status_all_variants_roundtrip`, `remote_follower_roundtrip`, `remote_interaction_roundtrip`, `remote_interaction_type_all_variants_roundtrip` |
| `config` | `instance_config_roundtrip`, `invite_url_params_json_roundtrip` |

## Validation

Run these commands to verify the crate (from the repo root, inside `nix develop`):

```bash
# Native build
cargo build -p federation_types

# WASM build (confirms dual-target compilation)
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build -p federation_types --target wasm32-unknown-unknown

# Unit tests (13 tests)
cargo test -p federation_types

# Formatting check
cargo fmt -p federation_types -- --check

# Linting
cargo clippy -p federation_types -- -D warnings
```

All five commands should exit 0 with no warnings or errors.
