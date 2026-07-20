//! Encrypted rehydration map: token → original PII text.
//!
//! Wire format: `XPII\x01` magic + 16-byte salt + 12-byte nonce + 16-byte GCM
//! tag + ciphertext. Key derivation is scrypt(passphrase, salt, N=2^14,
//! r=8, p=1) → 32 bytes.

use std::collections::HashMap;

use aes_gcm::Aes256Gcm;
use aes_gcm::aead::{AeadInOut, Generate, KeyInit, Nonce, Tag};
use scrypt::Params as ScryptParams;
use zeroize::Zeroizing;

use crate::{Result, XbergError};

/// Token → original PII text.
#[cfg_attr(alef, alef(skip))] // binding surface arrives with the engine/wasm integration ~keep
pub type RehydrationMap = HashMap<String, String>;

const MAGIC: &[u8; 5] = b"XPII\x01";
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const TAG_LEN: usize = 16;
const KEY_LEN: usize = 32;
/// Must stay in sync with the TypeScript implementation of this wire
/// format, which uses Node's `crypto.scryptSync` defaults
/// (N = 2^14, r = 8, p = 1); the `decrypts_map_produced_by_typescript`
/// test pins the compatibility.
/// Changing this value breaks wire-format compatibility with existing TS maps.
const SCRYPT_LOG_N: u8 = 14;
const SCRYPT_R: u32 = 8;
const SCRYPT_P: u32 = 1;

fn derive_key(passphrase: &str, salt: &[u8]) -> Result<Zeroizing<[u8; KEY_LEN]>> {
    // scrypt 0.12's `scrypt()` derives exactly `output.len()` bytes and ignores any
    // length baked into `Params`, so the output length is fixed by the KEY_LEN-sized
    // `key` buffer below; `Params::new` no longer takes a length argument. ~keep
    let params = ScryptParams::new(SCRYPT_LOG_N, SCRYPT_R, SCRYPT_P)
        .map_err(|e| XbergError::validation(format!("invalid scrypt parameters: {e}")))?;
    let mut key = Zeroizing::new([0u8; KEY_LEN]);
    scrypt::scrypt(passphrase.as_bytes(), salt, &params, &mut *key)
        .map_err(|e| XbergError::validation(format!("scrypt key derivation failed: {e}")))?;
    Ok(key)
}

/// Encrypt `map` with `passphrase`. Returns `XPII\x01` + salt(16) + nonce(12) + tag(16) + ciphertext.
#[cfg_attr(alef, alef(skip))]
pub fn encrypt_map(map: &RehydrationMap, passphrase: &str) -> Result<Vec<u8>> {
    let plaintext = serde_json::to_vec(map)
        .map_err(|e| XbergError::validation(format!("failed to serialize rehydration map: {e}")))?;

    let salt: [u8; SALT_LEN] = <[u8; SALT_LEN]>::try_generate()
        .map_err(|e| XbergError::validation(format!("failed to generate random salt: {e}")))?;
    let nonce_bytes: [u8; NONCE_LEN] = <[u8; NONCE_LEN]>::try_generate()
        .map_err(|e| XbergError::validation(format!("failed to generate random nonce: {e}")))?;

    let key_bytes = derive_key(passphrase, &salt)?;
    let cipher = Aes256Gcm::new_from_slice(&*key_bytes)
        .map_err(|e| XbergError::validation(format!("invalid AES-256 key: {e}")))?;
    let nonce = Nonce::<Aes256Gcm>::try_from(&nonce_bytes[..])
        .map_err(|e| XbergError::validation(format!("invalid AES-256-GCM nonce: {e}")))?;

    let mut buffer = plaintext;
    let tag = cipher
        .encrypt_inout_detached(&nonce, b"", buffer.as_mut_slice().into())
        .map_err(|e| XbergError::validation(format!("AES-256-GCM encryption failed: {e}")))?;

    let mut out = Vec::with_capacity(MAGIC.len() + SALT_LEN + NONCE_LEN + TAG_LEN + buffer.len());
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&salt);
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(tag.as_slice());
    out.extend_from_slice(&buffer);
    Ok(out)
}

