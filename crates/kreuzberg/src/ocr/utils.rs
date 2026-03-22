/// Minimal supported Tesseract version
pub const MINIMAL_SUPPORTED_TESSERACT_VERSION: u32 = 5;

/// TSV parsing constants
pub const TSV_WORD_LEVEL: u32 = 5;
pub const TSV_MIN_FIELDS: usize = 12;

/// Table formatting constants
pub const MIN_COLUMN_WIDTH: usize = 3;

/// Compute a blake3 hash string from input data.
///
/// Returns a 32-character hex string (128 bits of blake3 output).
pub fn compute_hash(data: &str) -> String {
    crate::cache::blake3_hash_bytes(data.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_hash_deterministic() {
        let input = "test_string";
        let hash1 = compute_hash(input);
        let hash2 = compute_hash(input);
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 32);
    }

    #[test]
    fn test_compute_hash_different_inputs() {
        let hash1 = compute_hash("test1");
        let hash2 = compute_hash("test2");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_compute_hash_empty() {
        let hash = compute_hash("");
        assert_eq!(hash.len(), 32);
    }
}
