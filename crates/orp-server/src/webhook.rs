use orp_core::OrpError;
use reqwest::Client;
use serde_json::json;
use tracing::warn;

use crate::state::AppState;

pub async fn notify_ingest(state: &AppState, recipient: &str, request_id: &str) {
    let Some(url) = state.config.webhook_url.as_ref() else {
        return;
    };
    let client = Client::new();
    let mut req = client.post(url).json(&json!({
        "recipient": recipient,
        "request_id": request_id,
        "event": "request.ingested",
    }));
    if let Some(secret) = &state.config.shared_secret {
        req = req.header("X-ORP-Secret", secret);
    }
    if let Err(e) = req.send().await {
        warn!(error = %e, recipient, request_id, "orp webhook notify failed");
    }
}

pub async fn verify_sender_signature(
    state: &AppState,
    req: &orp_core::Request,
) -> Result<(), OrpError> {
    let keys = crate::keys::resolve_verify_keys(
        &state.pool,
        &state.server_public_keys(),
        req.from_addr(),
        &req.sig.key_id,
    )
    .await?;
    orp_core::verify_request(req, &keys)
}
