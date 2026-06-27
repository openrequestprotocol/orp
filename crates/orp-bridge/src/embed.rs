use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use mail_builder::{
    headers::text::Text,
    mime::{BodyPart, MimePart},
    MessageBuilder,
};
use mailparse::MailHeaderMap;
use orp_core::{OrpError, Request};
use serde_json;

pub const ORP_HEADER: &str = "X-ORP-Request";
pub const ORP_INFO_HEADER: &str = "X-ORP-Info";
pub const ORP_INFO_URL: &str = "https://openrequestprotocol.org";
pub const ORP_MIME_TYPE: &str = "application/orp+json";

/// Embed a signed Request into email headers and MIME structure.
pub fn embed_in_email(req: &Request, subject: Option<&str>) -> Result<String, OrpError> {
    let json = serde_json::to_string(req).map_err(|e| OrpError::Serialization(e.to_string()))?;
    let encoded = URL_SAFE_NO_PAD.encode(json.as_bytes());
    let subj = subject
        .or(req.body.payload.subject.as_deref())
        .unwrap_or(&req.body.summary);

    let alt = MimePart::new(
        "multipart/alternative",
        vec![
            MimePart::new(
                "text/plain; charset=utf-8",
                BodyPart::Text(req.body.payload.text.clone().into()),
            )
            .inline(),
            MimePart::new(ORP_MIME_TYPE, BodyPart::Text(json.clone().into())).inline(),
        ],
    );

    MessageBuilder::new()
        .from(req.from_addr())
        .to(req.to_addr())
        .subject(Text::from(subj.to_string()))
        .header(ORP_HEADER, Text::from(encoded))
        .header(ORP_INFO_HEADER, Text::from(ORP_INFO_URL.to_string()))
        .body(alt)
        .write_to_string()
        .map_err(|e| OrpError::Serialization(e.to_string()))
}

/// Extract ORP Request from parsed email if embedded.
pub fn extract_from_email(raw: &str) -> Result<Option<Request>, OrpError> {
    let parsed = mailparse::parse_mail(raw.as_bytes())
        .map_err(|e| OrpError::Serialization(e.to_string()))?;

    if let Some(req) = extract_from_header(&parsed)? {
        return Ok(Some(req));
    }

    extract_from_mime(&parsed)
}

fn extract_from_header(parsed: &mailparse::ParsedMail) -> Result<Option<Request>, OrpError> {
    let headers = parsed.get_headers();
    let val = headers.get_first_value(ORP_HEADER);
    let Some(encoded) = val else {
        return Ok(None);
    };
    let bytes = URL_SAFE_NO_PAD
        .decode(encoded.trim())
        .map_err(|e| OrpError::Serialization(e.to_string()))?;
    let req: Request = serde_json::from_slice(&bytes)
        .map_err(|e| OrpError::Serialization(e.to_string()))?;
    Ok(Some(req))
}

fn extract_from_mime(parsed: &mailparse::ParsedMail) -> Result<Option<Request>, OrpError> {
    for part in &parsed.subparts {
        let mimetype = part.ctype.mimetype.as_str();
        if mimetype == ORP_MIME_TYPE {
            let body = part
                .get_body()
                .map_err(|e| OrpError::Serialization(e.to_string()))?;
            let req: Request = serde_json::from_str(&body)
                .map_err(|e| OrpError::Serialization(e.to_string()))?;
            return Ok(Some(req));
        }
        if let Some(req) = extract_from_mime(part)? {
            return Ok(Some(req));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use orp_core::{Importance, Intent, KeyPair, Payload, UnsignedRequest};

    fn sample_request(subject: &str, text: &str) -> Request {
        let kp = KeyPair::generate("k1");
        let unsigned = UnsignedRequest::new(
            "alice@example.com",
            "bob@example.com",
            Intent::Reply,
            "Hello",
            Importance::Normal,
            Payload {
                text: text.into(),
                html: None,
                subject: Some(subject.into()),
                action: None,
            },
        );
        kp.sign_request(&unsigned).unwrap()
    }

    #[test]
    fn embed_and_extract_roundtrip() {
        let signed = sample_request("Subject", "Body text");
        let email = embed_in_email(&signed, None).unwrap();
        let extracted = extract_from_email(&email).unwrap().unwrap();
        assert_eq!(extracted.id(), signed.id());
    }

    #[test]
    fn embed_and_extract_non_ascii_subject() {
        let signed = sample_request("Réunion demain — action requise", "Bonjour le monde!");
        let email = embed_in_email(&signed, None).unwrap();
        assert!(email.contains("Subject:"));
        let extracted = extract_from_email(&email).unwrap().unwrap();
        assert_eq!(extracted.id(), signed.id());
    }

    #[test]
    fn embed_boundary_like_string_in_body() {
        let marker = "orp_boundary_marker_should_not_break";
        let body = format!("Line with {marker} inside the plain text body.");
        let signed = sample_request("Test", &body);
        let email = embed_in_email(&signed, None).unwrap();
        assert!(email.contains(&body));
        let extracted = extract_from_email(&email).unwrap().unwrap();
        assert_eq!(extracted.body.payload.text, body);
    }
}
