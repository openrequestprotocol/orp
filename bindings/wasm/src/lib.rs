use wasm_bindgen::prelude::*;
use orp_core::{
    validate_against_policy, verify_request, Policy, PolicyCheckResult, PublicKeyBundle, Request,
};

#[wasm_bindgen]
pub fn validate_request_against_policy(
    request_json: &str,
    policy_json: &str,
    is_known_sender: bool,
) -> Result<String, JsValue> {
    let req: Request = serde_json::from_str(request_json)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let policy: Policy = serde_json::from_str(policy_json)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let result = validate_against_policy(&req, &policy, is_known_sender)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let out = match result {
        PolicyCheckResult::Accept => "accept".to_string(),
        PolicyCheckResult::Reject(r) => format!("reject:{r}"),
        PolicyCheckResult::DowngradeImportance(i) => format!("downgrade:{}", i.as_str()),
    };
    Ok(out)
}

#[wasm_bindgen]
pub fn verify_request_signature(request_json: &str, keys_json: &str) -> Result<(), JsValue> {
    let req: Request = serde_json::from_str(request_json)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let keys: Vec<PublicKeyBundle> = serde_json::from_str(keys_json)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    verify_request(&req, &keys).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen]
pub fn canonical_json(value_json: &str) -> Result<String, JsValue> {
    let value: serde_json::Value = serde_json::from_str(value_json)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let bytes = orp_core::canonical::to_canonical_bytes(&value)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    String::from_utf8(bytes).map_err(|e| JsValue::from_str(&e.to_string()))
}
