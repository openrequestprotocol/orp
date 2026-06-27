use chrono::Utc;
use orp_core::{
    Importance, Intent, OrpError, Payload, Stake, StakeKind, Transport, UnsignedRequest,
};

/// Result of inference from legacy email without embedded ORP data.
#[derive(Debug, Clone)]
pub struct InferredRequest {
    pub request: UnsignedRequest,
    pub confidence: f64,
}

/// Hook for pluggable inference (e.g. Mooncake's LLM triage).
pub trait InferenceHook: Send + Sync {
    fn infer(
        &self,
        from: &str,
        to: &str,
        subject: &str,
        body_text: &str,
    ) -> Result<InferredRequest, OrpError>;
}

/// Heuristic inference fallback when no LLM is available.
pub struct HeuristicInference;

impl InferenceHook for HeuristicInference {
    fn infer(
        &self,
        from: &str,
        to: &str,
        subject: &str,
        body_text: &str,
    ) -> Result<InferredRequest, OrpError> {
        let lower = format!("{subject} {body_text}").to_lowercase();
        let (intent, importance, confidence) = if lower.contains("unsubscribe")
            || lower.contains("newsletter")
        {
            (Intent::Fyi, Importance::Low, 0.85)
        } else if lower.contains("invoice") || lower.contains("payment due") {
            (Intent::Pay, Importance::Normal, 0.8)
        } else if lower.contains("docusign") || lower.contains("please sign") {
            (Intent::Sign, Importance::Normal, 0.8)
        } else if lower.contains('?') || subject.to_lowercase().starts_with("re:") {
            (Intent::Reply, Importance::Normal, 0.7)
        } else if lower.contains("approve") || lower.contains("decision") {
            (Intent::Decide, Importance::Normal, 0.75)
        } else if lower.contains("meeting") || lower.contains("calendar") {
            (Intent::Schedule, Importance::Normal, 0.75)
        } else {
            (Intent::Read, Importance::Normal, 0.5)
        };

        let summary = if subject.trim().is_empty() {
            truncate(body_text, 120)
        } else {
            subject.to_string()
        };

        let mut req = UnsignedRequest::new(from, to, intent, summary, importance, Payload {
            text: body_text.to_string(),
            html: None,
            subject: Some(subject.to_string()),
            action: None,
        });
        req.transport = Some(Transport::Inferred);
        req.stake = Stake {
            kind: StakeKind::None,
            receipt: None,
            amount_cents: None,
        };
        req.created_at = Some(Utc::now());

        Ok(InferredRequest {
            request: req,
            confidence,
        })
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(max).collect::<String>())
    }
}

/// Wrap legacy email into Request: extract embedded ORP or run inference.
pub fn wrap_legacy_email(
    raw: &str,
    from: &str,
    to: &str,
    subject: &str,
    body_text: &str,
    hook: &dyn InferenceHook,
) -> Result<(UnsignedRequest, f64, Transport), OrpError> {
    if let Some(req) = crate::extract::extract_from_email(raw)? {
        let mut body = req.body;
        body.transport = Some(Transport::EmailBridge);
        return Ok((body, 1.0, Transport::EmailBridge));
    }

    let inferred = hook.infer(from, to, subject, body_text)?;
    Ok((
        inferred.request,
        inferred.confidence,
        Transport::Inferred,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heuristic_detects_newsletter() {
        let hook = HeuristicInference;
        let r = hook
            .infer(
                "news@corp.com",
                "me@x.com",
                "Weekly digest",
                "Click unsubscribe to opt out",
            )
            .unwrap();
        assert_eq!(r.request.intent, Intent::Fyi);
        assert_eq!(r.request.importance, Importance::Low);
    }
}
