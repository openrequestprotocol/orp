use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;

use crate::error::OrpError;
use crate::policy::Policy;
use crate::request::Importance;

#[derive(Debug, Clone, Default)]
pub struct BudgetState {
    /// high-importance requests used per sender in current week window
    pub high_used: HashMap<String, u32>,
    /// unknown sender requests today
    pub unknown_today: HashMap<String, u32>,
    pub window_start: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct BudgetTracker {
    state: BudgetState,
}

impl Default for BudgetTracker {
    fn default() -> Self {
        Self {
            state: BudgetState {
                window_start: Some(Utc::now()),
                ..Default::default()
            },
        }
    }
}

impl BudgetTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn state(&self) -> &BudgetState {
        &self.state
    }

    fn maybe_roll_window(&mut self) {
        let now = Utc::now();
        if let Some(start) = self.state.window_start {
            if now - start > Duration::days(7) {
                self.state.high_used.clear();
                self.state.window_start = Some(now);
            }
        } else {
            self.state.window_start = Some(now);
        }
    }

    pub fn check_high_budget(
        &mut self,
        policy: &Policy,
        sender: &str,
        importance: Importance,
    ) -> Result<(), OrpError> {
        if importance != Importance::High {
            return Ok(());
        }
        self.maybe_roll_window();
        let limit = policy.high_budget_for_sender(sender);
        let used = self.state.high_used.get(sender).copied().unwrap_or(0);
        if used >= limit {
            return Err(OrpError::BudgetExceeded(format!(
                "sender {sender} exceeded high budget ({used}/{limit} per week)"
            )));
        }
        Ok(())
    }

    pub fn consume_high(&mut self, sender: &str, importance: Importance) {
        if importance == Importance::High {
            self.maybe_roll_window();
            *self.state.high_used.entry(sender.to_string()).or_insert(0) += 1;
        }
    }

    pub fn check_unknown_rate(
        &mut self,
        policy: &Policy,
        sender: &str,
        is_known: bool,
    ) -> Result<(), OrpError> {
        if is_known || policy.is_vip(sender) {
            return Ok(());
        }
        let used = self.state.unknown_today.get(sender).copied().unwrap_or(0);
        if used >= policy.rate_limits.unknown_per_day {
            return Err(OrpError::BudgetExceeded(format!(
                "unknown sender {sender} exceeded daily rate limit"
            )));
        }
        Ok(())
    }

    pub fn consume_unknown(&mut self, sender: &str, is_known: bool) {
        if !is_known {
            *self.state.unknown_today.entry(sender.to_string()).or_insert(0) += 1;
        }
    }
}
