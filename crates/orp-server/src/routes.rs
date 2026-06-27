use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use orp_bridge::inference::{wrap_legacy_email, HeuristicInference};
use orp_core::{DeliveryReceipt, FeedbackAction, Policy, PublicKeyBundle, Request, ReputationStore, Transport};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::auth::check_secret;
use crate::deliver::{deliver_outbound, ingest_inbound, load_policy, save_policy};
use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/.well-known/orp", get(well_known))
        .route("/v1/policy/{email}", get(get_policy).put(put_policy))
        .route("/v1/deliver", post(deliver))
        .route("/v1/request", post(ingest_request))
        .route("/v1/requests", get(list_requests))
        .route("/v1/requests/{id}/feedback", post(submit_feedback))
        .route("/v1/bridge/email", post(bridge_email))
        .route("/v1/keys", post(register_key))
        .route("/health", get(|| async { "ok" }))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn well_known(State(state): State<AppState>) -> Result<Json<orp_core::DiscoveryDocument>, ApiError> {
    let keys = crate::keys::discovery_public_keys(&state.pool, &state.server_public_keys())
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let mut doc = (*state.discovery).clone();
    doc.public_keys = keys;
    Ok(Json(doc))
}

async fn get_policy(
    State(state): State<AppState>,
    Path(email): Path<String>,
) -> Result<Json<Policy>, ApiError> {
    let policy = load_policy(&state, &email).await?;
    Ok(Json(policy))
}

async fn put_policy(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(email): Path<String>,
    Json(policy): Json<Policy>,
) -> Result<Json<serde_json::Value>, ApiError> {
    check_secret(&state, &headers).map_err(|(s, m)| ApiError { status: s, message: m })?;
    save_policy(&state, &email, &policy)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Json(json!({ "status": "ok", "recipient": email })))
}

async fn deliver(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<Request>,
) -> Result<Json<DeliveryReceipt>, ApiError> {
    let idem_key = headers
        .get("Idempotency-Key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or(req.id());
    if idem_key != req.id() {
        tracing::warn!(
            idem_key,
            request_id = req.id(),
            "Idempotency-Key differs from request.id; dedup uses request.id"
        );
    }
    let receipt = ingest_inbound(&state, &req, Transport::Native, 1.0).await?;
    Ok(Json(receipt))
}

#[derive(Deserialize)]
struct IngestBody {
    request: Request,
}

async fn ingest_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<IngestBody>,
) -> Result<Json<DeliveryReceipt>, ApiError> {
    check_secret(&state, &headers).map_err(|(s, m)| ApiError { status: s, message: m })?;
    let idem_key = headers
        .get("Idempotency-Key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or(body.request.id());
    if idem_key != body.request.id() {
        tracing::warn!(
            idem_key,
            request_id = body.request.id(),
            "Idempotency-Key differs from request.id; dedup uses request.id"
        );
    }
    let receipt = deliver_outbound(&state, &body.request).await?;
    Ok(Json(receipt))
}

#[derive(Deserialize)]
struct ListQuery {
    recipient: String,
    #[serde(default)]
    state: Option<String>,
}

#[derive(Serialize)]
struct RequestRow {
    id: String,
    sender: String,
    importance: String,
    intent: String,
    state: String,
    request: Request,
}

async fn list_requests(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<RequestRow>>, ApiError> {
    check_secret(&state, &headers).map_err(|(s, m)| ApiError { status: s, message: m })?;
    let state_filter = q.state.unwrap_or_else(|| "pending".into());
    let rows: Vec<(String, String, String, String, String, serde_json::Value)> = sqlx::query_as(
        r#"SELECT id, sender, importance, intent, state, request_json
           FROM orp_requests WHERE recipient = $1 AND state = $2
           ORDER BY created_at DESC LIMIT 100"#,
    )
    .bind(&q.recipient)
    .bind(&state_filter)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| ApiError::internal(e.to_string()))?;

    let out = rows
        .into_iter()
        .filter_map(|(id, sender, importance, intent, st, json)| {
            let request: Request = serde_json::from_value(json).ok()?;
            Some(RequestRow {
                id,
                sender,
                importance,
                intent,
                state: st,
                request,
            })
        })
        .collect();
    Ok(Json(out))
}

#[derive(Deserialize)]
struct FeedbackBody {
    recipient: String,
    action: String,
}

async fn submit_feedback(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<FeedbackBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let action = FeedbackAction::parse(&body.action)
        .ok_or_else(|| ApiError::bad_request("invalid action"))?;

    let row: Option<(serde_json::Value, String)> = sqlx::query_as(
        "SELECT request_json, sender FROM orp_requests WHERE id = $1 AND recipient = $2",
    )
    .bind(&id)
    .bind(&body.recipient)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| ApiError::internal(e.to_string()))?;

    let (req_json, sender) = row.ok_or_else(|| ApiError::not_found("request not found"))?;
    let req: Request =
        serde_json::from_value(req_json).map_err(|e| ApiError::internal(e.to_string()))?;

    let new_state = match action {
        FeedbackAction::Done => "done",
        FeedbackAction::Later => "later",
        FeedbackAction::Ignored | FeedbackAction::Spam => "ignored",
        FeedbackAction::UrgentOk => "pending",
        FeedbackAction::WaitingOn => "waiting_on",
    };

    sqlx::query("UPDATE orp_requests SET state = $1, updated_at = now() WHERE id = $2")
        .bind(new_state)
        .bind(&id)
        .execute(&state.pool)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;

    let feedback_id = format!("fb_{}", uuid::Uuid::new_v4());
    sqlx::query(
        r#"INSERT INTO orp_feedback (id, request_id, recipient, action) VALUES ($1, $2, $3, $4)"#,
    )
    .bind(&feedback_id)
    .bind(&id)
    .bind(&body.recipient)
    .bind(action.as_str())
    .execute(&state.pool)
    .await
    .map_err(|e| ApiError::internal(e.to_string()))?;

    let update = {
        let mut reps = state.reputation.write().await;
        let store = reps
            .entry(body.recipient.clone())
            .or_insert_with(ReputationStore::new);
        store.apply_feedback(&sender, req.body.importance, action)
    };

    Ok(Json(json!({
        "status": "ok",
        "reputation": {
            "sender": update.sender,
            "score": update.new_score,
            "effective_importance": update.effective_importance.as_str()
        }
    })))
}

