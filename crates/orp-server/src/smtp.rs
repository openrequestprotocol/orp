//! Outbound SMTP transport for the email bridge.
//!
//! When `ORP_SMTP_URL` is configured, the server can actually deliver the
//! degraded RFC 5322 message produced by `orp_bridge::degrade_to_email` to a
//! non-ORP recipient. Without it, bridged mail is only queued.

use std::str::FromStr;

use lettre::address::{Address, Envelope};
use lettre::transport::smtp::AsyncSmtpTransport;
use lettre::{AsyncTransport, Tokio1Executor};
use orp_core::OrpError;

/// Send a pre-built RFC 5322 message via SMTP.
///
/// `smtp_url` is a lettre connection URL, e.g. `smtp://user:pass@host:1025`
/// (plaintext/dev), `smtp://host:587` (STARTTLS), or `smtps://host:465`
/// (implicit TLS). The message bytes are sent verbatim; the envelope sender
/// and recipient are taken from the ORP request so the bridge preserves the
/// original `from`/`to`.
pub async fn send_raw_email(
    smtp_url: &str,
    from: &str,
    to: &str,
    raw_rfc822: &[u8],
) -> Result<(), OrpError> {
    let mailer = AsyncSmtpTransport::<Tokio1Executor>::from_url(smtp_url)
        .map_err(|e| OrpError::Transport(format!("smtp config: {e}")))?
        .build();

    let from_addr = Address::from_str(from)
        .map_err(|e| OrpError::Transport(format!("smtp from address {from:?}: {e}")))?;
    let to_addr = Address::from_str(to)
        .map_err(|e| OrpError::Transport(format!("smtp to address {to:?}: {e}")))?;

    let envelope = Envelope::new(Some(from_addr), vec![to_addr])
        .map_err(|e| OrpError::Transport(format!("smtp envelope: {e}")))?;

    mailer
        .send_raw(&envelope, raw_rfc822)
        .await
        .map_err(|e| OrpError::Transport(format!("smtp send: {e}")))?;

    Ok(())
}
