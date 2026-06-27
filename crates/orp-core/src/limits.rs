use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct LimitsPolicy {
    #[serde(default = "default_max_payload_bytes")]
    pub max_payload_bytes: u64,
    #[serde(default = "default_max_summary_len")]
    pub max_summary_len: usize,
}

pub const DEFAULT_MAX_PAYLOAD_BYTES: u64 = 256 * 1024;
pub const DEFAULT_MAX_SUMMARY_LEN: usize = 280;

fn default_max_payload_bytes() -> u64 {
    DEFAULT_MAX_PAYLOAD_BYTES
}

fn default_max_summary_len() -> usize {
    DEFAULT_MAX_SUMMARY_LEN
}

impl Default for LimitsPolicy {
    fn default() -> Self {
        Self {
            max_payload_bytes: DEFAULT_MAX_PAYLOAD_BYTES,
            max_summary_len: DEFAULT_MAX_SUMMARY_LEN,
        }
    }
}

impl LimitsPolicy {
    pub fn payload_len(&self, req: &crate::Request) -> usize {
        let mut n = req.body.payload.text.len();
        if let Some(html) = &req.body.payload.html {
            n += html.len();
        }
        if let Some(subject) = &req.body.payload.subject {
            n += subject.len();
        }
        n
    }
}
