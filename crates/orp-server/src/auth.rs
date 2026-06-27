use axum::http::HeaderMap;
use axum::http::StatusCode;

use crate::state::AppState;

pub fn check_secret(state: &AppState, headers: &HeaderMap) -> Result<(), (StatusCode, String)> {
    let Some(expected) = state.config.shared_secret.as_ref() else {
        return Ok(());
    };
    let provided = headers
        .get("x-orp-secret")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if provided == expected {
        Ok(())
    } else {
        Err((StatusCode::UNAUTHORIZED, "invalid orp secret".into()))
    }
}
