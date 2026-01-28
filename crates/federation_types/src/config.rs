use hdk::prelude::*;

/// Configuration for the collective fediverse instance.
/// Derived from an invite URL during hApp installation.
/// All nodes in the Holochain network share responsibility for serving this
/// instance.
#[derive(Serialize, Deserialize, SerializedBytes, Debug, Clone)]
pub struct InstanceConfig {
    /// The subdomain for this instance, e.g., "cats"
    pub subdomain: String,
    /// The gateway WebSocket URL, e.g., "wss://gateway.mewsfeed.example"
    pub gateway_url: String,
    /// The full instance URI, e.g., "https://cats.mewsfeed.example"
    pub instance_uri: String,
    /// Key ID for HTTP Signatures, e.g.,
    /// "https://cats.mewsfeed.example/actor#main-key"
    pub public_key_id: String,
}

/// Parsed components from an invite URL.
/// Used during bootstrapping to configure the S2S module.
///
/// Invite URL format:
/// `mewsfeed://join?gateway=wss://gateway.mewsfeed.example&subdomain=cats&key=<material>`
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InviteUrlParams {
    /// The gateway WebSocket URL
    pub gateway: String,
    /// The subdomain for the instance
    pub subdomain: String,
    /// Encrypted keypair or derivation seed material
    pub key_material: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use holochain_serialized_bytes::SerializedBytesError;

    #[test]
    fn instance_config_roundtrip() -> Result<(), SerializedBytesError> {
        let config = InstanceConfig {
            subdomain: "cats".to_string(),
            gateway_url: "wss://gateway.mewsfeed.example".to_string(),
            instance_uri: "https://cats.mewsfeed.example".to_string(),
            public_key_id: "https://cats.mewsfeed.example/actor#main-key".to_string(),
        };
        let bytes: SerializedBytes = config.clone().try_into()?;
        let restored: InstanceConfig = bytes.try_into()?;
        assert_eq!(restored.subdomain, config.subdomain);
        assert_eq!(restored.gateway_url, config.gateway_url);
        assert_eq!(restored.instance_uri, config.instance_uri);
        assert_eq!(restored.public_key_id, config.public_key_id);
        Ok(())
    }

    #[test]
    fn invite_url_params_json_roundtrip() {
        let params = InviteUrlParams {
            gateway: "wss://gateway.mewsfeed.example".to_string(),
            subdomain: "cats".to_string(),
            key_material: "base64-encoded-key-material".to_string(),
        };
        let json = serde_json::to_string(&params).expect("serialize");
        let restored: InviteUrlParams = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored.gateway, params.gateway);
        assert_eq!(restored.subdomain, params.subdomain);
        assert_eq!(restored.key_material, params.key_material);
    }
}
