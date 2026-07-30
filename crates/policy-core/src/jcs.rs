// RFC 8785 JSON Canonicalization Scheme
// Produces byte-identical output for identical semantic inputs.
// SEMANTICS.md §9: JCS(x) with SHA-256 digest over label || 0x00 || JCS(normalized(x))

use serde_json::Value;
use sha2::{Digest, Sha256};

/// Serialize value to canonical JSON (RFC 8785)
pub fn jcs(value: &Value) -> Vec<u8> {
    serde_jcs::to_vec(value).expect("JCS serialization failed")
}

/// Compute digest: H("label", x) = SHA256(UTF8(label) || 0x00 || JCS(normalize(x)))
pub fn digest(label: &str, value: &Value) -> String {
    let mut hasher = Sha256::new();
    hasher.update(label.as_bytes());
    hasher.update([0u8]); // 0x00 separator
    hasher.update(jcs(value));
    let hash = hasher.finalize();
    // Convert to hex string
    hash.iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_jcs_empty_object() {
        let val = json!({});
        let result = jcs(&val);
        assert_eq!(result, b"{}");
    }

    #[test]
    fn test_jcs_simple_object() {
        let val = json!({"a": 1, "b": 2});
        let result = jcs(&val);
        // JCS sorts keys
        assert_eq!(result, b"{\"a\":1,\"b\":2}");
    }

    #[test]
    fn test_jcs_number_canonical() {
        let val = json!({"x": 1.5});
        let result = jcs(&val);
        assert_eq!(result, b"{\"x\":1.5}");
    }

    #[test]
    fn test_digest_format() {
        let val = json!({"test": "value"});
        let d = digest("test-label", &val);
        // Should be hex string of SHA-256 (64 chars)
        assert_eq!(d.len(), 64);
        assert!(d.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
