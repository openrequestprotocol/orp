use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::request::PayloadAction;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseStatus {
    Accepted,
    Declined,
    Done,
    NeedsInfo,
}

impl ResponseStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Declined => "declined",
            Self::Done => "done",
            Self::NeedsInfo => "needs_info",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "accepted" => Some(Self::Accepted),
            "declined" => Some(Self::Declined),
            "done" => Some(Self::Done),
            "needs_info" => Some(Self::NeedsInfo),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnsignedResponse {
    pub v: String,
    pub id: String,
    #[serde(rename = "ref")]
    pub ref_id: String,
    pub from: String,
    pub to: String,
    pub status: ResponseStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<PayloadAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    #[serde(flatten)]
    pub body: UnsignedResponse,
    pub sig: crate::sign::SignatureBundle,
}

impl Response {
    pub fn new(body: UnsignedResponse, sig: crate::sign::SignatureBundle) -> Self {
        Self { body, sig }
    }

    pub fn id(&self) -> &str {
        &self.body.id
    }

    pub fn ref_id(&self) -> &str {
        &self.body.ref_id
    }

    pub fn from_addr(&self) -> &str {
        &self.body.from
    }

    pub fn to_addr(&self) -> &str {
        &self.body.to
    }
}

impl UnsignedResponse {
    pub fn new(
        ref_id: impl Into<String>,
        from: impl Into<String>,
        to: impl Into<String>,
        status: ResponseStatus,
    ) -> Self {
        Self {
            v: "0.2".into(),
            id: format!("resp_{}", uuid::Uuid::new_v4().simple()),
            ref_id: ref_id.into(),
            from: from.into(),
            to: to.into(),
            status,
            reason: None,
            result: None,
            created_at: Some(Utc::now()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sign::{KeyPair, verify_response};

    #[test]
    fn sign_and_verify_response_roundtrip() {
        let kp = KeyPair::generate("resp-key-1");
        let unsigned = UnsignedResponse::new(
            "req_test001",
            "bob@example.com",
            "alice@example.com",
            ResponseStatus::Done,
        );
        let signed = kp.sign_response(&unsigned).unwrap();
        verify_response(&signed, &[kp.public_bundle()]).unwrap();
        assert_eq!(signed.ref_id(), "req_test001");
        assert_eq!(signed.body.status, ResponseStatus::Done);
    }
}
