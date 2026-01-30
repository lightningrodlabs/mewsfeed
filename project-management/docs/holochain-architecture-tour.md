# MewsFeed Holochain Architecture Tour

This document provides a comprehensive tour of how MewsFeed utilizes Holochain's features and capabilities to build a decentralized Twitter-like social media application.

## Table of Contents

1. [Entry Types and Validation](#1-entry-types-and-validation)
2. [Links and Anchors](#2-links-and-anchors)
3. [DHT Operations](#3-dht-operations)
4. [Cross-Zome Calls](#4-cross-zome-calls)
5. [Source Chain Operations](#5-source-chain-operations)
6. [Paths and Prefix Indexing](#6-paths-and-prefix-indexing)
7. [Pagination Patterns](#7-pagination-patterns)
8. [Authorization Patterns](#8-authorization-patterns)
9. [Notification System](#9-notification-system)
10. [Architecture Summary](#10-architecture-summary)

---

## 1. Entry Types and Validation

### Entry Definitions

MewsFeed defines its core domain types in shared crates using the `#[hdk_entry_helper]` macro:

**`crates/mews_types/src/lib.rs`:**
```rust
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Mew {
    pub text: Option<String>,
    pub links: Option<Vec<String>>,
    pub mew_type: MewType,
}

pub enum MewType {
    Original,
    Reply(ActionHash),
    Quote(ActionHash),
    Mewmew(ActionHash),
}
```

The `Mew` entry supports four variants:
- **Original** - A new standalone mew
- **Reply** - A reply to another mew (references parent via ActionHash)
- **Quote** - A quote with commentary (references original)
- **Mewmew** - A repost/retweet (references original)

### Validation Rules

Validation happens in integrity zomes. MewsFeed implements several validation patterns:

**Character Limit Validation** (`dnas/mewsfeed/zomes/integrity/mews/src/mew.rs:5-29`):
```rust
pub fn validate_create_mew(action: EntryCreationAction, mew: Mew) -> ExternResult<ValidateCallbackResult> {
    // Load DNA properties for configurable limits
    let dna_info = dna_info()?;
    let properties = DnaProperties::try_from(dna_info.modifiers.properties)?;

    if let Some(text) = mew.text {
        if text.chars().count() < properties.mew_characters_min {
            return Ok(ValidateCallbackResult::Invalid(
                format!("Mew must be at least {} characters", properties.mew_characters_min)
            ));
        }
    }
    Ok(ValidateCallbackResult::Valid)
}
```

**Mewmew Deduplication via Source Chain** (`dnas/mewsfeed/zomes/integrity/mews/src/mew.rs:31-89`):

This is a sophisticated validation that scans the author's source chain to prevent duplicate retweets:

```rust
// Prevent same agent from mewmewing the same mew twice
if *action.action_seq() > 5 {
    let agent_activity = must_get_agent_activity(
        action.author().clone(),
        ChainFilter::new(action.prev_action().clone()).include_cached_entries(),
    )?;

    for registered_agent_activity in agent_activity {
        // Filter to only Mew entries in this zome
        if let Action::Create(create) = registered_agent_activity.action.action() {
            if create.entry_type == EntryType::App(AppEntryDef {
                entry_index: EntryDefIndex(0),
                zome_index,
                visibility: EntryVisibility::Public,
            }) {
                // Check if any previous mewmew targets same original
                let existing_mew: Mew = registered_agent_activity
                    .cached_entry.as_ref()
                    .unwrap().as_content().try_into()?;

                if let MewType::Mewmew(existing_target) = existing_mew.mew_type {
                    if existing_target == *mewmew_original_hash {
                        return Ok(ValidateCallbackResult::Invalid(
                            "Already mewmewed this mew".into()
                        ));
                    }
                }
            }
        }
    }
}
```

**Delete Authorization** (`dnas/mewsfeed/zomes/integrity/mews/src/mew.rs:102-113`):
```rust
pub fn validate_delete_mew(
    action: Delete,
    original_action: EntryCreationAction,
    _original_mew: Mew,
) -> ExternResult<ValidateCallbackResult> {
    if action.author != *original_action.author() {
        return Ok(ValidateCallbackResult::Invalid(
            "Only the author of a Mew can delete it".into()
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}
```

**Update Prevention** (`dnas/mewsfeed/zomes/integrity/mews/src/mew.rs:96-100`):
```rust
pub fn validate_update_mew(...) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Invalid("Mews cannot be updated".into()))
}
```

---

## 2. Links and Anchors

MewsFeed uses links extensively for indexing and relationships.

### Link Type Definitions

**`dnas/mewsfeed/zomes/integrity/mews/src/lib.rs:37-49`:**
```rust
#[hdk_link_types]
pub enum LinkTypes {
    AllMews,           // Global discovery anchor
    AgentMews,         // Agent -> their mews
    PrefixIndex,       // For tag search/autocomplete
    MewToResponses,    // Mew -> replies/quotes/mewmews
    HashtagToMews,     // #hashtag -> mews
    CashtagToMews,     // $cashtag -> mews
    MentionToMews,     // @mention -> mews
}
```

### Anchor Pattern with Paths

MewsFeed uses the `Path` type to create deterministic anchors for global discovery:

**`dnas/mewsfeed/zomes/coordinator/mews/src/mew.rs:14-20`:**
```rust
pub fn create_mew(mew: Mew) -> ExternResult<ActionHash> {
    let mew_hash = create_entry(&EntryTypes::Mew(mew.clone()))?;

    // Link from global "all_mews" anchor
    let path = Path::from("all_mews");
    create_link(
        path.path_entry_hash()?,
        mew_hash.clone(),
        LinkTypes::AllMews,
        ()
    )?;

    // Link from author's agent pubkey
    let my_agent_pub_key = agent_info()?.agent_initial_pubkey;
    create_link(
        my_agent_pub_key,
        mew_hash.clone(),
        LinkTypes::AgentMews,
        ()
    )?;

    Ok(mew_hash)
}
```

### Bidirectional Links

For efficient querying in both directions, MewsFeed creates bidirectional links:

**Likes** (`dnas/mewsfeed/zomes/coordinator/likes/src/liker_to_hashes.rs:10-25`):
```rust
pub fn add_hash_for_liker(input: AddHashForLikerInput) -> ExternResult<()> {
    // Liker -> Hash (what did this user like?)
    create_link(
        input.base_liker.clone(),
        input.target_hash.clone(),
        LinkTypes::LikerToHashes,
        ()
    )?;

    // Hash -> Liker (who liked this mew?)
    create_link(
        input.target_hash,
        input.base_liker,
        LinkTypes::HashToLikers,
        ()
    )?;

    Ok(())
}
```

The same pattern is used for:
- **Follows**: Follower -> Creators and Creator -> Followers
- **Pins**: Pinner -> Hashes and Hash -> Pinners

### Link Tags for Filtering

Link tags store metadata to enable filtering without fetching entries:

**`dnas/mewsfeed/zomes/coordinator/mews/src/mew_to_responses.rs:14-24`:**
```rust
pub fn add_response_for_mew(input: AddResponseForMewInput) -> ExternResult<()> {
    // Store response type in link tag for filtering
    let tag_bytes = SerializedBytes::try_from(input.response_type)?;

    create_link(
        input.base_original_mew_hash,
        input.target_response_mew_hash,
        LinkTypes::MewToResponses,
        LinkTag(tag_bytes.bytes().to_vec()),
    )?;
    Ok(())
}
```

This allows querying only replies, only quotes, or only mewmews without fetching all responses.

### Link Validation

Links have their own validation rules:

**`dnas/mewsfeed/zomes/integrity/mews/src/agent_mews.rs:3-42`:**
```rust
pub fn validate_create_link_agent_mews(
    action: CreateLink,
    base_address: AnyLinkableHash,
    target_address: AnyLinkableHash,
) -> ExternResult<ValidateCallbackResult> {
    // Target must be a valid Mew entry
    let action_hash = target_address.into_action_hash()
        .ok_or(wasm_error!("Target must be ActionHash"))?;
    let record = must_get_valid_record(action_hash)?;
    let _mew: Mew = record.entry().to_app_option()
        .map_err(|e| wasm_error!(e))?
        .ok_or(wasm_error!("Target must be Mew entry"))?;

    // Only the author can link their own mews
    if AnyLinkableHash::from(action.author) != base_address {
        return Ok(ValidateCallbackResult::Invalid(
            "Only the mew author can create AgentMews links".into()
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}
```

---

## 3. DHT Operations

### Create Operations

Creating entries and links in the DHT:

```rust
// Create entry
let mew_hash = create_entry(&EntryTypes::Mew(mew))?;

// Create link
create_link(base, target, LinkTypes::AllMews, ())?;
```

### Get Operations with Strategies

MewsFeed supports configurable data freshness via `GetStrategy`:

**`crates/hc_zome_input/src/lib.rs:14-28`:**
```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ZomeFnInput<T> {
    pub input: T,
    pub local: Option<bool>,  // Query local chain only or network?
}

impl<T> ZomeFnInput<T> {
    pub fn get_strategy(&self) -> GetStrategy {
        match self.local {
            Some(true) | None => GetStrategy::Local,  // Default: local only
            Some(false) => GetStrategy::Network,       // Fetch from network
        }
    }
}
```

**Usage** (`dnas/mewsfeed/zomes/coordinator/mews/src/agent_mews.rs:46-66`):
```rust
pub fn get_mews_for_agent(input: ZomeFnInput<GetAgentMewsInput>) -> ExternResult<Vec<FeedMew>> {
    let links = get_links(
        GetLinksInputBuilder::try_new(input.input.agent, LinkTypes::AgentMews)?
            .get_options(GetStrategy::from(input.get_strategy()))
            .build()
    )?;
    // ...
}
```

### Get with Details

To check if an entry has been deleted:

**`dnas/mewsfeed/zomes/coordinator/mews/src/mew_with_context.rs:16-75`:**
```rust
pub fn get_mew_with_context(mew_hash: ActionHash) -> ExternResult<Option<FeedMew>> {
    // Get full details including delete history
    let Some(details) = get_details(mew_hash.clone(), GetOptions::default())? else {
        return Ok(None);
    };

    let Details::Record(RecordDetails { record, deletes, .. }) = details else {
        return Err(wasm_error!("Expected Record details"));
    };

    // Extract deletion timestamp if deleted
    let deleted_timestamp = deletes.first().map(|delete_action| {
        delete_action.action().timestamp()
    });

    // ... build FeedMew with deletion info
}
```

### Delete Operations

Deleting entries and cleaning up links:

**`dnas/mewsfeed/zomes/coordinator/mews/src/mew.rs:66-123`:**
```rust
pub fn delete_mew(mew_hash: ActionHash) -> ExternResult<ActionHash> {
    // Remove from global index
    let path = Path::from("all_mews");
    let links = get_links(
        GetLinksInputBuilder::try_new(path.path_entry_hash()?, LinkTypes::AllMews)?.build()
    )?;
    for link in links {
        if link.target == mew_hash.clone().into() {
            delete_link(link.create_link_hash)?;
        }
    }

    // Remove from agent's mews
    let my_agent = agent_info()?.agent_initial_pubkey;
    let agent_links = get_links(
        GetLinksInputBuilder::try_new(my_agent, LinkTypes::AgentMews)?.build()
    )?;
    for link in agent_links {
        if link.target == mew_hash.clone().into() {
            delete_link(link.create_link_hash)?;
        }
    }

    // Delete the entry itself
    delete_entry(mew_hash)
}
```

### Batch Operations

Fetching multiple records efficiently:

```rust
HDK.with(|hdk| {
    hdk.borrow().get(
        links.iter()
            .map(|link| GetInput::new(link.target.clone().into(), GetOptions::default()))
            .collect()
    )
})
```

### Multi-Zome Link Queries

Querying links across multiple zomes in a single call:

**`dnas/mewsfeed/zomes/coordinator/mews/src/agent_to_notifications.rs:30-41`:**
```rust
let links = get_links(
    GetLinksInputBuilder::try_new(agent.clone(), LinkTypeFilter::Types(vec![
        // Mentions from mews zome (ZomeIndex 1, LinkType 6)
        (ZomeIndex(1), vec![LinkType(6)]),
        // Follows from follows zome (ZomeIndex 2, LinkType 1)
        (ZomeIndex(2), vec![LinkType(1)]),
        // Likes from likes zome (ZomeIndex 3, LinkType 1)
        (ZomeIndex(3), vec![LinkType(1)]),
    ]))?
    .build()
)?;
```

---

## 4. Cross-Zome Calls

MewsFeed uses a utility crate to simplify cross-zome communication:

**`crates/hc_call_utils/src/lib.rs:1-39`:**
```rust
pub fn call_local_zome<T, A>(zome_name: &str, fn_name: &str, input: A) -> ExternResult<T>
where
    T: serde::de::DeserializeOwned + std::fmt::Debug,
    A: serde::Serialize + std::fmt::Debug,
{
    match call(
        CallTargetCell::Local,
        zome_name,
        fn_name.into(),
        None,  // No capability required for local calls
        input,
    )? {
        ZomeCallResponse::Ok(result) => {
            Ok(result.decode()?)
        }
        ZomeCallResponse::Unauthorized(_, _, _, _) => {
            Err(wasm_error!("Unauthorized"))
        }
        ZomeCallResponse::NetworkError(msg) => {
            Err(wasm_error!("Network error: {}", msg))
        }
        // ... handle other response types
    }
}
```

### Usage Examples

**Mews zome calling Likes zome:**
```rust
// Get all likers for a mew
let likers: Vec<AgentPubKey> = call_local_zome(
    "likes",
    "get_likers_for_hash",
    mew_hash
)?;
```

**Mews zome calling Agent Pins zome:**
```rust
// Check if current agent has pinned this mew
let is_pinned: bool = call_local_zome(
    "agent_pins",
    "is_hash_pinned",
    hash
)?;
```

**Mews zome calling Follows zome:**
```rust
// Get all creators followed by an agent
let creators: Vec<AgentPubKey> = call_local_zome(
    "follows",
    "get_creators_for_follower",
    ZomeFnInput::new(GetCreatorsForFollowerInput { follower: agent }, Some(true))
)?;
```

**Mews zome calling Profiles zome:**
```rust
// Get profile for notification author
let profile: Option<Record> = call_local_zome(
    "profiles",
    "get_agent_profile",
    ZomeFnInput::new(agent_pub_key, Some(true))
)?;
```

---

## 5. Source Chain Operations

### Agent Activity for Validation

The mewmew deduplication check (shown in Section 1) demonstrates querying an agent's source chain history using `must_get_agent_activity()`.

### Joining Timestamp

**`dnas/mewsfeed/zomes/coordinator/profiles/src/lib.rs:7-27`:**
```rust
pub fn get_joining_timestamp_for_agent(
    input: ZomeFnInput<AgentPubKey>
) -> ExternResult<Option<Timestamp>> {
    let joining_agent_activity: AgentActivity = get_agent_activity(
        input.input,
        ChainQueryFilter::new()
            .action_type(ActionType::AgentValidationPkg)
            .include_entries(true),
        ActivityRequest::Full,
    )?;

    let Some(action) = joining_agent_activity.valid_activity.first() else {
        return Ok(None);
    };

    Ok(Some(action.action().timestamp()))
}
```

This queries the agent's first action (AgentValidationPkg) to determine when they joined the network.

---

## 6. Paths and Prefix Indexing

MewsFeed implements autocomplete/search for hashtags and cashtags using prefix indexing.

### Prefix Index Setup

**`dnas/mewsfeed/zomes/integrity/mews/src/lib.rs:25-27`:**
```rust
pub fn make_tag_prefix_index() -> ExternResult<PrefixIndex> {
    // 3-char prefix paths, 3-char result labels
    PrefixIndex::new("prefix_index".into(), LinkTypes::PrefixIndex, 3, 3)
}
```

### Adding Tags to Index

**`dnas/mewsfeed/zomes/coordinator/mews/src/hashtag_to_mews.rs:12-27`:**
```rust
pub fn add_hashtag_for_mew(input: AddHashtagForMewInput) -> ExternResult<()> {
    // Remove # prefix for indexing
    let tag_text = make_tag_text(input.base_hashtag.clone());
    let prefix_index = make_tag_prefix_index()?;

    // Add to prefix index, get path back
    let path = prefix_index.add_result_with_label(
        tag_text,
        input.base_hashtag.clone()
    )?;

    // Link from path to mew
    create_link(
        path.path_entry_hash()?,
        input.target_mew_hash,
        LinkTypes::HashtagToMews,
        LinkTag(input.base_hashtag.as_bytes().to_vec()),
    )?;

    Ok(())
}
```

### Searching Tags

**`dnas/mewsfeed/zomes/coordinator/mews/src/search_tags.rs:10-14`:**
```rust
fn search_tags(input: ZomeFnInput<SearchTagsInput>) -> ExternResult<Vec<String>> {
    let prefix_index = make_tag_prefix_index()?;
    let strategy = input.get_strategy();

    // Autocomplete search
    prefix_index.get_results(input.input.query, input.input.limit, strategy)
}
```

---

## 7. Pagination Patterns

MewsFeed implements three pagination strategies in `crates/hc_link_pagination/`:

### Timestamp-based Pagination

**`crates/hc_link_pagination/src/timestamp_pagination.rs`:**
```rust
pub struct TimestampPagination {
    pub after_timestamp: Option<Timestamp>,
    pub direction: Option<Direction>,  // Ascending or Descending
    pub limit: usize,
}

pub fn paginate_by_timestamp<T: Timestamped>(
    mut items: Vec<T>,
    page: Option<TimestampPagination>
) -> ExternResult<Vec<T>> {
    let Some(page) = page else { return Ok(items); };

    // Sort by timestamp
    items.sort_by(|a, b| a.timestamp().cmp(&b.timestamp()));

    if let Some(Direction::Descending) = page.direction {
        items.reverse();
    }

    // Filter by cursor
    if let Some(after) = page.after_timestamp {
        items = items.into_iter()
            .skip_while(|item| item.timestamp() <= after)
            .collect();
    }

    // Apply limit
    items.truncate(page.limit);
    Ok(items)
}
```

Used for: Notifications feed

### Hash-based Pagination

**`crates/hc_link_pagination/src/hash_pagination.rs`:**
```rust
pub struct HashPagination {
    pub after_hash: Option<AnyLinkableHash>,
    pub direction: Option<Direction>,
    pub limit: usize,
}

pub fn paginate_by_hash<T: Hashed>(
    mut items: Vec<T>,
    page: Option<HashPagination>
) -> ExternResult<Vec<T>> {
    let Some(page) = page else { return Ok(items); };

    // Find cursor position
    if let Some(after_hash) = page.after_hash {
        let pos = items.iter()
            .position(|item| item.hash() == after_hash);
        if let Some(pos) = pos {
            items = items.split_off(pos + 1);
        }
    }

    items.truncate(page.limit);
    Ok(items)
}
```

Used for: Agent mews, hashtag feeds, mention feeds

### Pagination Traits

```rust
pub trait Timestamped {
    fn timestamp(&self) -> Timestamp;
}

pub trait Hashed {
    fn hash(&self) -> AnyLinkableHash;
}
```

---

## 8. Authorization Patterns

MewsFeed uses author-based authorization rather than Holochain capabilities:

### Author-Only Actions

**Delete validation:**
```rust
if action.author != *original_action.author() {
    return Ok(ValidateCallbackResult::Invalid(
        "Only the author can delete".into()
    ));
}
```

**Link creation validation:**
```rust
if AnyLinkableHash::from(action.author) != base_address {
    return Ok(ValidateCallbackResult::Invalid(
        "Only the mew author can create this link".into()
    ));
}
```

### Self-Service Operations

Like, unlike, pin, unpin automatically use the caller's identity:

```rust
pub fn like(hash: AnyLinkableHash) -> ExternResult<()> {
    add_hash_for_liker(AddHashForLikerInput {
        base_liker: agent_info()?.agent_initial_pubkey,  // Always current agent
        target_hash: hash,
    })
}
```

---

## 9. Notification System

MewsFeed builds notifications by reconstructing events from link history (event sourcing pattern):

**`dnas/mewsfeed/zomes/coordinator/mews/src/agent_to_notifications.rs`:**

```rust
pub fn get_notifications_for_agent(
    input: ZomeFnInput<GetNotificationsForAgentInput>
) -> ExternResult<Vec<Notification>> {
    let agent = input.input.agent.clone();

    // Query links with full details (including deletes)
    let link_details = get_link_details(
        agent.clone(),
        LinkTypeFilter::Types(vec![...]),  // Multiple zome link types
        None,
    )?;

    let mut notifications = Vec::new();

    for LinkDetails { create_actions, delete_actions } in link_details {
        for create in create_actions {
            // Skip if author is self
            if create.action().author() == &agent {
                continue;
            }

            // Create notification from link creation
            notifications.push(Notification {
                notification_type: NotificationType::MyMewLicked,
                timestamp: create.action().timestamp(),
                agent: create.action().author().clone(),
                // ...
            });
        }

        for delete in delete_actions {
            // Create notification from link deletion (unlike)
            notifications.push(Notification {
                notification_type: NotificationType::MyMewUnlicked,
                timestamp: delete.action().timestamp(),
                // ...
            });
        }
    }

    // Paginate by timestamp
    paginate_by_timestamp(notifications, input.input.page)
}
```

### Notification Types

```rust
pub enum NotificationType {
    MyMewLicked,
    MyMewUnlicked,
    MyMewPinned,
    MyMewUnpinned,
    MyMewReplied,
    MyMewQuoted,
    MyMewMewmewed,
    MyAgentMentioned,
    MyAgentFollowed,
}
```

---

## 10. Architecture Summary

### Zome Structure

| Integrity Zome | Coordinator Zome | Purpose |
|---------------|------------------|---------|
| `mews_integrity` | `mews` | Core mew content, tags, mentions, notifications |
| `profiles_integrity` | `profiles` | User profiles, joining timestamps |
| `follows_integrity` | `follows` | Follow relationships |
| `likes_integrity` | `likes` | Licks (likes) on mews |
| `agent_pins_integrity` | `agent_pins` | Pinned mews |
| - | `ping` | Network connectivity check |

### Shared Crates

| Crate | Purpose |
|-------|---------|
| `mews_types` | Core domain types (Mew, FeedMew, Notification) |
| `follows_types` | Follow relationship types |
| `hc_link_pagination` | Pagination utilities |
| `hc_call_utils` | Cross-zome call helpers |
| `hc_zome_input` | Input validation with get strategy |

### Key Holochain Patterns Used

1. **Entry validation** with DNA properties for configurable limits
2. **Source chain scanning** for business rule enforcement (dedup)
3. **Path-based anchors** for global discovery
4. **Bidirectional links** for efficient relationship queries
5. **Link tags** for filtering without entry fetches
6. **Cross-zome calls** for modular architecture
7. **Prefix indexing** for search/autocomplete
8. **Multiple pagination strategies** (timestamp, hash, agent)
9. **Event sourcing from link history** for notifications
10. **Author-based authorization** via validation

### Statistics

- **5 integrity zomes** + **6 coordinator zomes** (the `ping` coordinator has no integrity zome)
- **9+ link types** across zomes
- **3 pagination strategies**
- **9 notification types** reconstructed from link events
- **5 cross-zome call patterns**

---

## Further Reading

- [Holochain Developer Documentation](https://developer.holochain.org/)
- [HDK API Reference](https://docs.rs/hdk/latest/hdk/)
- [Holochain Core Concepts](https://developer.holochain.org/concepts/)
