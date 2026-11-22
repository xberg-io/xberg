//! Encryption handler for PDF documents.
//!
//! This module provides the main interface for handling encrypted PDFs,
//! including password authentication and stream/string decryption.

use super::algorithms;
use super::{Algorithm, EncryptDict, Permissions};
use crate::error::{Error, Result};
use crate::object::Object;

/// Main encryption handler for PDF documents.
///
/// This struct manages the encryption state and provides methods for
/// authenticating passwords and decrypting content.
#[derive(Clone)]
pub struct EncryptionHandler {
    /// Encryption dictionary
    dict: EncryptDict,
    /// Derived encryption key (set after successful authentication)
    encryption_key: Option<Vec<u8>>,
    /// File identifier (first element of /ID array)
    file_id: Vec<u8>,
    /// Encryption algorithm for streams (/StmF)
    algorithm: Algorithm,
    /// Encryption algorithm for strings (/StrF)
    string_algorithm: Algorithm,
}

impl std::fmt::Debug for EncryptionHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EncryptionHandler")
            .field("algorithm", &self.algorithm)
            .field("string_algorithm", &self.string_algorithm)
            .field("version", &self.dict.version)
            .field("revision", &self.dict.revision)
            .field("authenticated", &self.encryption_key.is_some())
            .finish_non_exhaustive()
    }
}

impl Drop for EncryptionHandler {
    fn drop(&mut self) {
        if let Some(key) = self.encryption_key.as_mut() {
            zeroize::Zeroize::zeroize(key);
        }
    }
}

impl EncryptionHandler {
    /// Create a new encryption handler from an encryption dictionary.
    ///
    /// # Arguments
    ///
    /// * `encrypt_obj` - The /Encrypt dictionary object from the PDF trailer
    /// * `file_id` - The first element of the /ID array from the PDF trailer
    ///
    /// # Returns
    ///
    /// An encryption handler ready for password authentication
    pub fn new(encrypt_obj: &Object, file_id: Vec<u8>) -> Result<Self> {
        let dict = EncryptDict::from_object(encrypt_obj)?;
        let algorithm = dict.algorithm()?;
        let string_algorithm = dict.string_algorithm()?;

        tracing::info!(
            algorithm = ?algorithm,
            version = dict.version,
            revision = dict.revision,
            "PDF is encrypted"
        );

        // FIPS / sovereign-compliance gate. PDF Standard Security
        // R≤4 (ISO 32000-1 §7.6.3) hard-requires MD5 + RC4 (R=2/3)
        // or MD5 + AES-128 (R=4) — MD5 is forbidden under FIPS
        // 140-3 regardless of which symmetric cipher follows. This is
        // a provider-level check, not a compile-time one: the default
        // 'rust-crypto' provider always allows legacy algorithms, but
        // a FIPS-validated CryptoProvider installed via
        // `crate::crypto::set_provider` can reject them, and we reject
        // early here so callers get a clear error rather than a panic
        // deep inside the cipher path. Issue #236. ~keep
        if !crate::crypto::active().is_legacy_allowed() && dict.revision <= 4 {
            return Err(Error::InvalidPdf(format!(
                "active CryptoProvider '{}' rejects PDF Standard Security R={} \
                 (R≤4 requires MD5; FIPS 140-3 forbids MD5). \
                 Re-encrypt the document at R=6 (AES-256), or do not install \
                 a legacy-rejecting CryptoProvider at process startup if \
                 opening R≤4 documents is required.",
                crate::crypto::active().name(),
                dict.revision
            )));
        }

        // Runtime crypto-governance policy (#230). Orthogonal to the
        // provider: under the default `compat` policy this is a no-op
        // (byte-stable); a `strict`/`fips-strict` policy can forbid
        // reading legacy R≤4 (which fundamentally needs the MD5 KDF,
        // ISO 32000-1 §7.6.3 Algorithm 2). ~keep
        if dict.revision <= 4 {
            crate::crypto::record_algorithm_use(crate::crypto::AlgorithmId::HashMd5);
        }
        if dict.revision <= 4
            && !crate::crypto::active_policy()
                .allows(crate::crypto::AlgorithmId::HashMd5, crate::crypto::AlgorithmUse::Read)
        {
            return Err(Error::InvalidPdf(format!(
                "active crypto SecurityPolicy (mode={}) forbids opening PDF \
                 Standard Security R={} (R≤4 requires MD5 key derivation, \
                 denied for read). Set a 'compat' crypto policy to read \
                 legacy-encrypted documents.",
                crate::crypto::active_policy().mode().token(),
                dict.revision
            )));
        }

        Ok(Self {
            dict,
            encryption_key: None,
            file_id,
            algorithm,
            string_algorithm,
        })
    }

