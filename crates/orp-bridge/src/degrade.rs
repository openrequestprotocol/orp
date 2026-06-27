use crate::embed::{embed_in_email, ORP_HEADER};
use orp_core::{Request, OrpError};

/// Degrade a native ORP request to RFC 5322 email for non-ORP recipients.
///
/// Adds machine-readable ORP headers (`X-ORP-Request`, `X-ORP-Info`) via
/// `embed_in_email`. Per SPEC.md the bridge MUST NOT modify the sender's
/// visible message body for promotional purposes — no footer is injected.
pub fn degrade_to_email(req: &Request, subject: Option<&str>) -> Result<String, OrpError> {
    embed_in_email(req, subject)
}

pub fn has_orp_header(raw: &str) -> bool {
    raw.lines().any(|l| l.starts_with(ORP_HEADER))
}
