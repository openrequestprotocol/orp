//! Open Request Protocol core library.

pub mod budget;
pub mod canonical;
pub mod discovery;
pub mod error;
pub mod limits;
pub mod policy;
pub mod receipt;
pub mod reputation;
pub mod request;
pub mod response;
pub mod sign;
pub mod validate;

pub use budget::{BudgetState, BudgetTracker};
pub use discovery::{DiscoveryDocument, domain_from_email};
pub use error::OrpError;
pub use limits::{LimitsPolicy, DEFAULT_MAX_PAYLOAD_BYTES, DEFAULT_MAX_SUMMARY_LEN};
pub use policy::{Policy, SenderPolicy, UnknownSenderPolicy};
pub use receipt::{DeliveryReceipt, DeliveryStatus};
pub use reputation::{FeedbackAction, ReputationStore, ReputationUpdate};
pub use request::{
    Importance, Intent, Payload, Request, Stake, StakeKind, Transport, UnsignedRequest,
};
pub use response::{Response, ResponseStatus, UnsignedResponse};
pub use sign::{KeyPair, PublicKeyBundle, SignatureBundle, verify_request, verify_response};
pub use validate::{PolicyCheckResult, validate_against_policy};

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn sign_and_verify_roundtrip() {
        let kp = KeyPair::generate("key1");
        let req = UnsignedRequest::new(
            "alice@example.com",
            "bob@example.com",
            Intent::Reply,
            "Review the deck",
            Importance::Normal,
            Payload {
                text: "Please review by Friday.".into(),
                html: None,
                subject: Some("Deck review".into()),
                action: None,
            },
        );
        let signed = kp.sign_request(&req).unwrap();
        verify_request(&signed, &[kp.public_bundle()]).unwrap();
    }

    #[test]
    fn policy_rejects_blocked_sender() {
        let kp = KeyPair::generate("key1");
        let mut policy = Policy::default_for("bob@example.com");
        policy.senders.blocked.push("spam@evil.com".into());

        let req = UnsignedRequest::new(
            "spam@evil.com",
            "bob@example.com",
            Intent::Fyi,
            "Buy now",
            Importance::Low,
            Payload {
                text: "spam".into(),
                html: None,
                subject: None,
                action: None,
            },
        );
        let signed = kp.sign_request(&req).unwrap();
        let result = validate_against_policy(&signed, &policy, false).unwrap();
        assert!(matches!(result, PolicyCheckResult::Reject(_)));
    }

    #[test]
    fn budget_limits_high_importance() {
        let mut tracker = BudgetTracker::new();
        let policy = Policy::default_for("bob@example.com");
        tracker
            .check_high_budget(&policy, "alice@example.com", Importance::High)
            .unwrap();
        tracker.consume_high("alice@example.com", Importance::High);
        let err = tracker
            .check_high_budget(&policy, "alice@example.com", Importance::High)
            .unwrap_err();
        assert!(err.to_string().contains("budget"));
    }

    #[test]
    fn reputation_adjusts_inflated_importance() {
        let mut rep = ReputationStore::new();
        rep.apply_feedback("alice@example.com", Importance::High, FeedbackAction::Later);
        rep.apply_feedback("alice@example.com", Importance::High, FeedbackAction::Later);
        let adjusted = rep.adjust_importance("alice@example.com", Importance::High);
        assert_eq!(adjusted, Importance::Normal);
    }

    #[test]
    fn policy_rejects_oversized_summary() {
        let kp = KeyPair::generate("key1");
        let mut policy = Policy::default_for("bob@example.com");
        policy.limits.max_summary_len = 10;

        let req = UnsignedRequest::new(
            "alice@example.com",
            "bob@example.com",
            Intent::Fyi,
            "this summary is definitely too long for the limit",
            Importance::Low,
            Payload {
                text: "ok".into(),
                html: None,
                subject: None,
                action: None,
            },
        );
        let signed = kp.sign_request(&req).unwrap();
        let result = validate_against_policy(&signed, &policy, true).unwrap();
        assert!(matches!(result, PolicyCheckResult::Reject(_)));
    }

    #[test]
    fn policy_rejects_oversized_payload() {
        let kp = KeyPair::generate("key1");
        let mut policy = Policy::default_for("bob@example.com");
        policy.limits.max_payload_bytes = 10;

        let req = UnsignedRequest::new(
            "alice@example.com",
            "bob@example.com",
            Intent::Fyi,
            "short",
            Importance::Low,
            Payload {
                text: "this payload is way too large".into(),
                html: None,
                subject: None,
                action: None,
            },
        );
        let signed = kp.sign_request(&req).unwrap();
        let result = validate_against_policy(&signed, &policy, true).unwrap();
        assert!(matches!(result, PolicyCheckResult::Reject(_)));
    }
}
