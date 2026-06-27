use serde_json::{Map, Value};

use crate::error::OrpError;

/// Canonical JSON serialization (JCS-style: sorted keys, minimal separators).
pub fn to_canonical_bytes(value: &Value) -> Result<Vec<u8>, OrpError> {
    let sorted = sort_value(value);
    serde_json::to_vec(&sorted).map_err(|e| OrpError::Serialization(e.to_string()))
}

fn sort_value(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let mut out = Map::new();
            for k in keys {
                out.insert(k.clone(), sort_value(&map[k]));
            }
            Value::Object(out)
        }
        Value::Array(arr) => Value::Array(arr.iter().map(sort_value).collect()),
        other => other.clone(),
    }
}

/// Build canonical signing payload from a request JSON value (without sig).
pub fn signing_payload(request: &Value) -> Result<Vec<u8>, OrpError> {
    let mut obj = request
        .as_object()
        .ok_or_else(|| OrpError::InvalidRequest("request must be object".into()))?
        .clone();
    obj.remove("sig");
    to_canonical_bytes(&Value::Object(obj))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn canonical_sorts_keys() {
        let v = json!({"b": 2, "a": 1});
        let bytes = to_canonical_bytes(&v).unwrap();
        assert_eq!(bytes, br#"{"a":1,"b":2}"#);
    }
}
