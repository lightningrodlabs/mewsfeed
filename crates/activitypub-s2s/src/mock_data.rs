use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use rsa::{RsaPrivateKey, RsaPublicKey};

use crate::actor::{APActor, APPublicKey};
use crate::collections::OrderedCollection;
use crate::crypto;
use crate::error::S2SError;
use crate::webfinger::{WebFingerLink, WebFingerResponse};
use crate::{APActivity, AS_CONTEXT, SECURITY_CONTEXT};

/// Trait abstracting the data layer. Phase 2 uses `MockDataSource`;
/// Phase 3 will use `ZomeDataSource` backed by AppWebsocket calls.
#[async_trait]
pub trait FederationDataSource: Send + Sync {
    async fn get_actor(&self, username: &str) -> Result<Option<APActor>, S2SError>;
    async fn get_outbox(&self, username: &str) -> Result<Option<OrderedCollection>, S2SError>;
    async fn get_webfinger(
        &self,
        username: &str,
        domain: &str,
    ) -> Result<Option<WebFingerResponse>, S2SError>;
    async fn process_inbox(
        &self,
        username: &str,
        activity: APActivity,
    ) -> Result<(), S2SError>;
    async fn get_signing_key(
        &self,
        username: &str,
    ) -> Result<Option<RsaPrivateKey>, S2SError>;
    async fn get_public_key_pem(&self, username: &str) -> Result<Option<String>, S2SError>;
    /// Debug: return the list of activities received by a user's inbox.
    async fn get_received_activities(
        &self,
        username: &str,
    ) -> Result<Vec<APActivity>, S2SError>;
    /// Debug: return a user's private key as PEM.
    async fn get_private_key_pem(&self, username: &str) -> Result<Option<String>, S2SError>;
}

struct MockUser {
    username: String,
    display_name: String,
    summary: String,
    private_key: RsaPrivateKey,
    public_key: RsaPublicKey,
    public_key_pem: String,
    received: Arc<Mutex<Vec<APActivity>>>,
}

/// In-memory data source with hardcoded users for Phase 2 testing.
pub struct MockDataSource {
    domain: String,
    users: HashMap<String, MockUser>,
}

impl MockDataSource {
    pub fn new(domain: &str) -> Result<Self, S2SError> {
        let mut users = HashMap::new();
        for (username, display, summary) in [
            ("alice", "Alice", "Holochain enthusiast"),
            ("bob", "Bob", "Federation tester"),
        ] {
            let (private_key, public_key) = crypto::generate_rsa_keypair()?;
            let public_key_pem = crypto::public_key_to_pem(&public_key)?;
            users.insert(
                username.to_string(),
                MockUser {
                    username: username.to_string(),
                    display_name: display.to_string(),
                    summary: summary.to_string(),
                    private_key,
                    public_key,
                    public_key_pem,
                    received: Arc::new(Mutex::new(Vec::new())),
                },
            );
        }
        Ok(MockDataSource {
            domain: domain.to_string(),
            users,
        })
    }

    fn base_url(&self) -> String {
        format!("http://{}", self.domain)
    }

    fn build_actor(&self, user: &MockUser) -> APActor {
        let base = self.base_url();
        let actor_url = format!("{}/users/{}", base, user.username);
        APActor {
            context: serde_json::json!([AS_CONTEXT, SECURITY_CONTEXT]),
            id: actor_url.clone(),
            actor_type: "Person".to_string(),
            preferred_username: user.username.clone(),
            name: Some(user.display_name.clone()),
            summary: Some(format!("<p>{}</p>", user.summary)),
            icon: None,
            url: Some(actor_url.clone()),
            inbox: format!("{}/inbox", actor_url),
            outbox: format!("{}/outbox", actor_url),
            followers: Some(format!("{}/followers", actor_url)),
            following: Some(format!("{}/following", actor_url)),
            public_key: APPublicKey {
                id: format!("{}#main-key", actor_url),
                owner: actor_url,
                public_key_pem: user.public_key_pem.clone(),
            },
            manually_approves_followers: false,
            discoverable: true,
        }
    }
}

#[async_trait]
impl FederationDataSource for MockDataSource {
    async fn get_actor(&self, username: &str) -> Result<Option<APActor>, S2SError> {
        Ok(self.users.get(username).map(|u| self.build_actor(u)))
    }

    async fn get_outbox(&self, username: &str) -> Result<Option<OrderedCollection>, S2SError> {
        if !self.users.contains_key(username) {
            return Ok(None);
        }
        let base = self.base_url();
        Ok(Some(OrderedCollection {
            context: serde_json::json!(AS_CONTEXT),
            id: format!("{}/users/{}/outbox", base, username),
            collection_type: "OrderedCollection".to_string(),
            total_items: 0,
            first: None,
            last: None,
        }))
    }

    async fn get_webfinger(
        &self,
        username: &str,
        domain: &str,
    ) -> Result<Option<WebFingerResponse>, S2SError> {
        if !self.users.contains_key(username) {
            return Ok(None);
        }
        let base = self.base_url();
        let actor_url = format!("{}/users/{}", base, username);
        Ok(Some(WebFingerResponse {
            subject: format!("acct:{}@{}", username, domain),
            aliases: vec![actor_url.clone()],
            links: vec![
                WebFingerLink {
                    rel: "self".to_string(),
                    link_type: Some("application/activity+json".to_string()),
                    href: Some(actor_url),
                    template: None,
                    properties: Default::default(),
                },
                WebFingerLink {
                    rel: "http://webfinger.net/rel/profile-page".to_string(),
                    link_type: Some("text/html".to_string()),
                    href: Some(format!("{}/users/{}", base, username)),
                    template: None,
                    properties: Default::default(),
                },
            ],
        }))
    }

    async fn process_inbox(
        &self,
        username: &str,
        activity: APActivity,
    ) -> Result<(), S2SError> {
        let user = self
            .users
            .get(username)
            .ok_or_else(|| S2SError::NotFound(format!("user: {username}")))?;
        let mut received = user.received.lock().unwrap();
        tracing::info!(
            username,
            activity_type = activity.activity_type,
            "inbox received activity"
        );
        received.push(activity);
        Ok(())
    }

    async fn get_signing_key(
        &self,
        username: &str,
    ) -> Result<Option<RsaPrivateKey>, S2SError> {
        Ok(self.users.get(username).map(|u| u.private_key.clone()))
    }

    async fn get_public_key_pem(&self, username: &str) -> Result<Option<String>, S2SError> {
        Ok(self.users.get(username).map(|u| u.public_key_pem.clone()))
    }

    async fn get_received_activities(
        &self,
        username: &str,
    ) -> Result<Vec<APActivity>, S2SError> {
        let user = self
            .users
            .get(username)
            .ok_or_else(|| S2SError::NotFound(format!("user: {username}")))?;
        let received = user.received.lock().unwrap();
        Ok(received.clone())
    }

    async fn get_private_key_pem(&self, username: &str) -> Result<Option<String>, S2SError> {
        match self.users.get(username) {
            Some(u) => Ok(Some(crypto::private_key_to_pem(&u.private_key)?)),
            None => Ok(None),
        }
    }
}
