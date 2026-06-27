//! Email ↔ ORP Request bridge.

pub mod degrade;
pub mod embed;
pub mod extract;
pub mod inference;

pub use degrade::degrade_to_email;
pub use embed::{ORP_MIME_TYPE, ORP_HEADER, embed_in_email};
pub use extract::extract_from_email;
pub use inference::{InferredRequest, InferenceHook};
