use chrono::{DateTime, Duration, Utc};

use crate::error::OrpError;
use crate::policy::Policy;
use crate::request::Importance;

/// High-importance allowance window length.
pub const HIGH_WINDOW_DAYS: i64 = 7;
/// Unknown-sender rate-limit window length.
pub const UNKNOWN_WINDOW_DAYS: i64 = 1;

/// Per `(recipient, sender)` budget snapshot. Mirrors a row in
/// `orp_budget_state`.
///
/// The high-importance (weekly) and unknown-sender (daily) windows roll
/// **independently and per sender**:
///
/// - Each sender carries its own window anchors, so one sender's reset never
///   leaks into another's allowance.
/// - The daily unknown-sender limit actually resets every day instead of
///   living for the whole week (the previous implementation never reset it).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SenderBudget {
    pub high_used: u32,
    pub high_window_start: DateTime<Utc>,
    pub unknown_used: u32,
    pub unknown_window_start: DateTime<Utc>,
}

impl SenderBudget {
    /// A fresh budget with both windows anchored at `now`.
    pub fn fresh(now: DateTime<Utc>) -> Self {
        Self {
            high_used: 0,
            high_window_start: now,
            unknown_used: 0,
            unknown_window_start: now,
        }
    }

    /// Reset each counter whose window has fully elapsed. Windows are
    /// independent, so an expired daily window does not disturb the weekly one.
    pub fn roll(&mut self, now: DateTime<Utc>) {
        if now - self.high_window_start >= Duration::days(HIGH_WINDOW_DAYS) {
            self.high_used = 0;
            self.high_window_start = now;
        }
        if now - self.unknown_window_start >= Duration::days(UNKNOWN_WINDOW_DAYS) {
            self.unknown_used = 0;
            self.unknown_window_start = now;
        }
    }

    /// Error if a `high`-importance request would exceed the weekly allowance.
    pub fn check_high(&self, importance: Importance, limit: u32) -> Result<(), OrpError> {
        if importance == Importance::High && self.high_used >= limit {
            return Err(OrpError::BudgetExceeded(format!(
                "exceeded high-importance budget ({}/{} per week)",
                self.high_used, limit
            )));
        }
        Ok(())
    }

    /// Error if an unknown sender would exceed the daily rate limit.
    pub fn check_unknown(&self, limit: u32) -> Result<(), OrpError> {
        if self.unknown_used >= limit {
            return Err(OrpError::BudgetExceeded(format!(
                "unknown sender exceeded daily rate limit ({}/{})",
                self.unknown_used, limit
            )));
        }
        Ok(())
    }

    /// Record consumption of a `high`-importance request.
    pub fn consume_high(&mut self, importance: Importance) {
        if importance == Importance::High {
            self.high_used += 1;
        }
    }

    /// Record consumption of an unknown-sender request.
    pub fn consume_unknown(&mut self) {
        self.unknown_used += 1;
    }
}

/// Resolve the effective high-importance weekly limit for a sender.
pub fn high_limit(policy: &Policy, sender: &str) -> u32 {
    policy.high_budget_for_sender(sender)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(secs: i64) -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000 + secs, 0).unwrap()
    }

    #[test]
    fn high_budget_blocks_after_limit_then_resets_weekly() {
        let start = at(0);
        let mut b = SenderBudget::fresh(start);

        b.check_high(Importance::High, 1).unwrap();
        b.consume_high(Importance::High);
        // Second high within the same week is over budget.
        assert!(b.check_high(Importance::High, 1).is_err());

        // One day later: still the same weekly window, still blocked.
        b.roll(at(60 * 60 * 24));
        assert!(b.check_high(Importance::High, 1).is_err());

        // Eight days later: the weekly window has rolled, allowance restored.
        b.roll(at(60 * 60 * 24 * 8));
        b.check_high(Importance::High, 1).unwrap();
        assert_eq!(b.high_used, 0);
    }

    #[test]
    fn non_high_importance_never_consumes_high_budget() {
        let mut b = SenderBudget::fresh(at(0));
        b.consume_high(Importance::Normal);
        b.consume_high(Importance::Low);
        assert_eq!(b.high_used, 0);
        b.check_high(Importance::Normal, 1).unwrap();
    }

    #[test]
    fn unknown_rate_resets_daily_not_weekly() {
        let mut b = SenderBudget::fresh(at(0));

        b.consume_unknown();
        b.consume_unknown();
        b.consume_unknown();
        assert!(b.check_unknown(3).is_err());

        // A few hours later, same day: still blocked.
        b.roll(at(60 * 60 * 6));
        assert!(b.check_unknown(3).is_err());

        // Next day: daily window rolls and the limit resets. This is the bug
        // the old implementation had — it never reset the daily counter.
        b.roll(at(60 * 60 * 25));
        b.check_unknown(3).unwrap();
        assert_eq!(b.unknown_used, 0);
    }

    #[test]
    fn windows_roll_independently() {
        let mut b = SenderBudget::fresh(at(0));
        b.consume_high(Importance::High);
        b.consume_unknown();

        // After ~2 days only the daily window should have reset.
        b.roll(at(60 * 60 * 24 * 2));
        assert_eq!(b.unknown_used, 0, "daily window should have reset");
        assert_eq!(b.high_used, 1, "weekly window should NOT have reset yet");
    }
}
