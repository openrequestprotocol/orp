use chrono::{DateTime, Utc};
use orp_bridge::degrade::degrade_to_email;
use orp_core::{verify_request, DeliveryReceipt, OrpError, Request, Transport};
use reqwest::Client;
use serde_json::json;
use tracing::{info, warn};

use crate::discovery::{fetch_discovery, resolve_endpoint};
use crate::state::AppState;

/// Deliver a request: native S2S if recipient speaks ORP, else degrade to email bridge.
pub async fn deliver_outbound(state: &AppState, req: &Request) -> Result<DeliveryReceipt, OrpError> {
    crate::webhook::verify_sender_signature(state, req).await?;

    let recipient = req.to_addr();

    if is_registered_recipient(state, recipient).await? {
        info!(recipient, "local delivery (registered recipient)");
        return ingest_inbound(state, req, Transport::Native, 1.0).await;
    }

    if let Some(endpoint) = resolve_endpoint(recipient).await? {
        info!(recipient, endpoint, "native ORP delivery");
        return deliver_native(state, req, &endpoint).await;
    }

    if recipient
        .to_lowercase()
        .ends_with(&format!("@{}", state.config.domain.to_lowercase()))
    {
        info!(recipient, "local delivery");
        return ingest_inbound(state, req, Transport::Native, 1.0).await;
    }

    warn!(recipient, "no ORP endpoint; degrading to email bridge");
    let email = degrade_to_email(req, None)?;

    // If an SMTP transport is configured, actually deliver the degraded mail.
    if let Some(smtp_url) = state.config.smtp_url.as_deref() {
        match crate::smtp::send_raw_email(smtp_url, req.from_addr(), recipient, email.as_bytes())
            .await
        {
            Ok(()) => {
                info!(recipient, "bridged email delivered via SMTP");
                return Ok(DeliveryReceipt::accepted(req.id()));
            }
            Err(e) => {
                warn!(recipient, error = %e, "SMTP bridge send failed; queueing for retry");
                enqueue_bridge(state, req, recipient, &e.to_string()).await?;
                return Ok(DeliveryReceipt::queued(req.id()));
            }
        }
    }

    // No SMTP transport configured: queue (unsent) and be honest about it.
    enqueue_bridge(state, req, recipient, "email bridge: SMTP transport not configured").await?;
    Ok(DeliveryReceipt::queued(req.id()))
}

/// Persist a degraded request to the delivery queue for later (manual or
/// out-of-band) SMTP delivery. Uses a `mailto:` pseudo-endpoint so the row is
/// distinguishable from native S2S retries.
async fn enqueue_bridge(
    state: &AppState,
    req: &Request,
    recipient: &str,
    reason: &str,
) -> Result<(), OrpError> {
    let queue_id = format!("dq_{}", uuid::Uuid::new_v4());
    sqlx::query(
        r#"INSERT INTO orp_delivery_queue (id, request_json, target_endpoint, last_error)
           VALUES ($1, $2, $3, $4)"#,
    )
    .bind(&queue_id)
    .bind(json!(req))
    .bind(format!("mailto:{recipient}"))
    .bind(reason)
    .execute(&state.pool)
    .await
    .map_err(|e| OrpError::Serialization(e.to_string()))?;
    Ok(())
}

#[derive(Debug)]
pub enum DeliveryResult {
    Native { endpoint: String },
    Local,
    EmailBridge { raw_email: String },
    Queued { queue_id: String },
}

async fn deliver_native(
    state: &AppState,
    req: &Request,
    endpoint: &str,
) -> Result<DeliveryReceipt, OrpError> {
    let discovery = fetch_discovery(endpoint).await?;
    verify_request(req, &discovery.public_keys)?;

    let client = Client::new();
    let url = format!("{}/v1/deliver", endpoint.trim_end_matches('/'));
    let resp = client
        .post(&url)
        .json(req)
        .send()
        .await
        .map_err(|e| OrpError::Serialization(e.to_string()))?;

    if resp.status().is_success() {
        if let Ok(receipt) = resp.json::<DeliveryReceipt>().await {
            return Ok(receipt);
        }
        return Ok(DeliveryReceipt::accepted(req.id()));
    }

    let queue_id = format!("dq_{}", uuid::Uuid::new_v4());
    sqlx::query(
        r#"INSERT INTO orp_delivery_queue (id, request_json, target_endpoint, last_error)
           VALUES ($1, $2, $3, $4)"#,
    )
    .bind(&queue_id)
    .bind(json!(req))
    .bind(endpoint)
    .bind(resp.text().await.unwrap_or_default())
    .execute(&state.pool)
    .await
    .map_err(|e| OrpError::Serialization(e.to_string()))?;

    Ok(DeliveryReceipt::queued(req.id()))
}

