use hdk::prelude::*;

/// Lightweight reference to a remote fediverse actor.
/// Full actor data (avatar, bio, public keys, etc.) is cached in the S2S module,
/// not on the DHT. This struct holds only the minimal fields needed for DHT
/// consensus.
#[derive(Serialize, Deserialize, SerializedBytes, Debug, Clone)]
pub struct RemoteActorRef {
    /// The actor's canonical URI, e.g., "https://mastodon.social/users/alice"
    pub actor_uri: String,
    /// The actor's handle, e.g., "@alice@mastodon.social"
    pub handle: String,
    /// Display name, if known
    pub display_name: Option<String>,
}

/// A local Holochain agent following a remote fediverse actor.
/// Stored on the DHT so all nodes know about cross-network follow
/// relationships.
#[derive(Serialize, Deserialize, SerializedBytes, Debug, Clone)]
pub struct RemoteFollow {
    pub local_agent: AgentPubKey,
    /// The remote actor's canonical URI
    pub remote_actor_uri: String,
    pub status: RemoteFollowStatus,
    pub created_at: Timestamp,
}

/// Status of a pending remote follow request.
#[derive(Serialize, Deserialize, SerializedBytes, Debug, Clone, PartialEq, Eq)]
pub enum RemoteFollowStatus {
    /// Follow activity sent, awaiting Accept from remote actor
    Pending,
    /// Remote actor accepted the follow
    Accepted,
    /// Remote actor rejected the follow
    Rejected,
}

/// A remote fediverse actor following a local Holochain agent.
#[derive(Serialize, Deserialize, SerializedBytes, Debug, Clone)]
pub struct RemoteFollower {
    pub local_agent: AgentPubKey,
    /// The remote actor's canonical URI
    pub remote_actor_uri: String,
    pub followed_at: Timestamp,
}

/// A remote interaction (like, boost, reply) on a local mew from the fediverse.
#[derive(Serialize, Deserialize, SerializedBytes, Debug, Clone)]
pub struct RemoteInteraction {
    pub mew_hash: ActionHash,
    /// The remote actor's canonical URI
    pub actor_uri: String,
    pub interaction_type: RemoteInteractionType,
    /// The AP activity URI, needed for Undo operations
    pub activity_uri: String,
    pub timestamp: Timestamp,
}

/// The type of remote interaction from the fediverse.
#[derive(Serialize, Deserialize, SerializedBytes, Debug, Clone, PartialEq, Eq)]
pub enum RemoteInteractionType {
    Like,
    /// Boost/reblog (ActivityPub Announce activity)
    Announce,
    Reply {
        /// The AP Note URI of the reply
        note_uri: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use holochain_serialized_bytes::SerializedBytesError;

    fn test_action_hash() -> ActionHash {
        ActionHash::from_raw_36(vec![0xdb; 36])
    }

    fn test_agent_pub_key() -> AgentPubKey {
        AgentPubKey::from_raw_36(vec![0x84; 36])
    }

    fn test_timestamp() -> Timestamp {
        Timestamp::from_micros(1_700_000_000_000_000)
    }

    #[test]
    fn remote_actor_ref_roundtrip() -> Result<(), SerializedBytesError> {
        let actor = RemoteActorRef {
            actor_uri: "https://mastodon.social/users/alice".to_string(),
            handle: "@alice@mastodon.social".to_string(),
            display_name: Some("Alice".to_string()),
        };
        let bytes: SerializedBytes = actor.clone().try_into()?;
        let restored: RemoteActorRef = bytes.try_into()?;
        assert_eq!(restored.actor_uri, actor.actor_uri);
        assert_eq!(restored.handle, actor.handle);
        assert_eq!(restored.display_name, actor.display_name);
        Ok(())
    }

    #[test]
    fn remote_actor_ref_no_display_name() -> Result<(), SerializedBytesError> {
        let actor = RemoteActorRef {
            actor_uri: "https://pleroma.example/users/bob".to_string(),
            handle: "@bob@pleroma.example".to_string(),
            display_name: None,
        };
        let bytes: SerializedBytes = actor.clone().try_into()?;
        let restored: RemoteActorRef = bytes.try_into()?;
        assert_eq!(restored.display_name, None);
        Ok(())
    }

    #[test]
    fn remote_follow_roundtrip() -> Result<(), SerializedBytesError> {
        let follow = RemoteFollow {
            local_agent: test_agent_pub_key(),
            remote_actor_uri: "https://mastodon.social/users/alice".to_string(),
            status: RemoteFollowStatus::Accepted,
            created_at: test_timestamp(),
        };
        let bytes: SerializedBytes = follow.clone().try_into()?;
        let restored: RemoteFollow = bytes.try_into()?;
        assert_eq!(restored.remote_actor_uri, follow.remote_actor_uri);
        assert_eq!(restored.status, follow.status);
        Ok(())
    }

    #[test]
    fn remote_follow_status_all_variants_roundtrip() -> Result<(), SerializedBytesError> {
        let variants = vec![
            RemoteFollowStatus::Pending,
            RemoteFollowStatus::Accepted,
            RemoteFollowStatus::Rejected,
        ];
        for variant in variants {
            let bytes: SerializedBytes = variant.clone().try_into()?;
            let restored: RemoteFollowStatus = bytes.try_into()?;
            assert_eq!(restored, variant);
        }
        Ok(())
    }

    #[test]
    fn remote_follower_roundtrip() -> Result<(), SerializedBytesError> {
        let follower = RemoteFollower {
            local_agent: test_agent_pub_key(),
            remote_actor_uri: "https://mastodon.social/users/carol".to_string(),
            followed_at: test_timestamp(),
        };
        let bytes: SerializedBytes = follower.clone().try_into()?;
        let restored: RemoteFollower = bytes.try_into()?;
        assert_eq!(restored.remote_actor_uri, follower.remote_actor_uri);
        Ok(())
    }

    #[test]
    fn remote_interaction_roundtrip() -> Result<(), SerializedBytesError> {
        let interaction = RemoteInteraction {
            mew_hash: test_action_hash(),
            actor_uri: "https://mastodon.social/users/dave".to_string(),
            interaction_type: RemoteInteractionType::Like,
            activity_uri: "https://mastodon.social/activities/123".to_string(),
            timestamp: test_timestamp(),
        };
        let bytes: SerializedBytes = interaction.clone().try_into()?;
        let restored: RemoteInteraction = bytes.try_into()?;
        assert_eq!(restored.actor_uri, interaction.actor_uri);
        assert_eq!(restored.interaction_type, interaction.interaction_type);
        assert_eq!(restored.activity_uri, interaction.activity_uri);
        Ok(())
    }

    #[test]
    fn remote_interaction_type_all_variants_roundtrip() -> Result<(), SerializedBytesError> {
        let variants = vec![
            RemoteInteractionType::Like,
            RemoteInteractionType::Announce,
            RemoteInteractionType::Reply {
                note_uri: "https://mastodon.social/notes/456".to_string(),
            },
        ];
        for variant in variants {
            let bytes: SerializedBytes = variant.clone().try_into()?;
            let restored: RemoteInteractionType = bytes.try_into()?;
            assert_eq!(restored, variant);
        }
        Ok(())
    }
}