    /// Authenticate with a password.
    ///
    /// This attempts to authenticate with the given password as either
    /// a user password or owner password. If successful, the encryption
    /// key is derived and stored for future decryption operations.
    ///
    /// # Arguments
    ///
    /// * `password` - The password to authenticate (empty string for no password)
    ///
    /// # Returns
    ///
    /// `Ok(true)` if authentication succeeded, `Ok(false)` if it failed,
    /// or an error if the encryption is unsupported.
    pub fn authenticate(&mut self, password: &[u8]) -> Result<bool> {
        if let Some(key) = algorithms::authenticate_user_password(
            password,
            &self.dict.user_password,
            &self.dict.owner_password,
            self.dict.permissions,
            &self.file_id,
            self.dict.revision,
            self.dict.key_length_bytes(),
            self.dict.encrypt_metadata,
            self.dict.user_encryption.as_deref(),
        ) {
            self.encryption_key = Some(key);
            tracing::info!("authenticated with user password");
            self.warn_on_perms_mismatch();
            return Ok(true);
        }

        // Try authenticating as owner password (Algorithm 7 for R≤4, Algorithm 12 for R≥5)
        // ~keep
        if let Some(key) = algorithms::authenticate_owner_password(
            password,
            &self.dict.user_password,
            &self.dict.owner_password,
            self.dict.permissions,
            &self.file_id,
            self.dict.revision,
            self.dict.key_length_bytes(),
            self.dict.encrypt_metadata,
            self.dict.owner_encryption.as_deref(),
        )? {
            self.encryption_key = Some(key);
            tracing::info!("authenticated with owner password");
            self.warn_on_perms_mismatch();
            return Ok(true);
        }

        tracing::warn!("password authentication failed");
        Ok(false)
    }

    fn warn_on_perms_mismatch(&self) {
        if self.dict.revision < 5 {
            return;
        }
        let (Some(key), Some(perms)) = (self.encryption_key.as_deref(), self.dict.perms.as_deref()) else {
            return;
        };
        if !algorithms::verify_perms(key, perms, self.dict.permissions, self.dict.encrypt_metadata) {
            tracing::warn!(
                "encryption /Perms does not match /P or /EncryptMetadata; permissions may have been altered"
            );
        }
    }

    /// Check if the handler has been authenticated.
    pub fn is_authenticated(&self) -> bool {
        self.encryption_key.is_some()
    }

    /// Get the encryption key (if authenticated).
    pub fn encryption_key(&self) -> Option<&[u8]> {
        self.encryption_key.as_deref()
    }

    /// Get the permissions.
    pub fn permissions(&self) -> Permissions {
        Permissions::from_bits(self.dict.permissions)
    }

    /// Get the raw `/P` permission flag integer per PDF spec §7.6.3.2
    /// Table 22. Used by [`crate::document::PdfDocument::permissions`]
    /// to construct the [`crate::encryption::permissions::PdfPermissions`]
    /// struct.
    ///
    /// PDF spec uses two's-complement int32 with bits 13-32 reserved.
    pub fn raw_permissions(&self) -> i32 {
        self.dict.permissions
    }

    /// Get the encryption algorithm.
    pub fn algorithm(&self) -> Algorithm {
        self.algorithm
    }

    /// Decrypt a stream using the encryption key.
    ///
    /// PDF Spec: Section 7.6.2 - General Encryption Algorithm
    ///
    /// # Arguments
    ///
    /// * `data` - The encrypted stream data
    /// * `obj_num` - Object number
    /// * `gen_num` - Generation number
    ///
    /// # Returns
    ///
    /// The decrypted stream data, or an error if decryption fails
    pub fn decrypt_stream(&self, data: &[u8], obj_num: u32, gen_num: u32) -> Result<Vec<u8>> {
        self.decrypt_with(self.algorithm, data, obj_num, gen_num)
    }