/// Ingest inbound request with policy enforcement.
pub async fn ingest_inbound(
    state: &AppState,
    req: &Request,
    transport: Transport,
    confidence: f64,
) -> Result<DeliveryReceipt, OrpError> {
    let keys = crate::keys::resolve_verify_keys(
        &state.pool,
        &state.server_public_keys(),
        req.from_addr(),
        &req.sig.key_id,
    )
    .await?;
    verify_request(req, &keys)?;

    let recipient = req.to_addr();
    let sender = req.from_addr();

    let policy = load_policy(state, recipient).await?;

    let is_known = is_known_sender(state, recipient, sender).await?;

    let check = orp_core::validate_against_policy(req, &policy, is_known)?;
    if let orp_core::PolicyCheckResult::Reject(reason) = check {
        return Err(OrpError::PolicyViolation(reason));
    }

    enforce_budget(state, recipient, sender, &policy, req.body.importance, is_known).await?;

    let effective_importance = {
        let mut reps = state.reputation.write().await;
        let store = reps
            .entry(recipient.to_string())
            .or_insert_with(orp_core::ReputationStore::new);
        store.adjust_importance(sender, req.body.importance)
    };

    let mut stored = req.clone();
    stored.body.transport = Some(transport);
    stored.body.importance = effective_importance;

    let insert = sqlx::query(
        r#"INSERT INTO orp_requests (id, recipient, sender, request_json, importance, intent, transport, confidence)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
           ON CONFLICT (id) DO NOTHING"#,
    )
    .bind(stored.id())
    .bind(recipient)
    .bind(sender)
    .bind(json!(stored))
    .bind(stored.body.importance.as_str())
    .bind(stored.body.intent.as_str())
    .bind(match transport {
        Transport::Native => "native",
        Transport::EmailBridge => "email_bridge",
        Transport::Inferred => "inferred",
    })
    .bind(confidence)
    .execute(&state.pool)
    .await
    .map_err(|e| OrpError::Serialization(e.to_string()))?;

    if insert.rows_affected() == 0 {
        let existing: (DateTime<Utc>,) =
            sqlx::query_as("SELECT created_at FROM orp_requests WHERE id = $1")
                .bind(stored.id())
                .fetch_one(&state.pool)
                .await
                .map_err(|e| OrpError::Serialization(e.to_string()))?;
        return Ok(DeliveryReceipt::duplicate(stored.id(), existing.0));
    }

    sqlx::query(
        r#"INSERT INTO orp_known_senders (recipient, sender) VALUES ($1, $2)
           ON CONFLICT DO NOTHING"#,
    )
    .bind(recipient)
    .bind(sender)
    .execute(&state.pool)
    .await
    .map_err(|e| OrpError::Serialization(e.to_string()))?;

    crate::webhook::notify_ingest(state, recipient, stored.id()).await;

    Ok(DeliveryReceipt::accepted(stored.id()))
}

pub async fn load_policy(state: &AppState, email: &str) -> Result<orp_core::Policy, OrpError> {
    let row: Option<(serde_json::Value,)> =
        sqlx::query_as("SELECT policy_json FROM orp_users WHERE email = $1")
            .bind(email)
            .fetch_optional(&state.pool)
            .await
            .map_err(|e| OrpError::Serialization(e.to_string()))?;

    if let Some((json,)) = row {
        serde_json::from_value(json).map_err(|e| OrpError::InvalidPolicy(e.to_string()))
    } else {
        let policy = orp_core::Policy::default_for(email);
        sqlx::query(
            r#"INSERT INTO orp_users (email, policy_json) VALUES ($1, $2)
               ON CONFLICT (email) DO NOTHING"#,
        )
        .bind(email)
        .bind(json!(policy))
        .execute(&state.pool)
        .await
        .map_err(|e| OrpError::Serialization(e.to_string()))?;
        Ok(policy)
    }
}

