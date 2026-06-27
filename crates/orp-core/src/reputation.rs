use std::collections::HashMap;

use crate::request::Importance;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedbackAction {
    Done,
    Later,
    UrgentOk,
    Spam,
    Ignored,
    WaitingOn,
}

impl FeedbackAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Done => "done",
            Self::Later => "later",
            Self::UrgentOk => "urgent_ok",
            Self::Spam => "spam",
            Self::Ignored => "ignored",
            Self::WaitingOn => "waiting_on",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "done" => Some(Self::Done),
            "later" => Some(Self::Later),
            "urgent_ok" => Some(Self::UrgentOk),
            "spam" => Some(Self::Spam),
            "ignored" => Some(Self::Ignored),
            "waiting_on" => Some(Self::WaitingOn),
            _ => None,
        }
    }
}

/// Per-sender reputation score in [-1.0, 1.0].
/// Positive = sender's declared importance tends to match recipient behavior.
#[derive(Debug, Clone, Default)]
pub struct ReputationStore {
    scores: HashMap<String, f64>,
    high_claims: HashMap<String, u32>,
    high_confirmed: HashMap<String, u32>,
}

#[derive(Debug, Clone)]
pub struct ReputationUpdate {
    pub sender: String,
    pub old_score: f64,
    pub new_score: f64,
    pub effective_importance: Importance,
}

impl ReputationStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn score(&self, sender: &str) -> f64 {
        *self.scores.get(sender).unwrap_or(&0.0)
    }

    /// Apply feedback and return updated effective importance for future requests.
    pub fn apply_feedback(
        &mut self,
        sender: &str,
        declared: Importance,
        action: FeedbackAction,
    ) -> ReputationUpdate {
        let old = self.score(sender);
        let delta = match (declared, action) {
            (Importance::High, FeedbackAction::UrgentOk) => 0.15,
            (Importance::High, FeedbackAction::Done) => 0.05,
            (Importance::High, FeedbackAction::Later) => -0.2,
            (Importance::High, FeedbackAction::Spam) => -0.5,
            (Importance::High, FeedbackAction::Ignored) => -0.1,
            (Importance::Normal, FeedbackAction::Later) => -0.05,
            (Importance::Low, FeedbackAction::UrgentOk) => 0.1,
            _ => 0.0,
        };

        if declared == Importance::High {
            *self.high_claims.entry(sender.to_string()).or_insert(0) += 1;
            if matches!(action, FeedbackAction::UrgentOk | FeedbackAction::Done) {
                *self.high_confirmed.entry(sender.to_string()).or_insert(0) += 1;
            }
        }

        let new = (old + delta).clamp(-1.0, 1.0);
        self.scores.insert(sender.to_string(), new);

        ReputationUpdate {
            sender: sender.to_string(),
            old_score: old,
            new_score: new,
            effective_importance: self.adjust_importance(sender, declared),
        }
    }

    /// Downgrade declared importance based on sender reputation.
    pub fn adjust_importance(&self, sender: &str, declared: Importance) -> Importance {
        let score = self.score(sender);
        match declared {
            Importance::High if score < -0.3 => Importance::Normal,
            Importance::High if score < -0.6 => Importance::Low,
            Importance::Normal if score < -0.5 => Importance::Low,
            _ => declared,
        }
    }

    pub fn high_confirmation_rate(&self, sender: &str) -> f64 {
        let claims = self.high_claims.get(sender).copied().unwrap_or(0);
        let confirmed = self.high_confirmed.get(sender).copied().unwrap_or(0);
        if claims == 0 {
            return 0.5;
        }
        confirmed as f64 / claims as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spam_downgrades_reputation() {
        let mut store = ReputationStore::new();
        let u = store.apply_feedback("a@x.com", Importance::High, FeedbackAction::Spam);
        assert!(u.new_score < 0.0);
        assert_eq!(
            store.adjust_importance("a@x.com", Importance::High),
            Importance::Normal
        );
    }
}
