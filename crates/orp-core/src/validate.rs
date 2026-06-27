use crate::error::OrpError;
use crate::policy::{Policy, UnknownSenderPolicy};
use crate::request::{Importance, Request, StakeKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyCheckResult {
    Accept,
    Reject(String),
    DowngradeImportance(Importance),
}

pub fn validate_against_policy(
    req: &Request,
    policy: &Policy,
    is_known_sender: bool,
) -> Result<PolicyCheckResult, OrpError> {
    if !req.to_addr().eq_ignore_ascii_case(&policy.recipient) {
        return Ok(PolicyCheckResult::Reject(
            "recipient mismatch".into(),
        ));
    }

    if policy.is_blocked(req.from_addr()) {
        return Ok(PolicyCheckResult::Reject("sender blocked".into()));
    }

    let intent = req.body.intent.as_str();
    if !policy.accepts.intents.iter().any(|i| i == intent) {
        return Ok(PolicyCheckResult::Reject(format!(
            "intent '{intent}' not accepted"
        )));
    }

    for field in &policy.accepts.require {
        match field.as_str() {
            "summary" if req.body.summary.trim().is_empty() => {
                return Ok(PolicyCheckResult::Reject("summary required".into()));
            }
            "deadline" if req.body.deadline.is_none() => {
                return Ok(PolicyCheckResult::Reject("deadline required".into()));
            }
            "stake" if req.body.stake.kind == StakeKind::None => {
                return Ok(PolicyCheckResult::Reject("stake required".into()));
            }
            _ => {}
        }
    }

    if req.body.summary.len() > policy.limits.max_summary_len {
        return Ok(PolicyCheckResult::Reject(format!(
            "summary exceeds max length of {}",
            policy.limits.max_summary_len
        )));
    }
    if policy.limits.payload_len(req) as u64 > policy.limits.max_payload_bytes {
        return Ok(PolicyCheckResult::Reject(format!(
            "payload exceeds max size of {} bytes",
            policy.limits.max_payload_bytes
        )));
    }

    if !is_known_sender && !policy.is_vip(req.from_addr()) {
        match policy.senders.unknown {
            UnknownSenderPolicy::Reject => {
                return Ok(PolicyCheckResult::Reject(
                    "unknown senders not accepted".into(),
                ));
            }
            UnknownSenderPolicy::StakeRequired => {
                if req.body.stake.kind == StakeKind::None {
                    return Ok(PolicyCheckResult::Reject(
                        "stake required for unknown sender".into(),
                    ));
                }
            }
            UnknownSenderPolicy::Allow => {}
        }
    }

    // VIP bypass: high importance always accepted for VIPs when enabled
    if policy.senders.vip_bypass
        && policy.is_vip(req.from_addr())
        && req.body.importance == Importance::High
    {
        return Ok(PolicyCheckResult::Accept);
    }

    Ok(PolicyCheckResult::Accept)
}