/// Enforce per-sender budgets against the persistent `orp_budget_state` row.
///
/// Loads (or creates) the sender's budget, rolls any elapsed windows, checks
/// the limits, and — only if the request is admitted — consumes and persists
/// the updated counts. The whole check-then-consume runs in a transaction with
/// `FOR UPDATE` so concurrent deliveries cannot both slip past the same cap.
async fn enforce_budget(
    state: &AppState,
    recipient: &str,
    sender: &str,
    policy: &orp_core::Policy,
    importance: orp_core::Importance,
    is_known: bool,
) -> Result<(), OrpError> {
    let now = Utc::now();

    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|e| OrpError::Serialization(e.to_string()))?;

    let row: Option<(i32, DateTime<Utc>, i32, DateTime<Utc>)> = sqlx::query_as(
        r#"SELECT high_used, window_start, unknown_today, unknown_window_start
           FROM orp_budget_state
           WHERE recipient = $1 AND sender = $2
           FOR UPDATE"#,
    )
    .bind(recipient)
    .bind(sender)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| OrpError::Serialization(e.to_string()))?;

    let mut budget = match row {
        Some((high_used, high_window_start, unknown_used, unknown_window_start)) => {
            orp_core::SenderBudget {
                high_used: high_used.max(0) as u32,
                high_window_start,
                unknown_used: unknown_used.max(0) as u32,
                unknown_window_start,
            }
        }
        None => orp_core::SenderBudget::fresh(now),
    };

    budget.roll(now);

    let enforce_unknown = !is_known && !policy.is_vip(sender);
    budget.check_high(importance, orp_core::high_limit(policy, sender))?;
    if enforce_unknown {
        budget.check_unknown(policy.rate_limits.unknown_per_day)?;
    }

    budget.consume_high(importance);
    if enforce_unknown {
        budget.consume_unknown();
    }

    sqlx::query(
        r#"INSERT INTO orp_budget_state
               (recipient, sender, high_used, window_start, unknown_today, unknown_window_start)
           VALUES ($1, $2, $3, $4, $5, $6)
           ON CONFLICT (recipient, sender) DO UPDATE SET
               high_used = excluded.high_used,
               window_start = excluded.window_start,
               unknown_today = excluded.unknown_today,
               unknown_window_start = excluded.unknown_window_start"#,
    )
    .bind(recipient)
    .bind(sender)
    .bind(budget.high_used as i32)
    .bind(budget.high_window_start)
    .bind(budget.unknown_used as i32)
    .bind(budget.unknown_window_start)
    .execute(&mut *tx)
    .await
    .map_err(|e| OrpError::Serialization(e.to_string()))?;

    tx.commit()
        .await
        .map_err(|e| OrpError::Serialization(e.to_string()))?;

    Ok(())
}

async fn is_known_sender(state: &AppState, recipient: &str, sender: &str) -> Result<bool, OrpError> {
    let row: Option<(i64,)> = sqlx::query_as(
        "SELECT 1 FROM orp_known_senders WHERE recipient = $1 AND sender = $2",
    )
    .bind(recipient)
    .bind(sender)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| OrpError::Serialization(e.to_string()))?;
    Ok(row.is_some())
}

pub async fn is_registered_recipient(state: &AppState, email: &str) -> Result<bool, OrpError> {
    let row: Option<(i64,)> =
        sqlx::query_as("SELECT 1 FROM orp_users WHERE email = $1")
            .bind(email)
            .fetch_optional(&state.pool)
            .await
            .map_err(|e| OrpError::Serialization(e.to_string()))?;
    Ok(row.is_some())
}

pub async fn save_policy(state: &AppState, email: &str, policy: &orp_core::Policy) -> Result<(), OrpError> {
    sqlx::query(
        r#"INSERT INTO orp_users (email, policy_json) VALUES ($1, $2)
           ON CONFLICT (email) DO UPDATE SET policy_json = excluded.policy_json"#,
    )
    .bind(email)
    .bind(json!(policy))
    .execute(&state.pool)
    .await
    .map_err(|e| OrpError::Serialization(e.to_string()))?;
    Ok(())
}
