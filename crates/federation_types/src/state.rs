use hdk::prelude::*;

/// Per-mew federation state stored on the DHT.
///
/// This is *intent/state* that other agents can rely on:
/// - whether the author intended to federate (via `visibility`)
/// - optional mapping to a canonical ActivityPub URI (`ap_uri`)
///
/// Delivery attempts/results are tracked outside the DHT (S2S-local).
#[derive(Serialize, Deserialize, SerializedBytes, Debug, Clone)]
pub struct MewFederationState {
    pub mew_hash: ActionHash,
    /// The ActivityPub URI for this mew, e.g.,
    /// "https://cats.mewsfeed.example/notes/abc123".
    /// None if the mew has not yet been assigned an AP URI.
    pub ap_uri: Option<String>,
    /// Federation intent + audience, intended to be set at creation time.
    pub visibility: FederationVisibility,
}

/// Controls how a mew is federated to the ActivityPub network.
#[derive(Serialize, Deserialize, SerializedBytes, Debug, Clone, PartialEq, Eq)]
pub enum FederationVisibility {
    /// Sent to as:Public (appears in public timelines)
    Public,
    /// CC as:Public (does not appear in public timelines)
    Unlisted,
    /// Sent only to followers collection
    FollowersOnly,
    /// Sent to specific actor URIs only
    Direct(Vec<String>),
    /// Not federated at all; stays on Holochain only
    HolochainOnly,
}

#[cfg(test)]
mod tests {
    use super::*;
    use holochain_serialized_bytes::SerializedBytesError;

    fn test_action_hash() -> ActionHash {
        ActionHash::from_raw_36(vec![0xdb; 36])
    }

    #[test]
    fn mew_federation_state_roundtrip() -> Result<(), SerializedBytesError> {
        let state = MewFederationState {
            mew_hash: test_action_hash(),
            ap_uri: Some("https://cats.mewsfeed.example/notes/abc123".to_string()),
            visibility: FederationVisibility::Public,
        };
        let bytes: SerializedBytes = state.clone().try_into()?;
        let restored: MewFederationState = bytes.try_into()?;
        assert_eq!(restored.mew_hash, state.mew_hash);
        assert_eq!(restored.ap_uri, state.ap_uri);
        assert_eq!(restored.visibility, state.visibility);
        Ok(())
    }

    #[test]
    fn mew_federation_state_no_uri_roundtrip() -> Result<(), SerializedBytesError> {
        let state = MewFederationState {
            mew_hash: test_action_hash(),
            ap_uri: None,
            visibility: FederationVisibility::HolochainOnly,
        };
        let bytes: SerializedBytes = state.clone().try_into()?;
        let restored: MewFederationState = bytes.try_into()?;
        assert_eq!(restored.ap_uri, None);
        assert_eq!(restored.visibility, FederationVisibility::HolochainOnly);
        Ok(())
    }

    #[test]
    fn federation_visibility_all_variants_roundtrip() -> Result<(), SerializedBytesError> {
        let variants = vec![
            FederationVisibility::Public,
            FederationVisibility::Unlisted,
            FederationVisibility::FollowersOnly,
            FederationVisibility::Direct(vec![
                "https://mastodon.social/users/alice".to_string(),
                "https://pleroma.example/users/bob".to_string(),
            ]),
            FederationVisibility::HolochainOnly,
        ];
        for variant in variants {
            let bytes: SerializedBytes = variant.clone().try_into()?;
            let restored: FederationVisibility = bytes.try_into()?;
            assert_eq!(restored, variant);
        }
        Ok(())
    }

    #[test]
    fn federation_visibility_direct_empty_vec() -> Result<(), SerializedBytesError> {
        let vis = FederationVisibility::Direct(vec![]);
        let bytes: SerializedBytes = vis.clone().try_into()?;
        let restored: FederationVisibility = bytes.try_into()?;
        assert_eq!(restored, FederationVisibility::Direct(vec![]));
        Ok(())
    }
}
