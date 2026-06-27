use crate::embed::{embed_in_email, ORP_HEADER};
use orp_core::{Request, OrpError};

pub const VIRAL_FOOTER: &str = "\n\n---\nSent via Open Request Protocol — claim your list at https://openrequestprotocol.org";

/// Degrade a native ORP request to RFC 5322 email for non-ORP recipients.
pub fn degrade_to_email(req: &Request, subject: Option<&str>) -> Result<String, OrpError> {
    let mut email = embed_in_email(req, subject)?;
    // Append viral footer to plain part by inserting before closing boundary
    if let Some(idx) = email.rfind("\r\n\r\n--") {
        let (head, tail) = email.split_at(idx);
        email = format!("{head}{VIRAL_FOOTER}{tail}");
    } else {
        email.push_str(VIRAL_FOOTER);
    }
    Ok(email)
}

pub fn has_orp_header(raw: &str) -> bool {
    raw.lines().any(|l| l.starts_with(ORP_HEADER))
}