    fn decrypt_with(&self, algorithm: Algorithm, data: &[u8], obj_num: u32, gen_num: u32) -> Result<Vec<u8>> {
        let key = self
            .encryption_key
            .as_ref()
            .ok_or_else(|| Error::InvalidPdf("Not authenticated".to_string()))?;

        match algorithm {
            Algorithm::None => return Ok(data.to_vec()),
            Algorithm::Aes256 => {
                // AES-256 uses the file encryption key directly (no per-object key derivation)
                // per ISO 32000-2:2020 Section 7.6.3.3 ~keep
                if key.len() < 32 {
                    return Err(Error::InvalidPdf(format!(
                        "AES-256 file key too short: {} bytes (need 32)",
                        key.len()
                    )));
                }
                if data.len() < 16 {
                    return Err(Error::InvalidPdf("AES encrypted data too short".to_string()));
                }
                let (iv, ciphertext) = data.split_at(16);
                return super::aes::aes256_decrypt(&key[..32], iv, ciphertext)
                    .map_err(|e| Error::InvalidPdf(format!("AES-256 decryption failed: {}", e)));
            }
            _ => {}
        }

        // Per-object key derivation via MD5 — required for RC4 and AES-128 (R≤4). ~keep
        let obj_key = Self::compute_object_key(algorithm, key, obj_num, gen_num)?;

        // Decrypt based on algorithm (AES-256 and None already handled above) ~keep
        match algorithm {
            Algorithm::RC4_40 | Algorithm::Rc4_128 => super::rc4::rc4_crypt(&obj_key, data),
            Algorithm::Aes128 => {
                if obj_key.len() < 16 {
                    return Err(Error::InvalidPdf(format!(
                        "AES-128 object key too short: {} bytes (need 16)",
                        obj_key.len()
                    )));
                }
                if data.len() < 16 {
                    return Err(Error::InvalidPdf("AES encrypted data too short".to_string()));
                }
                let (iv, ciphertext) = data.split_at(16);
                super::aes::aes128_decrypt(&obj_key[..16], iv, ciphertext)
                    .map_err(|e| Error::InvalidPdf(format!("AES-128 decryption failed: {}", e)))
            }
            Algorithm::None | Algorithm::Aes256 => unreachable!("handled above"),
        }
    }

    /// Decrypt a string using the encryption key.
    ///
    /// # Arguments
    ///
    /// * `data` - The encrypted string data
    /// * `obj_num` - Object number
    /// * `gen_num` - Generation number
    ///
    /// # Returns
    ///
    /// The decrypted string data
    pub fn decrypt_string(&self, data: &[u8], obj_num: u32, gen_num: u32) -> Result<Vec<u8>> {
        self.decrypt_with(self.string_algorithm, data, obj_num, gen_num)
    }