#[derive(Deserialize)]
struct BridgeEmailBody {
    raw: String,
    from: String,
    to: String,
    subject: String,
    body_text: String,
}

async fn bridge_email(
    State(state): State<AppState>,
    Json(body): Json<BridgeEmailBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let hook = HeuristicInference;
    let (unsigned, confidence, transport) = wrap_legacy_email(
        &body.raw,
        &body.from,
        &body.to,
        &body.subject,
        &body.body_text,
        &hook,
    )
    .map_err(|e| ApiError::bad_request(e.to_string()))?;

    let signed = state
        .keypair
        .sign_request(&unsigned)
        .map_err(|e| ApiError::internal(e.to_string()))?;

    let receipt = ingest_inbound(&state, &signed, transport, confidence).await?;
    Ok(Json(json!({
        "status": receipt.status.as_str(),
        "id": receipt.id,
        "received_at": receipt.received_at,
        "confidence": confidence
    })))
}

#[derive(Deserialize)]
struct RegisterKeyBody {
    email: String,
    key: PublicKeyBundle,
}

async fn register_key(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<RegisterKeyBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    check_secret(&state, &headers).map_err(|(s, m)| ApiError { status: s, message: m })?;
    if body.email.is_empty() || body.key.key_id.is_empty() || body.key.value.is_empty() {
        return Err(ApiError::bad_request("email and key required"));
    }
    crate::keys::save_user_key(&state.pool, &body.email, &body.key)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let policy = load_policy(&state, &body.email).await.map_err(ApiError::from)?;
    save_policy(&state, &body.email, &policy)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Json(json!({ "status": "ok", "email": body.email, "key_id": body.key.key_id })))
}

pub async fn serve(state: AppState) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let app = router(state.clone());
    let listener = tokio::net::TcpListener::bind(state.config.bind).await?;
    tracing::info!("ORP server listening on {}", state.config.bind);
    axum::serve(listener, app).await?;
    Ok(())
}

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn bad_request(msg: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: msg.into(),
        }
    }
    fn not_found(msg: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message: msg.into(),
        }
    }
    fn internal(msg: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: msg.into(),
        }
    }
}

impl From<orp_core::OrpError> for ApiError {
    fn from(e: orp_core::OrpError) -> Self {
        match &e {
            orp_core::OrpError::PolicyViolation(_) | orp_core::OrpError::BudgetExceeded(_) => {
                ApiError::bad_request(e.to_string())
            }
            orp_core::OrpError::BadSignature | orp_core::OrpError::UnknownKey(_) => {
                ApiError::bad_request(e.to_string())
            }
            _ => ApiError::internal(e.to_string()),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        (self.status, Json(json!({ "error": self.message }))).into_response()
    }
}
