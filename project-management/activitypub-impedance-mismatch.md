# ActivityPub ↔ MewsFeed/Holochain Impedance Mismatch Analysis

This document analyzes the compatibility between the ActivityPub protocol and MewsFeed's Holochain-based architecture, identifying where mappings are clear, where questions remain, and where fundamental challenges exist.

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Clear Mappings](#clear-mappings)
3. [Areas with Questions](#areas-with-questions)
4. [Fundamental Challenges](#fundamental-challenges)
5. [Concept Mapping Tables](#concept-mapping-tables)
6. [Recommendations](#recommendations)

---

## Executive Summary

ActivityPub (AP) is designed around a **server-centric federation model** with HTTP-based delivery, while MewsFeed/Holochain operates on a **peer-to-peer DHT** with gossip-based propagation. This architectural difference creates the primary impedance mismatch.

| Category | Status | Notes |
|----------|--------|-------|
| Content types | ✅ Clear | Mews map well to AP Notes |
| Interactions (likes, replies) | ✅ Clear | Activity types align |
| Identity | ⚠️ Questions | Cryptographic vs URL-based |
| Discovery | ⚠️ Questions | WebFinger vs DHT lookup |
| Delivery model | ❌ Challenge | Addressed vs gossip |
| Privacy/visibility | ❌ Challenge | AP audiences vs DHT public data |
| Mutability | ❌ Challenge | AP allows updates, MewsFeed is immutable |

---

## Clear Mappings

### 1. Content: Mew ↔ Note

MewsFeed's `Mew` maps cleanly to ActivityPub's `Note` object.

| MewsFeed | ActivityPub | Notes |
|----------|-------------|-------|
| `Mew.text` | `Note.content` | Direct mapping |
| `Mew.mew_type: Original` | `Create { Note }` | New content |
| `Action.timestamp` | `Note.published` | Creation time |
| `ActionHash` | `Note.id` | Unique identifier (needs URI conversion) |
| `AgentPubKey` (author) | `Note.attributedTo` | Author reference |

**Example Mapping:**
```rust
// MewsFeed
Mew {
    text: "Hello Fediverse! #holochain",
    links: vec![],
    mew_type: MewType::Original,
}
```
```json
// ActivityPub
{
  "@context": "https://www.w3.org/ns/activitystreams",
  "type": "Note",
  "content": "Hello Fediverse! #holochain",
  "published": "2024-01-15T10:30:00Z",
  "attributedTo": "https://bridge.example/actors/uhCAk..."
}
```

### 2. Replies

MewsFeed's reply system maps directly to AP's `inReplyTo`.

| MewsFeed | ActivityPub | Notes |
|----------|-------------|-------|
| `MewType::Reply(ActionHash)` | `Note.inReplyTo` | Reference to parent |
| `MewToResponses` links | `Note.replies` collection | Finding responses |

**Status:** ✅ Straightforward mapping. The `ActionHash` reference needs conversion to a URI.

### 3. Quotes

| MewsFeed | ActivityPub | Notes |
|----------|-------------|-------|
| `MewType::Quote(ActionHash)` | Mastodon-style `_misskey_quote` or `quoteUrl` | Non-standard but widely used |

**Status:** ✅ Clear mapping, though AP quote support is implementation-specific (Mastodon, Misskey, etc. handle differently).

### 4. Reposts/Boosts

| MewsFeed | ActivityPub | Notes |
|----------|-------------|-------|
| `MewType::Mewmew(ActionHash)` | `Announce` activity | Reshare/boost |

**Example:**
```json
{
  "type": "Announce",
  "actor": "https://bridge.example/actors/uhCAk...",
  "object": "https://bridge.example/notes/uhCEk..."
}
```

**Status:** ✅ Clear 1:1 mapping.

### 5. Likes

| MewsFeed | ActivityPub | Notes |
|----------|-------------|-------|
| `like()` call | `Like` activity | Like action |
| `unlike()` call | `Undo { Like }` | Unlike action |
| `LikerToHashes` links | `Note.likes` collection | Finding likers |
| `licks_count` | Derived from collection | Count |

**Status:** ✅ Clear mapping.

### 6. Follows

| MewsFeed | ActivityPub | Notes |
|----------|-------------|-------|
| `add_creator_for_follower()` | `Follow` activity | Start following |
| `remove_creator_for_follower()` | `Undo { Follow }` | Unfollow |
| `FollowerToCreators` links | Actor's `following` collection | Who I follow |
| `CreatorToFollowers` links | Actor's `followers` collection | Who follows me |

**Status:** ✅ Clear mapping. AP follow requires acceptance; MewsFeed does not (see Questions).

### 7. Hashtags

| MewsFeed | ActivityPub | Notes |
|----------|-------------|-------|
| `HashtagToMews` links | `Note.tag` with `type: Hashtag` | Tag references |
| Prefix index search | No direct equivalent | Discovery |

**Example:**
```json
{
  "tag": [
    {
      "type": "Hashtag",
      "href": "https://bridge.example/tags/holochain",
      "name": "#holochain"
    }
  ]
}
```

**Status:** ✅ Clear mapping for basic hashtags.

### 8. Mentions

| MewsFeed | ActivityPub | Notes |
|----------|-------------|-------|
| `LinkTarget::Mention(AgentPubKey)` | `Note.tag` with `type: Mention` | Mentioning users |
| `MentionToMews` links | No direct equivalent | Finding mentions of me |

**Status:** ✅ Clear mapping.

### 9. Cashtags

| MewsFeed | ActivityPub | Notes |
|----------|-------------|-------|
| `CashtagToMews` links | Custom tag type or hashtag | No standard AP equivalent |

**Status:** ✅ Can map as hashtags or custom extension. MewsFeed-specific feature.

---

## Areas with Questions

### 1. Identity Mapping

**MewsFeed:** Uses `AgentPubKey` (Ed25519 public key, base64-encoded hash)
**ActivityPub:** Uses HTTP URIs (e.g., `https://mastodon.social/users/alice`)

**Questions:**
- How do we create stable URIs for Holochain agents?
  - A: agent chooses a name in their profile (similarly to signing up for a Mastodon account) -tfw
- Should a bridge service maintain a mapping database?
  - A: no! -tfw

**Possible Approaches:**
1. **Bridge-hosted URIs:** `https://bridge.example/actors/{AgentPubKey}`
2. **DID-based:** `did:holo:{AgentPubKey}` with DID resolution
3. **WebFinger:** Implement WebFinger at bridge for `@user@bridge.example`
   1. yes, this will need to be done, but I expect it can be done in the S2S module -tfw

**Open Questions:**
- What happens when a Holochain agent has multiple bridge connections?
  - A: does not happen. the bridge is identified by the domain, and specifying the subdomain (initially) will include specifying the *single* bridge. -tfw
- How do we handle key rotation or agent migration?

### 2. Follow Acceptance Model

**MewsFeed:** Follows are unilateral—anyone can follow anyone instantly
**ActivityPub:** Follows require acceptance (`Accept { Follow }`)

**Questions:**
- Should the bridge auto-accept follows from Holochain agents?
- How do we handle AP accounts with locked/approval-required follows?
- Should we add follow-request functionality to MewsFeed?

**Possible Approaches:**
1. Auto-accept all Holochain → AP follows (may not be desired)
2. Queue follow requests and notify MewsFeed user somehow
3. Add optional follow-request approval to MewsFeed

### 3. Profile Mapping

**MewsFeed:**
```rust
Profile {
    nickname: String,
    fields: BTreeMap<String, String>,
}
```

**ActivityPub Actor:**
```json
{
  "type": "Person",
  "preferredUsername": "alice",
  "name": "Alice Smith",
  "summary": "<p>Bio here</p>",
  "icon": { "type": "Image", "url": "..." },
  "image": { "type": "Image", "url": "..." },
  "attachment": [
    { "type": "PropertyValue", "name": "Website", "value": "..." }
  ]
}
```

**Questions:**
- How do we map `fields` to AP `attachment` PropertyValues?
- Where does bio/summary come from in MewsFeed profiles?
- How do we handle avatar/header images?
- Should we extend MewsFeed profiles to include more AP-compatible fields?

### 4. URL References

**MewsFeed:** `LinkTarget::Url(String)` in mew links
**ActivityPub:** URLs can appear in content as `<a>` tags

**Questions:**
- Should we parse mew text for URLs and include as links?
- How do we handle link preview cards (AP `attachment` with link preview)?

### 5. Notifications

**MewsFeed Notifications:**
- `MyMewLicked`, `MyMewResponded`, `MyAgentMentioned`, `MyAgentFollowed`, etc.

**ActivityPub:** No standard notification system (implementation-specific)

**Questions:**
- How do we surface AP interactions to MewsFeed users?
- Should the bridge maintain a notification queue?

### 6. Timestamps and Ordering

**MewsFeed:** Uses Holochain `Timestamp` (microseconds since Unix epoch)
**ActivityPub:** ISO 8601 datetime strings

**Questions:**
- How do we handle clock skew between systems?
- What timestamp represents the "canonical" order when bridging?

### 7. Collections and Pagination

**MewsFeed:** Uses `HashPagination` / `AgentPubKeyPagination`
**ActivityPub:** Uses `OrderedCollection` with `first`, `next` page links

**Questions:**
- How do we map cursor-based pagination between systems?
- How do we present MewsFeed's link-based queries as AP collections?

---

## Fundamental Challenges

### 1. Delivery Model Mismatch

**This is the most significant impedance mismatch.**

| Aspect | ActivityPub | MewsFeed/Holochain |
|--------|-------------|-------------------|
| Model | Push-based, addressed delivery | Pull-based, gossip propagation |
| Delivery | HTTP POST to recipient inbox | DHT publication + gossip |
| Addressing | Explicit `to`, `cc`, `bcc` | No explicit addressing |
| Guarantee | Best-effort, retries | Eventual consistency |

**Challenges:**
- AP requires knowing recipient inboxes upfront
- MewsFeed has no concept of "sending to" specific users
- A bridge must translate gossip-discovered content into addressed deliveries

**Implications:**
- Bridge must poll/monitor Holochain for new content
- Bridge must maintain knowledge of AP followers to deliver to their inboxes
- Latency between MewsFeed post and AP delivery depends on bridge polling

### 2. Privacy and Visibility

**ActivityPub Visibility Levels:**
- **Public:** `to: ["https://www.w3.org/ns/activitystreams#Public"]`
- **Unlisted:** `cc: ["https://www.w3.org/ns/activitystreams#Public"]`
- **Followers-only:** `to: [followers collection]`
- **Direct:** `to: [specific actors]`

**MewsFeed:** All content on DHT is effectively public to network participants.

**Challenges:**
- No way to create followers-only mews in MewsFeed
- No way to create direct/private mews
- Incoming private AP content has no MewsFeed equivalent

**Implications:**
- Bridge can only share public AP content with MewsFeed
- MewsFeed content bridges as public AP content only
- True private messaging requires a separate solution

### 3. Content Mutability

**ActivityPub:** Supports `Update` activity for editing posts
**MewsFeed:** Mews are **immutable**—no updates allowed

**Challenges:**
- Incoming AP updates have no MewsFeed representation
- MewsFeed users cannot edit bridged content

**Possible Approaches:**
1. Ignore incoming updates (lose edit history)
2. Create new mew with reference to original (thread pollution)
3. Store updates in bridge database only (not on Holochain)

### 4. Deletion Semantics

**ActivityPub:** `Delete` activity removes content; servers may or may not honor
**MewsFeed:** Delete creates tombstone; content marked deleted but history preserved

| Aspect | ActivityPub | MewsFeed |
|--------|-------------|----------|
| Model | Request deletion | Tombstone marker |
| Propagation | Federated to recipients | Gossip through DHT |
| Enforcement | Server choice | Cryptographic proof |
| Recovery | Impossible if honored | History preserved |

**Challenges:**
- AP deletion is a request; MewsFeed deletion is a fact
- Holochain's append-only nature means content may still be retrievable
- "Right to be forgotten" harder to implement

### 5. Server vs Peer Identity

**ActivityPub:** Actors are hosted on servers; server vouches for actor
**Holochain:** Agents are self-sovereign; no server authority

**Challenges:**
- AP expects HTTP endpoints for actors
- Bridge must act as "server" for Holochain agents
- Trust model fundamentally different

### 6. Real-time Updates

**ActivityPub:** No standard real-time; some implement WebSocket/SSE
**MewsFeed:** Signal-based notifications within Holochain

**Challenges:**
- No standard way to push updates between systems
- Bridge must poll both systems

### 7. Content Addressing vs Location Addressing

**ActivityPub:** Content identified by URL (location-based)
**Holochain:** Content identified by hash (content-addressed)

**Challenges:**
- Same content has different identifiers
- Content at URL can change; content at hash cannot
- Bridge must maintain bidirectional ID mapping

### 8. Moderation and Blocking

**ActivityPub:**
- Server-level blocks (defederation)
- User-level blocks (mute, block)
- Domain blocks

**MewsFeed:**
- No built-in blocking/muting
- No server-level moderation (peer-to-peer)

**Challenges:**
- How does a MewsFeed user block an AP account?
- How does AP blocking affect Holochain content?
- No moderation authority in Holochain

---

## Concept Mapping Tables

### Activities

| Activity | MewsFeed Equivalent | Mapping Complexity |
|----------|--------------------|--------------------|
| `Create { Note }` | `create_mew()` with `MewType::Original` | ✅ Simple |
| `Create { Note }` (reply) | `create_mew()` with `MewType::Reply` | ✅ Simple |
| `Announce` | `create_mew()` with `MewType::Mewmew` | ✅ Simple |
| `Like` | `like()` | ✅ Simple |
| `Undo { Like }` | `unlike()` | ✅ Simple |
| `Follow` | `add_creator_for_follower()` | ⚠️ Acceptance model |
| `Undo { Follow }` | `remove_creator_for_follower()` | ✅ Simple |
| `Delete` | `delete_mew()` | ⚠️ Semantics differ |
| `Update` | ❌ Not supported | ❌ Fundamental gap |
| `Accept { Follow }` | ❌ Not applicable | N/A |
| `Reject { Follow }` | ❌ Not applicable | N/A |
| `Block` | ❌ Not implemented | ❌ Missing feature |
| `Flag` (report) | ❌ Not implemented | ❌ Missing feature |

### Objects

| AP Object | MewsFeed Equivalent | Notes |
|-----------|--------------------| ------|
| `Note` | `Mew` | Primary content type |
| `Person` | Agent + `Profile` | Identity + metadata |
| `Article` | `Mew` (long text) | No distinction in MewsFeed |
| `Image` | ❌ Not implemented | MewsFeed is text-only currently |
| `Video` | ❌ Not implemented | MewsFeed is text-only currently |
| `Document` | ❌ Not implemented | MewsFeed is text-only currently |
| `Question` (poll) | ❌ Not implemented | No poll support |

### Collections

| AP Collection | MewsFeed Equivalent | Notes |
|---------------|--------------------| ------|
| `outbox` | `AgentMews` links | Agent's posts |
| `inbox` | ❌ No direct equivalent | Push vs pull |
| `followers` | `CreatorToFollowers` links | Follower list |
| `following` | `FollowerToCreators` links | Following list |
| `liked` | `LikerToHashes` links | Liked posts |
| `replies` | `MewToResponses` links | Thread replies |

---

## Recommendations

### Short-term (MVP Bridge)

1. **Identity:** Use bridge-hosted URIs (`https://bridge.example/actors/{AgentPubKey}`)
2. **Content:** Bridge `Original` mews as public `Note` activities
3. **Interactions:** Support Like, Announce, Reply in both directions
4. **Follows:** Auto-accept follows; document limitation
5. **Visibility:** All bridged content is public
6. **Updates:** Ignore incoming `Update` activities with warning

### Medium-term Enhancements

1. **Profile enrichment:** Extend MewsFeed profiles for better AP compatibility
2. **Follow requests:** Add optional approval mechanism to MewsFeed
3. **Blocking:** Implement user-level blocking in MewsFeed
4. **Media:** Add image/media support to mews

### Long-term Considerations

1. **Privacy:** Research private entry sharing in Holochain for followers-only content
2. **DID integration:** Move toward DID-based identity for better interop
3. **Moderation:** Develop community moderation patterns for Holochain
4. **Real-time:** Implement WebSocket/SSE for bridge notifications

### Bridge Architecture Implications

The bridge must:
1. **Poll Holochain** for new mews, likes, follows
2. **Maintain state** mapping ActionHashes ↔ AP URIs
3. **Track followers** to know where to deliver activities
4. **Handle inbox** for incoming AP activities
5. **Convert formats** bidirectionally
6. **Respect rate limits** on AP servers
7. **Handle errors** gracefully (unreachable servers, etc.)

---

## Appendix: Key Protocol References

- [ActivityPub W3C Recommendation](https://www.w3.org/TR/activitypub/)
- [Activity Streams 2.0](https://www.w3.org/TR/activitystreams-core/)
- [WebFinger RFC 7033](https://tools.ietf.org/html/rfc7033)
- [HTTP Signatures](https://datatracker.ietf.org/doc/html/draft-cavage-http-signatures)
- [Holochain Core Concepts](https://developer.holochain.org/concepts/)
- [Holochain HDK Documentation](https://docs.rs/hdk/latest/hdk/)
