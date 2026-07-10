//! Encrypted rehydration map: token → original PII text.
//!
//! Wire format: `XPII\x01` magic + 16-byte salt + 12-byte nonce + 16-byte GCM
//! tag + ciphertext. Key derivation is scrypt(passphrase, salt, N=2^14,
//! r=8, p=1) → 32 bytes.

use std::collections::HashMap;

use aes_gcm::aead::rand_core::RngCore;
use aes_gcm::aead::{AeadInPlace, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use scrypt::Params as ScryptParams;

use crate::{Result, XbergError};

/// Token → original PII text.
pub type RehydrationMap = HashMap<String, String>;

const MAGIC: &[u8; 5] = b"XPII\x01";
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const TAG_LEN: usize = 16;
const KEY_LEN: usize = 32;
/// Must stay in sync with the shipped TypeScript rehydration module
/// (`mcp-server/src/redaction/rehydration.ts`), which uses Node's
/// `crypto.scryptSync` defaults (N = 2^14, r = 8, p = 1).
/// Changing this value breaks wire-format compatibility with existing TS maps.
const SCRYPT_LOG_N: u8 = 14;
const SCRYPT_R: u32 = 8;
const SCRYPT_P: u32 = 1;

fn derive_key(passphrase: &str, salt: &[u8]) -> Result<[u8; KEY_LEN]> {
    let params = ScryptParams::new(SCRYPT_LOG_N, SCRYPT_R, SCRYPT_P, KEY_LEN)
        .map_err(|e| XbergError::validation(format!("invalid scrypt parameters: {e}")))?;
    let mut key = [0u8; KEY_LEN];
    scrypt::scrypt(passphrase.as_bytes(), salt, &params, &mut key)
        .map_err(|e| XbergError::validation(format!("scrypt key derivation failed: {e}")))?;
    Ok(key)
}

/// Encrypt `map` with `passphrase`. Returns `XPII\x01` + salt(16) + nonce(12) + tag(16) + ciphertext.
pub fn encrypt_map(map: &RehydrationMap, passphrase: &str) -> Result<Vec<u8>> {
    let plaintext = serde_json::to_vec(map)
        .map_err(|e| XbergError::validation(format!("failed to serialize rehydration map: {e}")))?;

    let mut salt = [0u8; SALT_LEN];
    OsRng.fill_bytes(&mut salt);
    let mut nonce_bytes = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce_bytes);

    let key_bytes = derive_key(passphrase, &salt)?;
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key_bytes));
    let nonce = Nonce::from_slice(&nonce_bytes);

    let mut buffer = plaintext;
    let tag = cipher
        .encrypt_in_place_detached(nonce, b"", &mut buffer)
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
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key_bytes));
    let nonce = Nonce::from_slice(nonce_bytes);

    let mut buffer = ciphertext.to_vec();
    cipher
        .decrypt_in_place_detached(nonce, b"", &mut buffer, tag.into())
        .map_err(|_| {
            XbergError::validation("failed to decrypt rehydration map — wrong passphrase or corrupted data")
        })?;

    serde_json::from_slice(&buffer)
        .map_err(|e| XbergError::validation(format!("failed to deserialize rehydration map: {e}")))
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
