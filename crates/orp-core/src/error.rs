use thiserror::Error;

#[derive(Debug, Error)]
pub enum OrpError {
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("invalid policy: {0}")]
    InvalidPolicy(String),
    #[error("signature verification failed")]
    BadSignature,
    #[error("policy violation: {0}")]
    PolicyViolation(String),
    #[error("budget exceeded: {0}")]
    BudgetExceeded(String),
    #[error("serialization error: {0}")]
    Serialization(String),
    #[error("unknown key id: {0}")]
    UnknownKey(String),
    #[error("transport error: {0}")]
    Transport(String),
}
