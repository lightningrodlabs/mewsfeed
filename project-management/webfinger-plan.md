# WebFinger Implementation Plan for MewsFeed

This document outlines the implementation plan for WebFinger (RFC 7033) support in MewsFeed's ActivityPub integration.

## Table of Contents

1. [Overview](#overview)
2. [WebFinger Protocol Summary](#webfinger-protocol-summary)
3. [Architecture Integration](#architecture-integration)
4. [Implementation Components](#implementation-components)
5. [Data Flow](#data-flow)
6. [Implementation Steps](#implementation-steps)
7. [Testing Plan](#testing-plan)
8. [Security Considerations](#security-considerations)

---

## Overview

WebFinger enables remote ActivityPub servers to discover MewsFeed users. When a Mastodon user searches for `@alice@holochain-net.mewsfeed.net`, Mastodon queries WebFinger to find Alice's Actor URL.

### User Discovery Flow

```
1. User searches: @alice@holochain-net.mewsfeed.net
2. Remote server queries: GET https://holochain-net.mewsfeed.net/.well-known/webfinger?resource=acct:alice@holochain-net.mewsfeed.net
3. MewsFeed returns: JRD with links to Actor, profile page, etc.
4. Remote server fetches Actor URL to get full profile
```

---

## WebFinger Protocol Summary

### Request Format

```
GET /.well-known/webfinger?resource=acct:{username}@{domain}
Host: {domain}
Accept: application/jrd+json
```

**Required Query Parameters:**
- `resource` - The resource to look up (must be `acct:` URI for ActivityPub)

**Optional Query Parameters:**
- `rel` - Filter returned links to specific relation types

### Response Format (JRD - JSON Resource Descriptor)

```json
{
  "subject": "acct:alice@holochain-net.mewsfeed.net",
  "aliases": [
    "https://holochain-net.mewsfeed.net/users/alice",
    "https://holochain-net.mewsfeed.net/@alice"
  ],
  "links": [
    {
      "rel": "self",
      "type": "application/activity+json",
      "href": "https://holochain-net.mewsfeed.net/users/alice"
    },
    {
      "rel": "http://webfinger.net/rel/profile-page",
      "type": "text/html",
      "href": "https://holochain-net.mewsfeed.net/@alice"
    },
    {
      "rel": "http://ostatus.org/schema/1.0/subscribe",
      "template": "https://holochain-net.mewsfeed.net/authorize_interaction?uri={uri}"
    }
  ]
}
```

### HTTP Response Codes

| Code | Meaning |
|------|---------|
| 200 | Success - JRD returned |
| 400 | Bad request - missing or malformed `resource` parameter |
| 404 | User not found or federation not enabled |
| 500 | Internal server error |

### Required Headers

**Response:**
```
Content-Type: application/jrd+json
Access-Control-Allow-Origin: *
```

---

## Architecture Integration

WebFinger is implemented across three components per the fedimew architecture:

```
│                         Components                               │
|------------------------------------------------------------
│                                                                  │
│  ┌──────────────────────┐                                       │
│  │ activitypub-s2s      │  Protocol types (WebFingerResponse,  │
│  │ crate                │  WebFingerLink)                       │
│  └──────────────────────┘                                       │
│            ▲                                                     │
│            │ imports                                             │
│     ┌──────┴──────┐                                             │
│     │             │                                             │
│  ┌──┴───────────┐ │                                             │
│  │ activitypub- │ │                                             │
│  │ s2s crate    │ │  HTTP handling, request parsing,           │
│  │              │ │  response formatting                        │
│  └──────────────┘ │                                             │
│         │         │                                             │
│         │ AppWebsocket                                          │
│         ▼         │                                             │
│  ┌──────────────┐ │                                             │
│  │ activitypub  ├─┘                                             │
│  │ zome         │    Username lookup, JRD generation            │
│  └──────────────┘                                               │
│                                                                  │
---------------------
```

---

## Implementation Components

### 1. Protocol Types (`crates/activitypub-s2s/src/webfinger.rs`)

```rust
use serde::{Deserialize, Serialize};

/// WebFinger request parsed from query parameters
#[derive(Debug, Clone)]
pub struct WebFingerQuery {
    /// The acct: URI being looked up
    pub resource: String,
    /// Optional relation type filter
    pub rel: Option<Vec<String>>,
}

impl WebFingerQuery {
    /// Parse the resource into username and domain
    /// Returns (username, domain) or error
    pub fn parse_acct(&self) -> Result<(String, String), WebFingerError> {
        // Parse "acct:alice@holochain-net.mewsfeed.net"
        let acct = self.resource.strip_prefix("acct:")
            .ok_or(WebFingerError::InvalidResource)?;
        let parts: Vec<&str> = acct.split('@').collect();
        if parts.len() != 2 {
            return Err(WebFingerError::InvalidResource);
        }
        Ok((parts[0].to_string(), parts[1].to_string()))
    }
}

/// WebFinger JRD response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebFingerResponse {
    /// The queried resource URI
    pub subject: String,

    /// Alternative URIs for the same resource
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub aliases: Vec<String>,

    /// Links to related resources
    pub links: Vec<WebFingerLink>,
}

/// A link in the JRD response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebFingerLink {
    /// Relation type (e.g., "self", "http://webfinger.net/rel/profile-page")
    pub rel: String,

    /// MIME type of the linked resource
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub link_type: Option<String>,

    /// URL of the linked resource
    #[serde(skip_serializing_if = "Option::is_none")]
    pub href: Option<String>,

    /// URI template (for subscribe links)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,

    /// Additional properties
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub properties: HashMap<String, Option<String>>,
}

/// Well-known relation types
pub mod rel {
    pub const SELF: &str = "self";
    pub const PROFILE_PAGE: &str = "http://webfinger.net/rel/profile-page";
    pub const SUBSCRIBE: &str = "http://ostatus.org/schema/1.0/subscribe";
}

/// Well-known MIME types
pub mod mime {
    pub const ACTIVITY_JSON: &str = "application/activity+json";
    pub const HTML: &str = "text/html";
    pub const JRD_JSON: &str = "application/jrd+json";
}

#[derive(Debug, Clone)]
pub enum WebFingerError {
    InvalidResource,
    UserNotFound,
    FederationDisabled,
    DomainMismatch,
    InternalError(String),
}
```

### 2. ActivityPub Zome Functions

**In `dnas/mewsfeed/zomes/coordinator/activitypub/src/webfinger.rs`:**

```rust
use hdk::prelude::*;
use activitypub_s2s::webfinger::*;

/// Input for WebFinger lookup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetWebFingerInput {
    pub username: String,
    pub domain: String,
}

/// Get WebFinger response for a username
/// Called by S2S module after parsing the HTTP request
#[hdk_extern]
pub fn get_webfinger_response(input: GetWebFingerInput) -> ExternResult<WebFingerResponse> {
    let GetWebFingerInput { username, domain } = input;

    // 1. Look up agent by username via UsernameToAgent link
    let agent = lookup_agent_by_username(&username)?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "User not found".into()
        )))?;

    // 2. Check federation is enabled for this agent
    let config = get_federation_config(agent.clone())?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Federation not enabled".into()
        )))?;

    if !config.enabled {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Federation disabled".into()
        )));
    }

    // 3. Build URLs using the domain from request
    let base_url = format!("https://{}", domain);
    let actor_url = format!("{}/users/{}", base_url, username);
    let profile_url = format!("{}/@{}", base_url, username);
    let subscribe_template = format!("{}/authorize_interaction?uri={{uri}}", base_url);

    // 4. Construct JRD response
    Ok(WebFingerResponse {
        subject: format!("acct:{}@{}", username, domain),
        aliases: vec![
            actor_url.clone(),
            profile_url.clone(),
        ],
        links: vec![
            // ActivityPub Actor (required for federation)
            WebFingerLink {
                rel: rel::SELF.to_string(),
                link_type: Some(mime::ACTIVITY_JSON.to_string()),
                href: Some(actor_url),
                template: None,
            },
            // Human-readable profile page
            WebFingerLink {
                rel: rel::PROFILE_PAGE.to_string(),
                link_type: Some(mime::HTML.to_string()),
                href: Some(profile_url),
                template: None,
            },
            // OStatus subscribe (for remote follow)
            WebFingerLink {
                rel: rel::SUBSCRIBE.to_string(),
                link_type: None,
                href: None,
                template: Some(subscribe_template),
            },
        ],
    })
}

/// Look up an agent by their registered username
fn lookup_agent_by_username(username: &str) -> ExternResult<Option<AgentPubKey>> {
    // Get links from username anchor to agent
    let path = Path::from(format!("usernames/{}", username.to_lowercase()));
    let links = get_links(
        GetLinksInputBuilder::try_new(path.path_entry_hash()?, LinkTypes::UsernameToAgent)?
            .build()
    )?;

    if let Some(link) = links.first() {
        let agent = AgentPubKey::try_from(link.target.clone())
            .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?;
        Ok(Some(agent))
    } else {
        Ok(None)
    }
}
```

**Link type for username lookups (in integrity zome):**

```rust
// In activitypub_integrity/src/lib.rs

#[hdk_link_types]
pub enum LinkTypes {
    AgentToFederationConfig,
    UsernameToAgent,  // Path("usernames/{username}") -> AgentPubKey
    // ... other link types
}
```

### 3. S2S Module HTTP Handler (`crates/activitypub-s2s/src/webfinger.rs`)

```rust
use activitypub_s2s::webfinger::*;
use crate::conductor::ConductorClient;
use crate::error::S2SError;

/// Handle incoming WebFinger request
pub async fn handle_webfinger_request(
    query: &str,
    expected_domain: &str,
    conductor: &ConductorClient,
) -> Result<WebFingerResponse, S2SError> {
    // 1. Parse query string
    let params = parse_query_string(query)?;

    let resource = params.get("resource")
        .ok_or(S2SError::BadRequest("Missing 'resource' parameter".into()))?;

    let rel_filter: Option<Vec<String>> = params.get("rel")
        .map(|r| r.split(',').map(String::from).collect());

    let query = WebFingerQuery {
        resource: resource.clone(),
        rel: rel_filter.clone(),
    };

    // 2. Parse and validate the acct: URI
    let (username, domain) = query.parse_acct()
        .map_err(|_| S2SError::BadRequest("Invalid 'resource' format".into()))?;

    // 3. Verify domain matches our expected domain
    if domain != expected_domain {
        return Err(S2SError::NotFound(format!(
            "Domain '{}' does not match expected '{}'",
            domain, expected_domain
        )));
    }

    // 4. Call zome to get WebFinger response
    let input = GetWebFingerInput { username, domain };
    let response: WebFingerResponse = conductor
        .call_zome("activitypub", "get_webfinger_response", input)
        .await
        .map_err(|e| match e {
            // Map zome errors to HTTP errors
            ZomeError::Guest(msg) if msg.contains("not found") => {
                S2SError::NotFound(msg)
            },
            ZomeError::Guest(msg) if msg.contains("disabled") => {
                S2SError::NotFound(msg)
            },
            other => S2SError::Internal(other.to_string()),
        })?;

    // 5. Filter links by rel if requested
    let response = if let Some(rels) = rel_filter {
        WebFingerResponse {
            links: response.links.into_iter()
                .filter(|link| rels.contains(&link.rel))
                .collect(),
            ..response
        }
    } else {
        response
    };

    Ok(response)
}

/// Convert WebFingerResponse to HTTP response
pub fn webfinger_to_http_response(response: WebFingerResponse) -> HttpResponse {
    HttpResponse {
        status: 200,
        headers: vec![
            ("Content-Type".into(), mime::JRD_JSON.into()),
            ("Access-Control-Allow-Origin".into(), "*".into()),
        ],
        body: serde_json::to_string(&response).unwrap(),
    }
}

/// Convert WebFinger error to HTTP response
pub fn webfinger_error_to_http_response(error: S2SError) -> HttpResponse {
    match error {
        S2SError::BadRequest(msg) => HttpResponse {
            status: 400,
            headers: vec![("Content-Type".into(), "application/json".into())],
            body: format!(r#"{{"error": "{}"}}"#, msg),
        },
        S2SError::NotFound(msg) => HttpResponse {
            status: 404,
            headers: vec![("Content-Type".into(), "application/json".into())],
            body: format!(r#"{{"error": "{}"}}"#, msg),
        },
        S2SError::Internal(msg) => HttpResponse {
            status: 500,
            headers: vec![("Content-Type".into(), "application/json".into())],
            body: format!(r#"{{"error": "Internal server error"}}"#),
        },
    }
}

fn parse_query_string(query: &str) -> Result<HashMap<String, String>, S2SError> {
    let mut params = HashMap::new();
    for pair in query.split('&') {
        if let Some((key, value)) = pair.split_once('=') {
            let key = urlencoding::decode(key)
                .map_err(|_| S2SError::BadRequest("Invalid URL encoding".into()))?;
            let value = urlencoding::decode(value)
                .map_err(|_| S2SError::BadRequest("Invalid URL encoding".into()))?;
            params.insert(key.into_owned(), value.into_owned());
        }
    }
    Ok(params)
}
```

### 4. HTTP Router Integration

**In `crates/activitypub-s2s/src/router.rs`:**

```rust
use crate::webfinger::{handle_webfinger_request, webfinger_to_http_response, webfinger_error_to_http_response};

pub async fn route_request(
   request: HttpRequest,
   domain: &str,
   conductor: &ConductorClient,
) -> HttpResponse {
    match (request.method.as_str(), request.path.as_str()) {
        // WebFinger endpoint
        ("GET", "/.well-known/webfinger") => {
            match handle_webfinger_request(&request.query, domain, conductor).await {
                Ok(response) => webfinger_to_http_response(response),
                Err(error) => webfinger_error_to_http_response(error),
            }
        },

        // Actor endpoint (fetched after WebFinger discovery)
        ("GET", path) if path.starts_with("/users/") => {
            handle_actor_request(path, conductor).await
        },

        // Inbox endpoint
        ("POST", path) if path.ends_with("/inbox") => {
            handle_inbox_request(request, conductor).await
        },

        // Other routes...
        _ => HttpResponse {
            status: 404,
            headers: vec![],
            body: "Not found".into(),
        },
    }
}
```

---

## Data Flow

### Complete WebFinger Request Flow

```
│ Remote Server (e.g., Mastodon)                                             │
│                                                                            │
│  User searches: @alice@holochain-net.mewsfeed.net                              │
│                                                                            │
                                     │
                                     │ 1. HTTP GET
                                     │    /.well-known/webfinger?resource=acct:alice@holochain-net.mewsfeed.net
                                     ▼
│ HTTP Gateway                                                               │
│                                                                            │
│  - Extracts subdomain: main                                               │
│  - Forwards to S2S module's local endpoint                                │
│                                                                            │
                                     │
                                     │ 2. Forward HTTP request
                                     ▼
│ ActivityPub S2S Module (in Tauri)                                         │
│                                                                            │
│  - Parse query: resource=acct:alice@holochain-net.mewsfeed.net                 │
│  - Extract: username=alice, domain=holochain-net.mewsfeed.net                  │
│  - Validate domain matches expected                                        │
│                                                                            │
                                     │
                                     │ 3. AppWebsocket call
                                     │    activitypub.get_webfinger_response({username, domain})
                                     ▼
│ ActivityPub Zome                                                          │
│                                                                            │
│  - Lookup: Path("usernames/alice") → UsernameToAgent link                 │
│  - Get agent's FederationConfig                                           │
│  - Verify federation enabled                                               │
│  - Build JRD with actor URL, profile URL, subscribe template              │
│                                                                            │
                                     │
                                     │ 4. Return WebFingerResponse
                                     ▼
│ ActivityPub S2S Module                                                    │
│                                                                            │
│  - Serialize to JSON                                                       │
│  - Set Content-Type: application/jrd+json                                 │
│  - Set CORS headers                                                        │
│                                                                            │
                                     │
                                     │ 5. HTTP 200 + JRD body
                                     ▼
│ HTTP Gateway → Remote Server                                              │
│                                                                            │
│  Response:                                                                 │
│  {                                                                         │
│    "subject": "acct:alice@holochain-net.mewsfeed.net",                         │
│    "links": [                                                              │
│      {                                                                     │
│        "rel": "self",                                                      │
│        "type": "application/activity+json",                               │
│        "href": "https://holochain-net.mewsfeed.net/users/alice"                │
│      },                                                                    │
│      ...                                                                   │
│    ]                                                                       │
│  }                                                                         │
│                                                                            │
└——————————————————┘
                                     │
                                     │ 6. Remote server fetches actor URL
                                     │    GET https://holochain-net.mewsfeed.net/users/alice
                                     │    Accept: application/activity+json
                                     ▼
                              (Actor endpoint)
```

---

## Implementation Steps

### Phase 1: Types (in `activitypub-s2s` crate)

1. **Create `webfinger.rs` module**
   - Define `WebFingerQuery` for parsing incoming requests
   - Define `WebFingerResponse` (JRD format)
   - Define `WebFingerLink` struct
   - Add constants for well-known relation types and MIME types
   - Add `WebFingerError` enum

2. **Add to lib.rs exports**
   ```rust
   pub mod webfinger;
   pub use webfinger::*;
   ```

### Phase 2: Zome Functions

1. **Add UsernameToAgent link type** (integrity zome)
   - Link from Path("usernames/{username}") to AgentPubKey

2. **Implement username registration** (coordinator zome)
   - When enabling federation, create UsernameToAgent link
   - Validate username uniqueness
   - Normalize username (lowercase)

3. **Implement `get_webfinger_response`** (coordinator zome)
   - Look up agent by username
   - Verify federation enabled
   - Build and return JRD

### Phase 3: S2S Module

1. **Create `webfinger.rs` module**
   - Implement request parsing
   - Implement domain validation
   - Implement zome call
   - Implement response formatting

2. **Integrate with HTTP router**
   - Route `/.well-known/webfinger` to WebFinger handler
   - Handle CORS preflight requests

3. **Error handling**
   - Map zome errors to appropriate HTTP status codes
   - Return JSON error bodies

### Phase 4: Integration

1. **Wire up S2S module to gateway**
   - Ensure gateway forwards `.well-known` paths correctly

2. **End-to-end testing**
   - Test with curl/httpie
   - Test with actual Mastodon instance

---

## Testing Plan

### Unit Tests

**Types (`activitypub-s2s/src/webfinger.rs`):**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_acct() {
        let query = WebFingerQuery {
            resource: "acct:alice@holochain-net.mewsfeed.net".to_string(),
            rel: None,
        };
        let (user, domain) = query.parse_acct().unwrap();
        assert_eq!(user, "alice");
        assert_eq!(domain, "holochain-net.mewsfeed.net");
    }

    #[test]
    fn parse_invalid_acct_no_prefix() {
        let query = WebFingerQuery {
            resource: "alice@holochain-net.mewsfeed.net".to_string(),
            rel: None,
        };
        assert!(query.parse_acct().is_err());
    }

    #[test]
    fn parse_invalid_acct_no_at() {
        let query = WebFingerQuery {
            resource: "acct:alice".to_string(),
            rel: None,
        };
        assert!(query.parse_acct().is_err());
    }

    #[test]
    fn serialize_jrd_response() {
        let response = WebFingerResponse {
            subject: "acct:alice@example.com".to_string(),
            aliases: vec!["https://example.com/users/alice".to_string()],
            links: vec![
                WebFingerLink {
                    rel: rel::SELF.to_string(),
                    link_type: Some(mime::ACTIVITY_JSON.to_string()),
                    href: Some("https://example.com/users/alice".to_string()),
                    template: None,
                },
            ],
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"subject\""));
        assert!(json.contains("application/activity+json"));
    }
}
```

**S2S Module (`activitypub-s2s/src/webfinger.rs`):**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_query_string_basic() {
        let params = parse_query_string("resource=acct%3Aalice%40example.com").unwrap();
        assert_eq!(params.get("resource").unwrap(), "acct:alice@example.com");
    }

    #[test]
    fn parse_query_string_with_rel() {
        let params = parse_query_string("resource=acct%3Aalice%40example.com&rel=self").unwrap();
        assert_eq!(params.get("rel").unwrap(), "self");
    }

    #[test]
    fn domain_mismatch_returns_not_found() {
        // Test that querying for wrong domain returns 404
    }
}
```

### Integration Tests (Tryorama)

**In `tests/src/activitypub/webfinger.test.ts`:**

```typescript
import { describe, it, expect } from 'vitest';
import { runScenario } from '@holochain/tryorama';

describe('WebFinger', () => {
  it('returns JRD for federated user', async () => {
    await runScenario(async (scenario) => {
      const alice = await scenario.addPlayerWithApp(...);

      // Enable federation with username
      await alice.cells[0].callZome({
        zome_name: 'activitypub',
        fn_name: 'enable_federation',
        payload: { username: 'alice' },
      });

      // Query WebFinger
      const response = await alice.cells[0].callZome({
        zome_name: 'activitypub',
        fn_name: 'get_webfinger_response',
        payload: {
          username: 'alice',
          domain: 'test.mewsfeed.example'
        },
      });

      expect(response.subject).toBe('acct:alice@test.mewsfeed.example');
      expect(response.links).toHaveLength(3);
      expect(response.links[0].rel).toBe('self');
      expect(response.links[0].href).toContain('/users/alice');
    });
  });

  it('returns error for non-existent user', async () => {
    await runScenario(async (scenario) => {
      const alice = await scenario.addPlayerWithApp(...);

      await expect(alice.cells[0].callZome({
        zome_name: 'activitypub',
        fn_name: 'get_webfinger_response',
        payload: { username: 'nonexistent', domain: 'test.mewsfeed.example' },
      })).rejects.toThrow('not found');
    });
  });

  it('returns error for user with federation disabled', async () => {
    await runScenario(async (scenario) => {
      const alice = await scenario.addPlayerWithApp(...);

      // Enable then disable federation
      await alice.cells[0].callZome({
        zome_name: 'activitypub',
        fn_name: 'enable_federation',
        payload: { username: 'alice' },
      });
      await alice.cells[0].callZome({
        zome_name: 'activitypub',
        fn_name: 'disable_federation',
        payload: null,
      });

      await expect(alice.cells[0].callZome({
        zome_name: 'activitypub',
        fn_name: 'get_webfinger_response',
        payload: { username: 'alice', domain: 'test.mewsfeed.example' },
      })).rejects.toThrow('disabled');
    });
  });

  it('enforces unique usernames', async () => {
    await runScenario(async (scenario) => {
      const [alice, bob] = await scenario.addPlayersWithApps([...]);

      await alice.cells[0].callZome({
        zome_name: 'activitypub',
        fn_name: 'enable_federation',
        payload: { username: 'coolname' },
      });

      await expect(bob.cells[0].callZome({
        zome_name: 'activitypub',
        fn_name: 'enable_federation',
        payload: { username: 'coolname' },
      })).rejects.toThrow('already taken');
    });
  });
});
```

### Manual Testing

1. **curl tests against running instance:**
   ```bash
   # Valid request
   curl -i "https://holochain-net.mewsfeed.net/.well-known/webfinger?resource=acct:alice@holochain-net.mewsfeed.net"

   # With rel filter
   curl -i "https://holochain-net.mewsfeed.net/.well-known/webfinger?resource=acct:alice@holochain-net.mewsfeed.net&rel=self"

   # Missing resource
   curl -i "https://holochain-net.mewsfeed.net/.well-known/webfinger"

   # Invalid resource format
   curl -i "https://holochain-net.mewsfeed.net/.well-known/webfinger?resource=invalid"

   # Non-existent user
   curl -i "https://holochain-net.mewsfeed.net/.well-known/webfinger?resource=acct:nobody@holochain-net.mewsfeed.net"
   ```

2. **Mastodon search test:**
   - On Mastodon, search for `@alice@holochain-net.mewsfeed.net`
   - Verify the profile resolves
   - Verify follow button works

---

## Security Considerations

### 1. Username Validation

- **Normalize usernames:** Convert to lowercase to prevent `Alice` vs `alice` confusion
- **Restrict characters:** Only allow `[a-z0-9_]` to prevent injection and confusion
- **Length limits:** Min 1, max 30 characters
- **Reserved names:** Block `admin`, `root`, `system`, etc.

```rust
fn validate_username(username: &str) -> Result<String, String> {
    let normalized = username.to_lowercase();

    if normalized.len() < 1 || normalized.len() > 30 {
        return Err("Username must be 1-30 characters".into());
    }

    if !normalized.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err("Username can only contain letters, numbers, and underscores".into());
    }

    const RESERVED: &[&str] = &["admin", "root", "system", "moderator", "support"];
    if RESERVED.contains(&normalized.as_str()) {
        return Err("Username is reserved".into());
    }

    Ok(normalized)
}
```

### 2. Domain Validation

- Verify the queried domain matches the expected domain
- Prevent information disclosure about other networks

### 3. Rate Limiting

- Apply rate limits at the gateway level
- Prevent enumeration attacks (probing for usernames)

### 4. CORS

- Set `Access-Control-Allow-Origin: *` per RFC 7033
- WebFinger must be publicly accessible for federation

### 5. Information Disclosure

- Only return information for users who have explicitly enabled federation
- Don't reveal existence of users who haven't opted in

---

## File Summary

| File | Purpose |
|------|---------|
| `crates/activitypub-s2s/src/webfinger.rs` | Protocol types: `WebFingerResponse`, `WebFingerLink` |
| `dnas/mewsfeed/zomes/integrity/activitypub/src/lib.rs` | `UsernameToAgent` link type |
| `dnas/mewsfeed/zomes/coordinator/activitypub/src/webfinger.rs` | `get_webfinger_response` zome function |
| `dnas/mewsfeed/zomes/coordinator/activitypub/src/config.rs` | `enable_federation` with username registration |
| `crates/activitypub-s2s/src/webfinger.rs` | HTTP handling, request parsing, response formatting |
| `crates/activitypub-s2s/src/router.rs` | Route `/.well-known/webfinger` to handler |
| `tests/src/activitypub/webfinger.test.ts` | Integration tests |

---

## Dependencies

### Rust Crates

```toml
# In activitypub-s2s/Cargo.toml
[package]
name = "activitypub-s2s"
version = "0.0.1"
edition = "2021"

[lib]
name = "activitypub_s2s"

[dependencies]
serde = { workspace = true, features = ["derive"] }
serde_json = "1"
```

---

## References

- [RFC 7033 - WebFinger](https://tools.ietf.org/html/rfc7033)
- [ActivityPub W3C Recommendation](https://www.w3.org/TR/activitypub/)
- [Mastodon WebFinger implementation](https://docs.joinmastodon.org/spec/webfinger/)
- [fedimew.md](./fedimew.md) - Overall ActivityPub architecture
- [activitypub-impedance-mismatch.md](./activitypub-impedance-mismatch.md) - Protocol mapping analysis
