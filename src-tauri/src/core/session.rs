use rand::Rng;

const CODE_CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
const CODE_LENGTH: usize = 6;

/// Generate a cryptographically random 6-character alphanumeric session code
pub fn generate_session_code() -> String {
    let mut rng = rand::thread_rng();
    (0..CODE_LENGTH)
        .map(|_| {
            let idx = rng.gen_range(0..CODE_CHARSET.len());
            CODE_CHARSET[idx] as char
        })
        .collect()
}

/// Normalize a user-entered code to canonical form (Fix #8)
pub fn normalize_code(code: &str) -> String {
    code.trim().to_uppercase()
}

/// Validate that a session code matches the expected format.
/// Accepts both upper and lower case — use normalize_code() before comparing.
pub fn validate_code_format(code: &str) -> bool {
    let upper = code.trim().to_uppercase();
    upper.len() == CODE_LENGTH
        && upper
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_length() {
        let code = generate_session_code();
        assert_eq!(code.len(), CODE_LENGTH);
    }

    #[test]
    fn test_code_format() {
        for _ in 0..100 {
            let code = generate_session_code();
            assert!(validate_code_format(&code));
        }
    }
}
