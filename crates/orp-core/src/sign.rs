use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::canonical::signing_payload;
use crate::error::OrpError;
use crate::request::{Request, UnsignedRequest};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureBundle {
    pub alg: String,
    pub key_id: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicKeyBundle {
    pub key_id: String,
    pub alg: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct KeyPair {
    pub key_id: String,
    signing_key: SigningKey,
    verifying_key: VerifyingKey,
}

impl KeyPair {
    pub fn generate(key_id: impl Into<String>) -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        Self {
            key_id: key_id.into(),
            signing_key,
            verifying_key,
        }
    }

    pub fn from_seed(key_id: impl Into<String>, seed: &[u8; 32]) -> Self {
        let signing_key = SigningKey::from_bytes(seed);
        let verifying_key = signing_key.verifying_key();
        Self {
            key_id: key_id.into(),
            signing_key,
            verifying_key,
        }
    }

    pub fn public_bundle(&self) -> PublicKeyBundle {
        PublicKeyBundle {
            key_id: self.key_id.clone(),
            alg: "ed25519".into(),
            value: URL_SAFE_NO_PAD.encode(self.verifying_key.as_bytes()),
        }
    }

    pub fn sign_request(&self, req: &UnsignedRequest) -> Result<Request, OrpError> {
        let value = serde_json::to_value(req).map_err(|e| OrpError::Serialization(e.to_string()))?;
        let payload = signing_payload(&value)?;
        let sig = self.signing_key.sign(&payload);
        let bundle = SignatureBundle {
            alg: "ed25519".into(),
            key_id: self.key_id.clone(),
            value: URL_SAFE_NO_PAD.encode(sig.to_bytes()),
        };
        Ok(Request::new(req.clone(), bundle))
    }
}

pub fn verify_request(req: &Request, keys: &[PublicKeyBundle]) -> Result<(), OrpError> {
    let key = keys
        .iter()
        .find(|k| k.key_id == req.sig.key_id)
        .ok_or_else(|| OrpError::UnknownKey(req.sig.key_id.clone()))?;

    if key.alg != "ed25519" || req.sig.alg != "ed25519" {
        return Err(OrpError::BadSignature);
    }

    let pk_bytes = URL_SAFE_NO_PAD
        .decode(&key.value)
        .map_err(|_| OrpError::BadSignature)?;
    let pk_array: [u8; 32] = pk_bytes
        .try_into()
        .map_err(|_| OrpError::BadSignature)?;
    let verifying_key =
        VerifyingKey::from_bytes(&pk_array).map_err(|_| OrpError::BadSignature)?;

    let mut value =
        serde_json::to_value(&req.body).map_err(|e| OrpError::Serialization(e.to_string()))?;
    if let Value::Object(ref mut map) = value {
        map.remove("sig");
    }
    let payload = signing_payload(&value)?;
    let sig_bytes = URL_SAFE_NO_PAD
        .decode(&req.sig.value)
        .map_err(|_| OrpError::BadSignature)?;
    let sig_array: [u8; 64] = sig_bytes
        .try_into()
        .map_err(|_| OrpError::BadSignature)?;
    let signature = Signature::from_bytes(&sig_array);

    verifying_key
        .verify(&payload, &signature)
        .map_err(|_| OrpError::BadSignature)
}