/// Decrypt a blob from [`encrypt_map`].
#[cfg_attr(alef, alef(skip))]
pub fn decrypt_map(blob: &[u8], passphrase: &str) -> Result<RehydrationMap> {
    let min_len = MAGIC.len() + SALT_LEN + NONCE_LEN + TAG_LEN;
    if blob.len() < min_len || &blob[..MAGIC.len()] != MAGIC {
        return Err(XbergError::validation(
            "rehydration blob is too short or missing the XPII magic header",
        ));
    }

    let mut offset = MAGIC.len();
    let salt = &blob[offset..offset + SALT_LEN];
    offset += SALT_LEN;
    let nonce_bytes = &blob[offset..offset + NONCE_LEN];
    offset += NONCE_LEN;
    let tag = &blob[offset..offset + TAG_LEN];
    offset += TAG_LEN;
    let ciphertext = &blob[offset..];

    let key_bytes = derive_key(passphrase, salt)?;
    let cipher = Aes256Gcm::new_from_slice(&*key_bytes)
        .map_err(|e| XbergError::validation(format!("invalid AES-256 key: {e}")))?;
    let nonce = Nonce::<Aes256Gcm>::try_from(nonce_bytes)
        .map_err(|e| XbergError::validation(format!("invalid AES-256-GCM nonce: {e}")))?;
    let tag =
        Tag::<Aes256Gcm>::try_from(tag).map_err(|e| XbergError::validation(format!("invalid AES-256-GCM tag: {e}")))?;

    // Zeroizing: after decryption the buffer holds the plaintext PII map;
    // wipe it when it drops rather than leaving it in freed memory. ~keep
    let mut buffer = Zeroizing::new(ciphertext.to_vec());
    cipher
        .decrypt_inout_detached(&nonce, b"", buffer.as_mut_slice().into(), &tag)
        .map_err(|_| XbergError::validation("failed to decrypt rehydration map: wrong passphrase or corrupted data"))?;

    serde_json::from_slice(&buffer)
        .map_err(|e| XbergError::validation(format!("failed to deserialize rehydration map: {e}")))
}

/// Parse the PII category out of a redaction token's bracket contents.
///
/// Tokens are structured as `"[CATEGORY_N]"` (e.g. `"[EMAIL_1]"`,
/// `"[PERSON_2]"`). Returns `None` if `token` doesn't follow that shape:
/// missing brackets, no trailing `_<N>` suffix, or an empty category.
fn category_from_token(token: &str) -> Option<&str> {
    let inner = token.strip_prefix('[')?.strip_suffix(']')?;
    let underscore_idx = inner.rfind('_')?;
    let (category, rest) = inner.split_at(underscore_idx);
    let suffix = &rest[1..]; // skip the underscore itself
    if category.is_empty() || suffix.is_empty() || !suffix.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some(category)
}

/// One vault match, in either direction of lookup.
#[cfg_attr(alef, alef(skip))] // binding surface arrives with the engine/wasm integration ~keep
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SubjectMatch {
    pub token: String,
    pub original: String,
    /// Category parsed from the token's bracket contents (e.g. `"EMAIL"`
    /// from `"[EMAIL_1]"`), or `None` if the token doesn't follow the
    /// `"[CATEGORY_N]"` convention.
    pub category: Option<String>,
}

/// Return `true` if `query` identifies `token`/`original`: exact match on
/// `token` (tokens are structured like `"[EMAIL_1]"`), Unicode
/// case-insensitive substring match on `original`, matching the TypeScript
/// implementation's `toLowerCase()` semantics.
fn subject_matches(token: &str, original: &str, query: &str, query_lower: &str) -> bool {
    // An empty query is a substring of every original value, so without this
    // guard `find_subject`/`forget_subject` would treat "" as "match
    // everything"; for `forget_subject`, that means a blank erase query
    // wipes the whole rehydration map instead of matching nothing. ~keep
    if query.is_empty() {
        return false;
    }
    token == query || original.to_lowercase().contains(query_lower)
}

