use std::time::Duration;

use orp_core::{domain_from_email, DiscoveryDocument, OrpError};
use reqwest::Client;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct WellKnownOrp {
    v: String,
    endpoint: String,
    #[serde(default)]
    #[allow(dead_code)]
    policy_url: Option<String>,
}

/// Resolve recipient's ORP endpoint via HTTPS .well-known/orp
pub async fn resolve_endpoint(email: &str) -> Result<Option<String>, OrpError> {
    let domain = domain_from_email(email)
        .ok_or_else(|| OrpError::InvalidRequest("invalid email".into()))?;

    if domain == "localhost" || domain.ends_with(".local") {
        return Ok(None);
    }

    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| OrpError::Serialization(e.to_string()))?;

    let url = format!("https://{domain}/.well-known/orp");
    let resp = match client.get(&url).send().await {
        Ok(r) if r.status().is_success() => r,
        _ => return Ok(None),
    };

    let doc: WellKnownOrp = resp
        .json()
        .await
        .map_err(|e| OrpError::Serialization(e.to_string()))?;

    if doc.v != "0.1" && doc.v != "0.2" {
        return Ok(None);
    }

    Ok(Some(doc.endpoint))
}

/// Fetch discovery document from a remote endpoint.
pub async fn fetch_discovery(endpoint: &str) -> Result<DiscoveryDocument, OrpError> {
    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| OrpError::Serialization(e.to_string()))?;
    let url = format!("{}/.well-known/orp", endpoint.trim_end_matches('/'));
    let doc: DiscoveryDocument = client
        .get(&url)
        .send()
        .await
        .map_err(|e| OrpError::Serialization(e.to_string()))?
        .json()
        .await
        .map_err(|e| OrpError::Serialization(e.to_string()))?;
    Ok(doc)
}

/// Fetch recipient policy from remote server.
pub async fn fetch_policy(endpoint: &str, email: &str) -> Result<orp_core::Policy, OrpError> {
    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| OrpError::Serialization(e.to_string()))?;
    let url = format!(
        "{}/v1/policy/{}",
        endpoint.trim_end_matches('/'),
        urlencoding_encode(email)
    );
    let policy: orp_core::Policy = client
        .get(&url)
        .send()
        .await
        .map_err(|e| OrpError::Serialization(e.to_string()))?
        .json()
        .await
        .map_err(|e| OrpError::Serialization(e.to_string()))?;
    Ok(policy)
}

fn urlencoding_encode(s: &str) -> String {
    s.replace('@', "%40")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domain_extraction() {
        assert_eq!(
            domain_from_email("bob@example.com"),
            Some("example.com".into())
        );
    }
}
