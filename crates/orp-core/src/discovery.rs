use serde::{Deserialize, Serialize};

use crate::limits::LimitsPolicy;
use crate::sign::PublicKeyBundle;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryDocument {
    pub v: String,
    pub endpoint: String,
    pub public_keys: Vec<PublicKeyBundle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy_url: Option<String>,
    #[serde(default)]
    pub limits: LimitsPolicy,
}

impl DiscoveryDocument {
    pub fn new(endpoint: impl Into<String>, public_keys: Vec<PublicKeyBundle>) -> Self {
        let endpoint = endpoint.into();
        let policy_url = format!("{}/v1/policy/{{email}}", endpoint.trim_end_matches('/'));
        Self {
            v: "0.2".into(),
            endpoint,
            public_keys,
            policy_url: Some(policy_url),
            limits: LimitsPolicy::default(),
        }
    }
}

/// Extract domain from email address.
pub fn domain_from_email(email: &str) -> Option<String> {
    email.rsplit('@').next().map(|d| d.to_lowercase())
}
