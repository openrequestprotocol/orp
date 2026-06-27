use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::limits::LimitsPolicy;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub v: String,
    pub recipient: String,
    pub accepts: AcceptsPolicy,
    #[serde(default)]
    pub senders: SenderPolicy,
    #[serde(default)]
    pub budgets: BudgetPolicy,
    #[serde(default)]
    pub rate_limits: RateLimits,
    #[serde(default)]
    pub limits: LimitsPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcceptsPolicy {
    pub intents: Vec<String>,
    #[serde(default)]
    pub require: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SenderPolicy {
    #[serde(default = "default_true")]
    pub vip_bypass: bool,
    #[serde(default)]
    pub unknown: UnknownSenderPolicy,
    #[serde(default)]
    pub blocked: Vec<String>,
    #[serde(default)]
    pub vip: Vec<String>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum UnknownSenderPolicy {
    #[default]
    StakeRequired,
    Reject,
    Allow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetPolicy {
    #[serde(default = "default_high_per_week")]
    pub default_high_per_week: u32,
    #[serde(default)]
    pub per_sender_overrides: HashMap<String, SenderBudgetOverride>,
}

fn default_high_per_week() -> u32 {
    1
}

impl Default for BudgetPolicy {
    fn default() -> Self {
        Self {
            default_high_per_week: 1,
            per_sender_overrides: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SenderBudgetOverride {
    pub high_per_week: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimits {
    #[serde(default = "default_unknown_per_day")]
    pub unknown_per_day: u32,
}

fn default_unknown_per_day() -> u32 {
    3
}

impl Default for RateLimits {
    fn default() -> Self {
        Self {
            unknown_per_day: 3,
        }
    }
}

impl Default for SenderPolicy {
    fn default() -> Self {
        Self {
            vip_bypass: true,
            unknown: UnknownSenderPolicy::StakeRequired,
            blocked: Vec::new(),
            vip: Vec::new(),
        }
    }
}

impl Policy {
    pub fn default_for(recipient: impl Into<String>) -> Self {
        Self {
            v: "0.2".into(),
            recipient: recipient.into(),
            accepts: AcceptsPolicy {
                intents: vec![
                    "read".into(),
                    "reply".into(),
                    "decide".into(),
                    "pay".into(),
                    "sign".into(),
                    "schedule".into(),
                    "fyi".into(),
                ],
                require: vec!["summary".into()],
            },
            senders: SenderPolicy::default(),
            budgets: BudgetPolicy::default(),
            rate_limits: RateLimits::default(),
            limits: LimitsPolicy::default(),
        }
    }

    pub fn high_budget_for_sender(&self, sender: &str) -> u32 {
        self.budgets
            .per_sender_overrides
            .get(sender)
            .map(|o| o.high_per_week)
            .unwrap_or(self.budgets.default_high_per_week)
    }

    pub fn is_vip(&self, sender: &str) -> bool {
        self.senders.vip.iter().any(|v| v.eq_ignore_ascii_case(sender))
    }

    pub fn is_blocked(&self, sender: &str) -> bool {
        self.senders
            .blocked
            .iter()
            .any(|b| b.eq_ignore_ascii_case(sender))
    }
}