/// Search a decrypted map for `query`, matching either the token or the
/// original value (case-insensitive substring match on `original`; exact
/// match on `token`, since tokens are structured like `"[EMAIL_1]"`).
///
/// Results are sorted by token for deterministic output.
#[cfg_attr(alef, alef(skip))]
pub fn find_subject(map: &RehydrationMap, query: &str) -> Vec<SubjectMatch> {
    let query_lower = query.to_lowercase();
    let mut matches: Vec<SubjectMatch> = map
        .iter()
        .filter(|(token, original)| subject_matches(token, original, query, &query_lower))
        .map(|(token, original)| SubjectMatch {
            token: token.clone(),
            original: original.clone(),
            category: category_from_token(token).map(str::to_string),
        })
        .collect();
    matches.sort_by(|a, b| a.token.cmp(&b.token));
    matches
}

/// Remove every mapping whose token or original value matches `query`.
/// Returns the removed entries (the caller re-encrypts and persists the
/// resulting map; this function does not touch disk).
///
/// Idempotent: calling this again with the same `query` after the matching
/// entries have already been removed returns an empty `Vec`.
#[cfg_attr(alef, alef(skip))]
pub fn forget_subject(map: &mut RehydrationMap, query: &str) -> Vec<SubjectMatch> {
    let query_lower = query.to_lowercase();
    let tokens_to_remove: Vec<String> = map
        .iter()
        .filter(|(token, original)| subject_matches(token, original, query, &query_lower))
        .map(|(token, _)| token.clone())
        .collect();

    let mut removed: Vec<SubjectMatch> = tokens_to_remove
        .into_iter()
        .filter_map(|token| {
            map.remove(&token).map(|original| SubjectMatch {
                category: category_from_token(&token).map(str::to_string),
                token,
                original,
            })
        })
        .collect();
    removed.sort_by(|a, b| a.token.cmp(&b.token));
    removed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_encrypt_decrypt() {
        let mut map = HashMap::new();
        map.insert("[EMAIL_1]".to_string(), "alice@example.com".to_string());
        map.insert("[PERSON_1]".to_string(), "Alice Smith".to_string());
        let encrypted = encrypt_map(&map, "correct horse battery staple").expect("encrypt");
        let decrypted = decrypt_map(&encrypted, "correct horse battery staple").expect("decrypt");
        assert_eq!(decrypted, map);
    }

    #[test]
    fn wrong_passphrase_fails_to_decrypt() {
        let mut map = HashMap::new();
        map.insert("[EMAIL_1]".to_string(), "alice@example.com".to_string());
        let encrypted = encrypt_map(&map, "correct passphrase").expect("encrypt");
        let err = decrypt_map(&encrypted, "wrong passphrase").expect_err("must fail");
        assert!(err.to_string().to_ascii_lowercase().contains("decrypt"));
    }

    #[test]
    fn each_encryption_uses_a_fresh_salt_and_nonce() {
        let mut map = HashMap::new();
        map.insert("[EMAIL_1]".to_string(), "alice@example.com".to_string());
        let a = encrypt_map(&map, "same passphrase").expect("encrypt a");
        let b = encrypt_map(&map, "same passphrase").expect("encrypt b");
        assert_ne!(a, b);
    }

    #[test]
    fn magic_bytes_are_present() {
        let map = HashMap::new();
        let encrypted = encrypt_map(&map, "x").expect("encrypt empty map");
        assert_eq!(&encrypted[..5], b"XPII\x01");
    }

    #[test]
    fn decrypts_map_produced_by_typescript() {
        let hex = include_str!("../testdata/ts_map_fixture.hex");
        let bytes = hex_decode(hex.trim());
        let map = decrypt_map(&bytes, "test-passphrase").expect("Rust decrypts TS-encrypted map");
        assert_eq!(map.get("[EMAIL_1]").map(String::as_str), Some("a@b.com"));
        assert_eq!(map.get("[PERSON_1]").map(String::as_str), Some("Jane Doe"));
    }

    fn sample_map() -> RehydrationMap {
        let mut map = HashMap::new();
        map.insert("[PERSON_1]".to_string(), "Alice Johnson".to_string());
        map.insert("[PERSON_2]".to_string(), "Bob Smith".to_string());
        map.insert("[EMAIL_1]".to_string(), "alice@example.com".to_string());
        map
    }

    #[test]
    fn find_subject_matches_by_original_value_substring() {
        let map = sample_map();
        // "johnson" only overlaps "Alice Johnson" (case-insensitive substring),
        // not "alice@example.com", unlike "alice", which would match both. ~keep
        let matches = find_subject(&map, "johnson");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].token, "[PERSON_1]");
        assert_eq!(matches[0].original, "Alice Johnson");
        assert_eq!(matches[0].category.as_deref(), Some("PERSON"));
    }

    #[test]
    fn find_subject_matches_by_exact_token() {
        let map = sample_map();
        let matches = find_subject(&map, "[EMAIL_1]");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].token, "[EMAIL_1]");
        assert_eq!(matches[0].original, "alice@example.com");
        assert_eq!(matches[0].category.as_deref(), Some("EMAIL"));

        // A substring of the token (not an exact match) must not match. ~keep
        assert!(find_subject(&map, "EMAIL_1").is_empty());
    }

    #[test]
    fn find_subject_returns_empty_for_no_match() {
        let map = sample_map();
        assert!(find_subject(&map, "nonexistent subject").is_empty());
    }

    #[test]
    fn find_subject_matches_unicode_case_insensitively() {
        // Parity with the TypeScript implementation's toLowerCase(): a
        // lowercase query must match an uppercase non-ASCII original. ~keep
        let mut map = RehydrationMap::new();
        map.insert("[PERSON_1]".to_string(), "\u{d6}ZLEM Y\u{131}lmaz".to_string());
        let matches = find_subject(&map, "\u{f6}zlem");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].token, "[PERSON_1]");
    }

    #[test]
    fn find_subject_rejects_empty_query_instead_of_matching_everything() {
        let map = sample_map();
        assert!(find_subject(&map, "").is_empty());
    }

    #[test]
    fn forget_subject_rejects_empty_query_instead_of_wiping_the_map() {
        let mut map = sample_map();
        let original_len = map.len();
        let removed = forget_subject(&mut map, "");
        assert!(removed.is_empty());
        assert_eq!(map.len(), original_len);
    }

    #[test]
    fn forget_subject_removes_matching_entries_and_returns_them() {
        let mut map = sample_map();
        let removed = forget_subject(&mut map, "johnson");

        assert_eq!(removed.len(), 1);
        assert_eq!(removed[0].token, "[PERSON_1]");
        assert_eq!(removed[0].original, "Alice Johnson");

        assert!(!map.contains_key("[PERSON_1]"));
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn forget_subject_is_idempotent_on_repeated_calls() {
        let mut map = sample_map();
        let first = forget_subject(&mut map, "johnson");
        assert_eq!(first.len(), 1);

        let second = forget_subject(&mut map, "johnson");
        assert!(second.is_empty());
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn forget_then_reencrypt_round_trips() {
        let mut map = sample_map();
        let removed = forget_subject(&mut map, "[PERSON_2]");
        assert_eq!(removed.len(), 1);
        assert_eq!(removed[0].original, "Bob Smith");

        let encrypted = encrypt_map(&map, "gdpr-erasure-passphrase").expect("re-encrypt");
        let decrypted = decrypt_map(&encrypted, "gdpr-erasure-passphrase").expect("decrypt");

        assert_eq!(decrypted.len(), 2);
        assert!(!decrypted.contains_key("[PERSON_2]"));
        assert_eq!(decrypted.get("[PERSON_1]").map(String::as_str), Some("Alice Johnson"));
        assert_eq!(
            decrypted.get("[EMAIL_1]").map(String::as_str),
            Some("alice@example.com")
        );
    }

    fn hex_decode(s: &str) -> Vec<u8> {
        assert!(
            s.len().is_multiple_of(2),
            "hex input must have even length, got {}",
            s.len()
        );
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("valid hex"))
            .collect()
    }
}
