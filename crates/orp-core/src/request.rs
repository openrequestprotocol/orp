use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Intent {
    Read,
    Reply,
    Decide,
    Pay,
    Sign,
    Schedule,
    #[serde(rename = "do")]
    Do,
    Fyi,
}

impl Intent {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Reply => "reply",
            Self::Decide => "decide",
            Self::Pay => "pay",
            Self::Sign => "sign",
            Self::Schedule => "schedule",
            Self::Do => "do",
            Self::Fyi => "fyi",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Importance {
    Low,
    Normal,
    High,
}

impl Importance {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Normal => "normal",
            Self::High => "high",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Transport {
    Native,
    EmailBridge,
    Inferred,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payload {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub html: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<PayloadAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayloadAction {
    #[serde(rename = "type")]
    pub action_type: String,
    #[serde(default)]
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StakeKind {
    None,
    Reputation,
    Escrow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stake {
    pub kind: StakeKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receipt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount_cents: Option<i64>,
}

impl Default for Stake {
    fn default() -> Self {
        Self {
            kind: StakeKind::None,
            receipt: None,
            amount_cents: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnsignedRequest {
    pub v: String,
    pub id: String,
    pub from: String,
    pub to: String,
    pub intent: Intent,
    pub summary: String,
    pub importance: Importance,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deadline: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread: Option<String>,
    pub payload: Payload,
    #[serde(default)]
    pub stake: Stake,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transport: Option<Transport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    #[serde(flatten)]
    pub body: UnsignedRequest,
    pub sig: crate::sign::SignatureBundle,
}

impl Request {
    pub fn new(body: UnsignedRequest, sig: crate::sign::SignatureBundle) -> Self {
        Self { body, sig }
    }

    pub fn id(&self) -> &str {
        &self.body.id
    }

    pub fn from_addr(&self) -> &str {
        &self.body.from
    }

    pub fn to_addr(&self) -> &str {
        &self.body.to
    }
}

impl UnsignedRequest {
    pub fn new(
        from: impl Into<String>,
        to: impl Into<String>,
        intent: Intent,
        summary: impl Into<String>,
        importance: Importance,
        payload: Payload,
    ) -> Self {
        Self {
            v: "0.2".into(),
            id: format!("req_{}", uuid::Uuid::new_v4().simple()),
            from: from.into(),
            to: to.into(),
            intent,
            summary: summary.into(),
            importance,
            deadline: None,
            thread: None,
            payload,
            stake: Stake::default(),
            transport: None,
            created_at: Some(Utc::now()),
        }
    }
}