    /// Compute the object-specific encryption key.
    ///
    /// PDF Spec: Algorithm 1 - Encryption key algorithm
    ///
    /// # Arguments
    ///
    /// * `base_key` - The base encryption key
    /// * `obj_num` - Object number
    /// * `gen_num` - Generation number
    ///
    /// # Returns
    ///
    /// The object-specific key
    fn compute_object_key(algorithm: Algorithm, base_key: &[u8], obj_num: u32, gen_num: u32) -> Result<Vec<u8>> {
        // #230 Phase C: route the Algorithm-1 per-object-key MD5
        // through the governed provider (byte-identical under the
        // default `compat` policy; a strict/FIPS policy denies it
        // at the primitive, not just the operation gate). ~keep
        let mut hasher = super::md5_kdf_hasher()?;

        // Step a: Extend key with object/generation number ~keep
        hasher.update(base_key);
        hasher.update(&obj_num.to_le_bytes()[..3]);
        hasher.update(&gen_num.to_le_bytes()[..2]);

        // Step b: For AES, add "sAlT" string ~keep
        if algorithm.is_aes() {
            hasher.update(b"sAlT");
        }

        // Step c: MD5 hash ~keep
        let hash = hasher.finalize();

        // Step d: Key is first (n + 5) bytes, max 16 ~keep
        let key_len = (base_key.len() + 5).min(16);
        Ok(hash[..key_len].to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Full integration tests would require creating encrypted PDFs
    // or using real encrypted PDF samples. These are basic unit tests. ~keep

    #[test]
    fn test_compute_object_key_rc4() {
        let base_key = &[0x01, 0x23, 0x45, 0x67, 0x89];
        let handler = create_test_handler(Algorithm::RC4_40);

        let obj_key = EncryptionHandler::compute_object_key(handler.algorithm, base_key, 1, 0).unwrap();

        // Key should be (5 + 5).min(16) = 10 bytes ~keep
        assert_eq!(obj_key.len(), 10);
    }

    #[test]
    fn test_compute_object_key_aes() {
        let base_key = &[0x01; 16];
        let handler = create_test_handler(Algorithm::Aes128);

        let obj_key = EncryptionHandler::compute_object_key(handler.algorithm, base_key, 1, 0).unwrap();

        // Key should be (16 + 5).min(16) = 16 bytes ~keep
        assert_eq!(obj_key.len(), 16);
    }

    #[test]
    fn test_decrypt_stream_aes128_with_short_key() {
        // RC4-40 produces a 10-byte key; AES needs 16. Should error, not panic. ~keep
        let mut handler = create_test_handler(Algorithm::Aes128);
        handler.encryption_key = Some(vec![0x01; 5]);
        let data = vec![0u8; 32];
        let result = handler.decrypt_stream(&data, 1, 0);
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("AES-128 object key too short"), "got: {}", err_msg);
    }

    #[test]
    fn test_decrypt_stream_aes256_with_short_key() {
        let mut handler = create_test_handler(Algorithm::Aes256);
        handler.encryption_key = Some(vec![0x01; 16]);
        let data = vec![0u8; 32];
        let result = handler.decrypt_stream(&data, 1, 0);
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("AES-256 file key too short"), "got: {}", err_msg);
    }

    #[test]
    fn test_decrypt_stream_aes256_uses_key_directly() {
        use crate::encryption::aes;

        let mut handler = create_test_handler(Algorithm::Aes256);
        let file_key = vec![0x42u8; 32];
        handler.encryption_key = Some(file_key.clone());

        let iv = [0u8; 16];
        let plaintext = b"Hello, AES-256!!";
        let encrypted = aes::aes256_encrypt(&file_key, &iv, plaintext).unwrap();

        // Prepend IV to ciphertext (as PDF spec requires) ~keep
        let mut data = iv.to_vec();
        data.extend_from_slice(&encrypted);

        let result = handler.decrypt_stream(&data, 1, 0).unwrap();
        assert_eq!(&result, plaintext);
    }

    fn create_test_handler(algorithm: Algorithm) -> EncryptionHandler {
        EncryptionHandler {
            dict: EncryptDict {
                filter: "Standard".to_string(),
                sub_filter: None,
                version: match algorithm {
                    Algorithm::RC4_40 => 1,
                    Algorithm::Rc4_128 => 2,
                    Algorithm::Aes128 => 4,
                    Algorithm::Aes256 => 5,
                    Algorithm::None => 0,
                },
                length: Some(match algorithm {
                    Algorithm::RC4_40 => 40,
                    Algorithm::Rc4_128 => 128,
                    Algorithm::Aes128 => 128,
                    Algorithm::Aes256 => 256,
                    Algorithm::None => 0,
                }),
                revision: match algorithm {
                    Algorithm::RC4_40 => 2,
                    Algorithm::Rc4_128 => 3,
                    Algorithm::Aes128 => 4,
                    Algorithm::Aes256 => 5,
                    Algorithm::None => 0,
                },
                owner_password: vec![0; 32],
                user_password: vec![0; 32],
                permissions: -1,
                encrypt_metadata: true,
                owner_encryption: None,
                user_encryption: None,
                perms: None,
                stream_crypt_method: None,
                string_crypt_method: None,
            },
            encryption_key: Some(vec![0x01; 16]),
            file_id: b"test_id".to_vec(),
            algorithm,
            string_algorithm: algorithm,
        }
    }
}
