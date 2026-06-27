use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryStatus {
    Accepted,
    Duplicate,
    Queued,
}

impl DeliveryStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Duplicate => "duplicate",
            Self::Queued => "queued",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryReceipt {
    pub status: DeliveryStatus,
    pub id: String,
    pub received_at: DateTime<Utc>,
}

impl DeliveryReceipt {
    pub fn accepted(id: impl Into<String>) -> Self {
        Self {
            status: DeliveryStatus::Accepted,
            id: id.into(),
            received_at: Utc::now(),
        }
    }

    pub fn duplicate(id: impl Into<String>, received_at: DateTime<Utc>) -> Self {
        Self {
            status: DeliveryStatus::Duplicate,
            id: id.into(),
            received_at,
        }
    }

    pub fn queued(id: impl Into<String>) -> Self {
        Self {
            status: DeliveryStatus::Queued,
            id: id.into(),
            received_at: Utc::now(),
        }
    }
}
