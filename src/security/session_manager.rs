use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use hmac::{Hmac, Mac};
use rand::Rng;
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Session manager that generates and validates HMAC-signed tokens.
///
/// Tokens bind a function name and call ID together with an expiry time,
/// preventing replay attacks and cross-function/cross-call misuse.
#[derive(Clone)]
pub struct SessionManager {
    secret: Vec<u8>,
    token_expiry_secs: u64,
}

impl SessionManager {
    /// Create a new session manager with a random 32-byte secret.
    pub fn new(token_expiry_secs: u64) -> Self {
        let mut rng = rand::thread_rng();
        let secret: Vec<u8> = (0..32).map(|_| rng.r#gen()).collect();
        SessionManager {
            secret,
            token_expiry_secs,
        }
    }

    /// Create a new session manager with the default expiry (3600 seconds).
    pub fn with_defaults() -> Self {
        Self::new(3600)
    }

    /// Get the configured token expiry in seconds.
    pub fn token_expiry_secs(&self) -> u64 {
        self.token_expiry_secs
    }

    /// Create or confirm a session, returning the call ID.
    pub fn create_session(&self, call_id: Option<&str>) -> String {
        match call_id {
            Some(id) => id.to_string(),
            None => self.generate_uuid(),
        }
    }

    /// Generate an HMAC-SHA256 signed token for a given function and call.
    ///
    /// Token format (before base64url encoding):
    ///   `{call_id}.{function_name}.{expiry}.{nonce}.{hmac_hex}`
    pub fn create_token(&self, function_name: &str, call_id: &str) -> String {
        let expiry = current_time_secs() + self.token_expiry_secs;
        let nonce = hex_encode(&random_bytes(8));

        let message = format!("{}:{}:{}:{}", call_id, function_name, expiry, nonce);
        let signature = self.hmac_hex(&message);

        let payload = format!(
            "{}.{}.{}.{}.{}",
            call_id, function_name, expiry, nonce, signature
        );

        URL_SAFE_NO_PAD.encode(payload.as_bytes())
    }

    /// Validate a token against the expected function name and call ID.
    ///
    /// Uses timing-safe comparison for all security-critical fields.
    pub fn validate_token(&self, function_name: &str, call_id: &str, token: &str) -> bool {
        let decoded = match URL_SAFE_NO_PAD.decode(token) {
            Ok(d) => d,
            Err(_) => return false,
        };

        let decoded_str = match String::from_utf8(decoded) {
            Ok(s) => s,
            Err(_) => return false,
        };

        let parts: Vec<&str> = decoded_str.split('.').collect();
        if parts.len() != 5 {
            return false;
        }

        let token_call_id = parts[0];
        let token_function = parts[1];
        let token_expiry = parts[2];
        let token_nonce = parts[3];
        let token_signature = parts[4];

        // Timing-safe comparison of the function name
        if !constant_time_eq(function_name, token_function) {
            return false;
        }

        // Check token has not expired
        let expiry: u64 = match token_expiry.parse() {
            Ok(e) => e,
            Err(_) => return false,
        };
        if expiry < current_time_secs() {
            return false;
        }

        // Recreate the signature with the extracted nonce and compare
        let message = format!(
            "{}:{}:{}:{}",
            token_call_id, token_function, token_expiry, token_nonce
        );
        let expected_signature = self.hmac_hex(&message);

        if !constant_time_eq(&expected_signature, token_signature) {
            return false;
        }

        // Timing-safe comparison of the call ID
        if !constant_time_eq(call_id, token_call_id) {
            return false;
        }

        true
    }

    // ── Private helpers ──────────────────────────────────────────────────

    fn hmac_hex(&self, message: &str) -> String {
        let mut mac =
            HmacSha256::new_from_slice(&self.secret).expect("HMAC key should be valid");
        mac.update(message.as_bytes());
        let result = mac.finalize().into_bytes();
        hex_encode(&result)
    }

    fn generate_uuid(&self) -> String {
        let mut bytes = random_bytes(16);

        // Set version to 0100 (UUID v4)
        bytes[6] = (bytes[6] & 0x0f) | 0x40;
        // Set variant to 10xx
        bytes[8] = (bytes[8] & 0x3f) | 0x80;

        let hex = hex_encode(&bytes);
        format!(
            "{}-{}-{}-{}-{}",
            &hex[0..8],
            &hex[8..12],
            &hex[12..16],
            &hex[16..20],
            &hex[20..32]
        )
    }
}

/// Timing-safe string comparison using HMAC.
fn constant_time_eq(a: &str, b: &str) -> bool {
    let key = b"signalwire-session-manager-compare";
    let mut mac_a =
        HmacSha256::new_from_slice(key).expect("HMAC key should be valid");
    mac_a.update(a.as_bytes());
    let digest_a = mac_a.finalize().into_bytes();

    let mut mac_b =
        HmacSha256::new_from_slice(key).expect("HMAC key should be valid");
    mac_b.update(b.as_bytes());
    let digest_b = mac_b.finalize().into_bytes();

    digest_a == digest_b
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn random_bytes(count: usize) -> Vec<u8> {
    let mut rng = rand::thread_rng();
    (0..count).map(|_| rng.r#gen()).collect()
}

/// Get current time in seconds since Unix epoch.
fn current_time_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("System time should be after Unix epoch")
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_construction() {
        let sm = SessionManager::new(7200);
        assert_eq!(sm.token_expiry_secs(), 7200);
    }

    #[test]
    fn test_construction_defaults() {
        let sm = SessionManager::with_defaults();
        assert_eq!(sm.token_expiry_secs(), 3600);
    }

    #[test]
    fn test_create_session_with_id() {
        let sm = SessionManager::with_defaults();
        let id = sm.create_session(Some("call-123"));
        assert_eq!(id, "call-123");
    }

    #[test]
    fn test_create_session_without_id() {
        let sm = SessionManager::with_defaults();
        let id = sm.create_session(None);
        // UUID v4 format: 8-4-4-4-12 hex
        assert_eq!(id.len(), 36);
        assert_eq!(id.chars().filter(|&c| c == '-').count(), 4);
    }

    #[test]
    fn test_round_trip() {
        let sm = SessionManager::with_defaults();
        let token = sm.create_token("my_function", "call-123");
        assert!(sm.validate_token("my_function", "call-123", &token));
    }

    #[test]
    fn test_wrong_function_rejected() {
        let sm = SessionManager::with_defaults();
        let token = sm.create_token("my_function", "call-123");
        assert!(!sm.validate_token("wrong_function", "call-123", &token));
    }

    #[test]
    fn test_wrong_call_id_rejected() {
        let sm = SessionManager::with_defaults();
        let token = sm.create_token("my_function", "call-123");
        assert!(!sm.validate_token("my_function", "call-999", &token));
    }

    #[test]
    fn test_expired_token() {
        let sm = SessionManager::new(0); // 0 second expiry
        let token = sm.create_token("func", "call-1");
        // Token expires immediately (at current time + 0)
        // We need to wait for the clock to tick past
        std::thread::sleep(std::time::Duration::from_secs(1));
        assert!(!sm.validate_token("func", "call-1", &token));
    }

    #[test]
    fn test_tampered_token() {
        let sm = SessionManager::with_defaults();
        let token = sm.create_token("my_function", "call-123");
        // Tamper with the token by changing characters
        let tampered = if token.ends_with('A') {
            format!("{}B", &token[..token.len() - 1])
        } else {
            format!("{}A", &token[..token.len() - 1])
        };
        assert!(!sm.validate_token("my_function", "call-123", &tampered));
    }

    #[test]
    fn test_different_secrets() {
        let sm1 = SessionManager::with_defaults();
        let sm2 = SessionManager::with_defaults();
        let token = sm1.create_token("func", "call-1");
        // Different secret should reject
        assert!(!sm2.validate_token("func", "call-1", &token));
    }

    #[test]
    fn test_invalid_base64() {
        let sm = SessionManager::with_defaults();
        assert!(!sm.validate_token("func", "call-1", "not-valid-base64!!!"));
    }

    #[test]
    fn test_wrong_part_count() {
        let sm = SessionManager::with_defaults();
        let bad_payload = URL_SAFE_NO_PAD.encode(b"a.b.c"); // only 3 parts
        assert!(!sm.validate_token("func", "call-1", &bad_payload));
    }

    #[test]
    fn test_invalid_expiry() {
        let sm = SessionManager::with_defaults();
        let bad_payload = URL_SAFE_NO_PAD.encode(b"call-1.func.notanumber.nonce.sig");
        assert!(!sm.validate_token("func", "call-1", &bad_payload));
    }

    #[test]
    fn test_token_uniqueness() {
        let sm = SessionManager::with_defaults();
        let t1 = sm.create_token("func", "call-1");
        let t2 = sm.create_token("func", "call-1");
        // Nonce makes each token unique
        assert_ne!(t1, t2);
        // But both should validate
        assert!(sm.validate_token("func", "call-1", &t1));
        assert!(sm.validate_token("func", "call-1", &t2));
    }

    #[test]
    fn test_clone_preserves_secret() {
        let sm = SessionManager::with_defaults();
        let token = sm.create_token("func", "call-1");
        let sm_clone = sm.clone();
        // Clone should validate tokens from original
        assert!(sm_clone.validate_token("func", "call-1", &token));
    }

    #[test]
    fn test_constant_time_eq() {
        assert!(constant_time_eq("hello", "hello"));
        assert!(!constant_time_eq("hello", "world"));
        assert!(!constant_time_eq("hello", "hell"));
        assert!(constant_time_eq("", ""));
    }

    #[test]
    fn test_uuid_format() {
        let sm = SessionManager::with_defaults();
        let uuid = sm.generate_uuid();
        let parts: Vec<&str> = uuid.split('-').collect();
        assert_eq!(parts.len(), 5);
        assert_eq!(parts[0].len(), 8);
        assert_eq!(parts[1].len(), 4);
        assert_eq!(parts[2].len(), 4);
        assert_eq!(parts[3].len(), 4);
        assert_eq!(parts[4].len(), 12);
        // Version nibble is 4
        assert!(parts[2].starts_with('4'));
    }
}
