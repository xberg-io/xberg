//! PDF encryption support.
//!
//! This module implements PDF encryption/decryption according to the PDF specification
//! (ISO 32000-1:2008, Section 7.6). It supports:
//!
//! - RC4 encryption (40-bit and 128-bit) for PDF 1.4-1.5
//! - AES encryption (128-bit and 256-bit) for PDF 1.6+
//! - Standard Security Handler (password validation, permissions)
//!
//! # Encryption Algorithms
//!
//! ## RC4 (PDF 1.4-1.5)
//! - RC4-40: 40-bit key length (weak, legacy)
//! - RC4-128: 128-bit key length
//!
//! ## AES (PDF 1.6+)
//! - AES-128: 128-bit key length with CBC mode
//! - AES-256: 256-bit key length with CBC mode (PDF 2.0)
//!
//! # Security Considerations
//!
//! - RC4-40 is cryptographically weak and should only be used for legacy documents
//! - Password validation uses constant-time comparison to prevent timing attacks
//! - Key derivation follows PDF specification algorithms (using MD5 or SHA-256)
//!
//! # References
//!
//! - PDF Spec Section 7.6: Encryption
//! - PDF Spec Section 7.6.3: Standard Security Handler
//! - PDF Spec Section 7.6.5: Algorithm 2 (Key Derivation)

use crate::error::{Error, Result};
use crate::object::Object;

mod aes;
mod algorithms;
mod certificate;
mod handler;
pub mod permissions;
// `pub(crate)` so the `crypto::RustCryptoProvider::SymmetricCipher`
// impl in `src/crypto/rust_provider.rs` can call
// `rc4::rc4_crypt_impl` (the cipher-only entry point that does NOT
// re-route through the active provider, breaking the
// `provider.rc4() → rc4_crypt → provider.rc4()` cycle). RC4 is
// required by PDF Standard Security Handler R≤4 (ISO 32000-1 §7.6.3). ~keep
pub(crate) mod rc4;
mod write_handler;

pub use algorithms::{compute_encryption_key, compute_owner_password_hash, compute_user_password_hash};
pub use certificate::{
    CertEncryptDict, CertSubFilter, CertificateEncryption, CertificateEncryptionHandler, KeyTransportAlgorithm,
    RecipientInfo, RecipientPermissions,
};
pub use handler::EncryptionHandler;
pub use permissions::PdfPermissions;
pub use write_handler::EncryptionWriteHandler;

/// A fresh MD5 hasher for the ISO 32000-1 §7.6.3 Standard-Security-
/// Handler password key-derivation, obtained through the active
/// [`crate::crypto`] provider (#230 Phase C).
///
/// Routing the *primitive* through the provider — not just the
/// operation gate in [`handler::EncryptionHandler::new`] /
/// [`write_handler`] — lets a `strict`/`fips-strict`
/// `SecurityPolicy` (or a FIPS provider) deny weak-crypto key
/// derivation at the boundary it actually happens, closing the
/// "operation-gated but primitive-ungated" gap (feature-230 §3.5 risk
/// table). Under the default `compat` policy the returned hasher is
/// the same `md5::Md5`, so every existing encrypted PDF still decrypts
/// and newly written ones are **byte-for-byte unchanged**.
///
/// The *non-security* opaque MD5 uses — the file identifier
/// ([`generate_file_id`]) and the embedded-file `/CheckSum`
/// (`writer::embedded_files`) — are deliberately **not** routed here:
/// they are not security decisions, and `AlgorithmUse` has no
/// non-security variant, so gating them would make a strict policy
/// refuse to write otherwise-compliant AES-256 documents. They remain
/// direct `md5` calls (feature-230 §2.2).
///
/// # Errors
/// Maps the provider/policy denial
/// ([`crate::crypto::Error::AlgorithmNotPermitted`]) to a clear
/// [`Error::InvalidPdf`], consistent with the existing R≤4 / legacy
/// rejection messages in this module.
pub(crate) fn md5_kdf_hasher() -> Result<Box<dyn crate::crypto::Hasher>> {
    crate::crypto::active()
        .hasher(crate::crypto::HashAlgorithm::Md5)
        .map_err(|e| {
            Error::InvalidPdf(format!(
                "MD5 key derivation is not permitted by the active crypto \
                 policy/provider (ISO 32000-1 §7.6.3 Standard Security \
                 Handler, R≤4): {e}"
            ))
        })
}

/// Encryption algorithm used in the PDF.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Algorithm {
    /// No encryption
    None,
    /// RC4 with 40-bit key (PDF 1.4, V=1, R=2)
    RC4_40,
    /// RC4 with 128-bit key (PDF 1.5, V=2, R=3)
    Rc4_128,
    /// AES with 128-bit key in CBC mode (PDF 1.6, V=4, R=4)
    Aes128,
    /// AES with 256-bit key in CBC mode (PDF 2.0, V=5, R=5/6)
    Aes256,
}

impl Algorithm {
    /// Get the key length in bytes for this algorithm.
    pub fn key_length(&self) -> usize {
        match self {
            Algorithm::None => 0,
            Algorithm::RC4_40 => 5,
            Algorithm::Rc4_128 => 16,
            Algorithm::Aes128 => 16,
            Algorithm::Aes256 => 32,
        }
    }

    /// Check if this is an AES algorithm.
    pub fn is_aes(&self) -> bool {
        matches!(self, Algorithm::Aes128 | Algorithm::Aes256)
    }

    /// Check if this is an RC4 algorithm.
    pub fn is_rc4(&self) -> bool {
        matches!(self, Algorithm::RC4_40 | Algorithm::Rc4_128)
    }
}

/// PDF encryption dictionary (/Encrypt entry in trailer).
///
/// PDF Spec: Section 7.6.1 - General
#[derive(Debug, Clone)]
pub struct EncryptDict {
    /// Filter name (should be "Standard")
    pub filter: String,
    /// SubFilter name (optional, for public-key security)
    pub sub_filter: Option<String>,
    /// Algorithm version (V): 1=RC4-40, 2=RC4-128, 4=AES-128, 5=AES-256
    pub version: u32,
    /// Key length in bits (Length): 40-128 for RC4, 128/256 for AES
    pub length: Option<u32>,
    /// Revision number (R): 2, 3, 4, 5, or 6
    pub revision: u32,
    /// Owner password hash (O): 32 or 48 bytes
    pub owner_password: Vec<u8>,
    /// User password hash (U): 32 or 48 bytes
    pub user_password: Vec<u8>,
    /// User permissions (P): 32-bit integer
    pub permissions: i32,
    /// Encrypt metadata flag (EncryptMetadata): true by default
    pub encrypt_metadata: bool,
    /// Additional encryption parameters (for V=5/R=6)
    pub owner_encryption: Option<Vec<u8>>,
    /// User encryption key (UE entry, for V=5/R=6)
    pub user_encryption: Option<Vec<u8>>,
    /// Encrypted permissions (Perms entry, for V=5/R=6)
    pub perms: Option<Vec<u8>>,
    /// Stream crypt filter method (CFM from /CF dictionary, for V=4).
    /// "V2" = RC4-128, "AESV2" = AES-128. None means not specified (defaults to AES-128).
    pub stream_crypt_method: Option<String>,
    /// String crypt filter method (CFM of /StrF, for V=4/5). `"Identity"` and `"None"` mean strings are
    /// stored unencrypted; None means not specified (follows the stream algorithm).
    pub string_crypt_method: Option<String>,
}

impl EncryptDict {
    /// Parse an encryption dictionary from a PDF object.
    ///
    /// PDF Spec: Section 7.6.1 - General
    pub fn from_object(obj: &Object) -> Result<Self> {
        let dict = obj
            .as_dict()
            .ok_or_else(|| Error::InvalidPdf("Encrypt entry is not a dictionary".to_string()))?;

        let filter = dict
            .get("Filter")
            .and_then(|o| o.as_name())
            .ok_or_else(|| Error::InvalidPdf("Encrypt dictionary missing /Filter".to_string()))?
            .to_string();

        let version = dict
            .get("V")
            .and_then(|o| match o {
                Object::Integer(i) => Some(*i as u32),
                _ => None,
            })
            .ok_or_else(|| Error::InvalidPdf("Encrypt dictionary missing /V".to_string()))?;

        let revision = dict
            .get("R")
            .and_then(|o| match o {
                Object::Integer(i) => Some(*i as u32),
                _ => None,
            })
            .ok_or_else(|| Error::InvalidPdf("Encrypt dictionary missing /R".to_string()))?;

        let owner_password = dict
            .get("O")
            .and_then(|o| o.as_string())
            .ok_or_else(|| Error::InvalidPdf("Encrypt dictionary missing /O".to_string()))?
            .to_vec();

        let user_password = dict
            .get("U")
            .and_then(|o| o.as_string())
            .ok_or_else(|| Error::InvalidPdf("Encrypt dictionary missing /U".to_string()))?
            .to_vec();

        let permissions = dict
            .get("P")
            .and_then(|o| match o {
                Object::Integer(i) => Some(*i as i32),
                _ => None,
            })
            .ok_or_else(|| Error::InvalidPdf("Encrypt dictionary missing /P".to_string()))?;

        let sub_filter = dict.get("SubFilter").and_then(|o| o.as_name()).map(|s| s.to_string());

        let length = dict.get("Length").and_then(|o| match o {
            Object::Integer(i) => Some(*i as u32),
            _ => None,
        });

        let encrypt_metadata = dict
            .get("EncryptMetadata")
            .and_then(|o| match o {
                Object::Boolean(b) => Some(*b),
                _ => None,
            })
            .unwrap_or(true);

        let owner_encryption = dict.get("OE").and_then(|o| o.as_string()).map(|s| s.to_vec());

        let user_encryption = dict.get("UE").and_then(|o| o.as_string()).map(|s| s.to_vec());

        let perms = dict.get("Perms").and_then(|o| o.as_string()).map(|s| s.to_vec());

        // V=4/5: /StmF and /StrF each name a crypt filter whose /CFM selects the cipher (PDF Spec Table 25):
        // "V2" = RC4-128, "AESV2" = AES-128, "AESV3" = AES-256, "None" = unencrypted. A name of /Identity
        // means no encryption, and an absent or null name defaults to it (ISO 32000-2 Table 21),
        // independently of the other entry. ~keep
        let crypt_filter_method = |key: &str| -> Option<String> {
            if version != 4 && version != 5 {
                return None;
            }
            let filter_name = match dict.get(key) {
                None | Some(Object::Null) => return Some("Identity".to_string()),
                Some(entry) => entry.as_name()?,
            };
            if filter_name == "Identity" {
                return Some("Identity".to_string());
            }
            dict.get("CF")
                .and_then(|cf| cf.as_dict())
                .and_then(|cf_dict| cf_dict.get(filter_name))
                .and_then(|filter_obj| filter_obj.as_dict())
                .and_then(|filter_dict| filter_dict.get("CFM"))
                .and_then(|cfm| cfm.as_name())
                .map(|s| s.to_string())
        };
        let stream_crypt_method = crypt_filter_method("StmF");
        let string_crypt_method = crypt_filter_method("StrF");

        Ok(EncryptDict {
            filter,
            sub_filter,
            version,
            length,
            revision,
            owner_password,
            user_password,
            permissions,
            encrypt_metadata,
            owner_encryption,
            user_encryption,
            perms,
            stream_crypt_method,
            string_crypt_method,
        })
    }

    /// Determine the encryption algorithm from V and R values.
    ///
    /// PDF Spec: Table 20 - Encryption dictionary entries
    pub fn algorithm(&self) -> Result<Algorithm> {
        self.algorithm_for_method(self.stream_crypt_method.as_deref())
    }

    /// Determine the algorithm that protects strings (the /StrF crypt filter), which may differ from the
    /// stream algorithm.
    pub fn string_algorithm(&self) -> Result<Algorithm> {
        match self.string_crypt_method.as_deref() {
            None => self.algorithm(),
            method => self.algorithm_for_method(method),
        }
    }

    fn algorithm_for_method(&self, method: Option<&str>) -> Result<Algorithm> {
        match (self.version, self.revision) {
            (1, 2) => Ok(Algorithm::RC4_40),
            (2, 3) => Ok(Algorithm::Rc4_128),
            (4, _) => {
                // V=4 means crypt-filter-based encryption. The actual algorithm
                // is determined by /CFM in the /CF dictionary (PDF Spec Table 25):
                //   "V2" = RC4-128
                //   "AESV2" = AES-128 ~keep
                match method {
                    Some("V2") => {
                        tracing::debug!(
                            revision = self.revision,
                            cfm = "V2",
                            "resolved crypt filter method: RC4-128"
                        );
                        Ok(Algorithm::Rc4_128)
                    }
                    Some("Identity") | Some("None") => Ok(Algorithm::None),
                    Some("AESV2") | None => {
                        // None: /CF missing or unparseable — default to AES-128
                        // for backward compatibility. ~keep
                        Ok(Algorithm::Aes128)
                    }
                    Some(other) => {
                        tracing::warn!(
                            revision = self.revision,
                            cfm = other,
                            "unknown crypt filter method; falling back to AES-128"
                        );
                        Ok(Algorithm::Aes128)
                    }
                }
            }
            (5, 5) | (5, 6) if matches!(method, Some("Identity") | Some("None")) => Ok(Algorithm::None),
            (5, 5) | (5, 6) => Ok(Algorithm::Aes256),
            // Lenient: V determines algorithm, R may be non-standard ~keep
            (1, r) => {
                tracing::warn!(revision = r, "non-standard encryption V=1; using RC4-40");
                Ok(Algorithm::RC4_40)
            }
            (2, r) => {
                tracing::warn!(revision = r, "non-standard encryption V=2; using RC4-128");
                Ok(Algorithm::Rc4_128)
            }
            _ => Err(Error::Unsupported(format!(
                "Unsupported encryption version V={}, R={}",
                self.version, self.revision
            ))),
        }
    }

    /// Get the effective key length in bytes.
    pub fn key_length_bytes(&self) -> usize {
        if let Some(length) = self.length {
            (length / 8) as usize
        } else {
            match self.version {
                1 => 5,
                2 => 16,
                4 => 16,
                5 => 32,
                _ => 16,
            }
        }
    }

    /// Serialize the encryption dictionary to a PDF Object.
    ///
    /// This creates a dictionary object suitable for the /Encrypt entry in the trailer.
    pub fn to_object(&self) -> Object {
        use std::collections::HashMap;

        let mut dict: HashMap<String, Object> = HashMap::new();

        dict.insert("Filter".to_string(), Object::Name(self.filter.clone()));
        dict.insert("V".to_string(), Object::Integer(self.version as i64));
        dict.insert("R".to_string(), Object::Integer(self.revision as i64));
        dict.insert("O".to_string(), Object::String(self.owner_password.clone()));
        dict.insert("U".to_string(), Object::String(self.user_password.clone()));
        dict.insert("P".to_string(), Object::Integer(self.permissions as i64));

        if let Some(ref sub_filter) = self.sub_filter {
            dict.insert("SubFilter".to_string(), Object::Name(sub_filter.clone()));
        }

        if let Some(length) = self.length {
            dict.insert("Length".to_string(), Object::Integer(length as i64));
        }

        if !self.encrypt_metadata {
            dict.insert("EncryptMetadata".to_string(), Object::Boolean(false));
        }

        if let Some(ref oe) = self.owner_encryption {
            dict.insert("OE".to_string(), Object::String(oe.clone()));
        }

        if let Some(ref ue) = self.user_encryption {
            dict.insert("UE".to_string(), Object::String(ue.clone()));
        }

        if let Some(ref perms) = self.perms {
            dict.insert("Perms".to_string(), Object::String(perms.clone()));
        }

        // For V=4 (crypt-filter-based), add crypt filter entries ~keep
        if self.version == 4 {
            let cfm = self.stream_crypt_method.as_deref().unwrap_or("AESV2").to_string();
            let mut cf_dict: HashMap<String, Object> = HashMap::new();
            let mut std_cf: HashMap<String, Object> = HashMap::new();
            std_cf.insert("CFM".to_string(), Object::Name(cfm));
            std_cf.insert("AuthEvent".to_string(), Object::Name("DocOpen".to_string()));
            std_cf.insert("Length".to_string(), Object::Integer(16));
            cf_dict.insert("StdCF".to_string(), Object::Dictionary(std_cf));
            dict.insert("CF".to_string(), Object::Dictionary(cf_dict));
            dict.insert("StmF".to_string(), Object::Name("StdCF".to_string()));
            dict.insert("StrF".to_string(), Object::Name("StdCF".to_string()));
        }

        // For V=5 (AES-256), add crypt filter entries ~keep
        if self.version == 5 {
            let mut cf_dict: HashMap<String, Object> = HashMap::new();
            let mut std_cf: HashMap<String, Object> = HashMap::new();
            std_cf.insert("CFM".to_string(), Object::Name("AESV3".to_string()));
            std_cf.insert("AuthEvent".to_string(), Object::Name("DocOpen".to_string()));
            std_cf.insert("Length".to_string(), Object::Integer(32));
            cf_dict.insert("StdCF".to_string(), Object::Dictionary(std_cf));
            dict.insert("CF".to_string(), Object::Dictionary(cf_dict));
            dict.insert("StmF".to_string(), Object::Name("StdCF".to_string()));
            dict.insert("StrF".to_string(), Object::Name("StdCF".to_string()));
        }

        Object::Dictionary(dict)
    }
}

/// Builder for creating encryption dictionaries.
///
/// This provides a convenient way to create properly configured encryption
/// for writing encrypted PDFs.
pub struct EncryptDictBuilder {
    algorithm: Algorithm,
    user_password: Vec<u8>,
    owner_password: Vec<u8>,
    permissions: i32,
    encrypt_metadata: bool,
}

impl EncryptDictBuilder {
    /// Create a new builder with the specified algorithm.
    pub fn new(algorithm: Algorithm) -> Self {
        Self {
            algorithm,
            user_password: Vec::new(),
            owner_password: Vec::new(),
            permissions: -1,
            encrypt_metadata: true,
        }
    }

    /// Set the user password (required for opening the document).
    pub fn user_password(mut self, password: &[u8]) -> Self {
        self.user_password = password.to_vec();
        self
    }

    /// Set the owner password (required for full access).
    pub fn owner_password(mut self, password: &[u8]) -> Self {
        self.owner_password = password.to_vec();
        self
    }

    /// Set user permissions (P value).
    pub fn permissions(mut self, permissions: i32) -> Self {
        self.permissions = permissions;
        self
    }

    /// Set whether to encrypt metadata.
    pub fn encrypt_metadata(mut self, encrypt: bool) -> Self {
        self.encrypt_metadata = encrypt;
        self
    }

    /// Build the encryption dictionary.
    ///
    /// This computes all required hashes and returns the complete dictionary.
    ///
    /// # Arguments
    /// * `file_id` - The first element of the PDF file identifier array
    pub fn build(self, file_id: &[u8]) -> Result<EncryptDict> {
        let (version, revision) = match self.algorithm {
            Algorithm::None => (0, 0),
            Algorithm::RC4_40 => (1, 2),
            Algorithm::Rc4_128 => (2, 3),
            Algorithm::Aes128 => (4, 4),
            Algorithm::Aes256 => (5, 6),
        };

        // FIPS gate: the FIPS-validated `AwsLcProvider`
        // refuses MD5 / RC4 entirely, so writing an R≤4 dict under it
        // would produce ciphertext that the same provider can't read
        // back. Reject up front with a clear error rather than letting
        // the deeper RC4 path return AlgorithmNotPermitted. ~keep
        if revision > 0 && revision <= 4 && !crate::crypto::active().is_legacy_allowed() {
            return Err(crate::Error::InvalidPdf(format!(
                "active CryptoProvider '{}' rejects PDF Standard Security R={} \
                 (R≤4 requires MD5 + RC4; FIPS 140-3 forbids both). \
                 Use Algorithm::Aes256 (R=6) or build xberg-native-pdf \
                 without the 'fips' feature.",
                crate::crypto::active().name(),
                revision
            )));
        }

        // Runtime crypto-governance policy (#230). No-op under the
        // default `compat` policy (byte-stable); `strict`/`fips-strict`
        // forbid *writing* legacy R≤4 (it fundamentally needs the MD5
        // KDF, ISO 32000-1 §7.6.3) while still allowing legacy reads. ~keep
        if revision > 0 && revision <= 4 {
            crate::crypto::record_algorithm_use(crate::crypto::AlgorithmId::HashMd5);
        }
        if revision > 0
            && revision <= 4
            && !crate::crypto::active_policy()
                .allows(crate::crypto::AlgorithmId::HashMd5, crate::crypto::AlgorithmUse::Write)
        {
            return Err(crate::Error::InvalidPdf(format!(
                "active crypto SecurityPolicy (mode={}) forbids writing PDF \
                 Standard Security R={} (R≤4 requires MD5; denied for write). \
                 Use Algorithm::Aes256 (R=6) or set a 'compat' crypto policy.",
                crate::crypto::active_policy().mode().token(),
                revision
            )));
        }

        let key_length = self.algorithm.key_length();

        // Use owner password if provided, otherwise use user password ~keep
        let owner_pass = if self.owner_password.is_empty() {
            self.user_password.clone()
        } else {
            self.owner_password.clone()
        };

        if revision >= 5 {
            // AES-256 (R6): file key is random; U/UE and O/OE are computed per
            // PDF 2.0 Algorithm 8 and 9 using the actual passwords. ~keep
            let (user_hash, user_encryption, file_key) =
                algorithms::compute_u_and_ue(&self.user_password, key_length, revision)?;
            let (owner_hash, owner_encryption) =
                algorithms::compute_o_and_oe(&owner_pass, &self.user_password, &file_key, &user_hash, revision)?;
            return Ok(EncryptDict {
                filter: "Standard".to_string(),
                sub_filter: None,
                version,
                length: Some((key_length * 8) as u32),
                revision,
                owner_password: owner_hash,
                user_password: user_hash,
                permissions: self.permissions,
                encrypt_metadata: self.encrypt_metadata,
                owner_encryption: Some(owner_encryption),
                user_encryption: Some(user_encryption),
                perms: None,
                stream_crypt_method: None,
                string_crypt_method: None,
            });
        }

        let owner_hash =
            algorithms::compute_owner_password_hash(&owner_pass, &self.user_password, revision, key_length)?;

        let encryption_key = algorithms::compute_encryption_key(
            &self.user_password,
            &owner_hash,
            self.permissions,
            file_id,
            revision,
            key_length,
            self.encrypt_metadata,
        )?;

        let user_hash = algorithms::compute_user_password_hash(&encryption_key, file_id, revision)?;

        Ok(EncryptDict {
            filter: "Standard".to_string(),
            sub_filter: None,
            version,
            length: Some((key_length * 8) as u32),
            revision,
            owner_password: owner_hash,
            user_password: user_hash,
            permissions: self.permissions,
            encrypt_metadata: self.encrypt_metadata,
            owner_encryption: None,
            user_encryption: None,
            perms: None,
            stream_crypt_method: None,
            string_crypt_method: None,
        })
    }
}

/// PDF encryption permissions (P field).
///
/// PDF Spec: Table 22 - User access permissions
#[derive(Debug, Clone, Copy)]
pub struct Permissions {
    bits: i32,
}

impl Permissions {
    /// Create permissions from the P field value.
    pub fn from_bits(bits: i32) -> Self {
        Self { bits }
    }

    /// Decoded view per [`PdfPermissions`]. The bit decoding lives
    /// in one place; this method-style API delegates so the two
    /// decoders cannot drift apart.
    #[inline]
    fn decoded(&self) -> PdfPermissions {
        PdfPermissions::from_p_flag(self.bits)
    }

    /// Check if printing is allowed.
    pub fn can_print(&self) -> bool {
        self.decoded().print_low_res
    }

    /// Check if modifying the document is allowed.
    pub fn can_modify(&self) -> bool {
        self.decoded().modify
    }

    /// Check if copying text/graphics is allowed.
    pub fn can_copy(&self) -> bool {
        self.decoded().copy
    }

    /// Check if adding/modifying annotations is allowed.
    pub fn can_annotate(&self) -> bool {
        self.decoded().annotate
    }

    /// Check if filling form fields is allowed (R>=3).
    pub fn can_fill_forms(&self) -> bool {
        self.decoded().fill_forms
    }

    /// Check if content extraction for accessibility is allowed (R>=3).
    pub fn can_extract_accessibility(&self) -> bool {
        self.decoded().accessibility
    }

    /// Check if assembling the document is allowed (R>=3).
    pub fn can_assemble(&self) -> bool {
        self.decoded().assemble
    }

    /// Check if high-quality printing is allowed (R>=3).
    pub fn can_print_high_quality(&self) -> bool {
        self.decoded().print_high_res
    }
}

/// Generate a unique file ID for the PDF.
///
/// PDF Spec: Section 14.4 - File Identifiers
///
/// The file identifier array contains two strings:
/// - First string: A permanent identifier based on file contents at creation
/// - Second string: A changing identifier updated each time the file is saved
///
/// This function generates both strings as the same value (for new PDFs).
/// For incremental updates, the first ID should be preserved.
///
/// # Returns
///
/// A tuple of (permanent_id, changing_id) as 16-byte vectors
pub fn generate_file_id() -> (Vec<u8>, Vec<u8>) {
    let uuid = uuid::Uuid::new_v4();
    let uuid_bytes = uuid.as_bytes();

    // SystemTime::now() is not available on wasm32-unknown-unknown.
    // The UUID v4 already provides sufficient entropy for a unique opaque
    // file identifier (ISO 32000-1 §14.4), so we skip the time contribution
    // on that target. ~keep
    #[cfg(not(target_arch = "wasm32"))]
    let now_bytes: Option<[u8; 16]> = {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        Some(now.as_nanos().to_le_bytes())
    };
    #[cfg(target_arch = "wasm32")]
    let now_bytes: Option<[u8; 16]> = None;

    // PDF spec (ISO 32000-1 §14.4) specifies MD5 for the file identifier.
    // #230 Phase C: intentionally a *direct* MD5 — the file identifier
    // is a non-security opaque tag, not a key-derivation. It is NOT
    // routed through `md5_kdf_hasher` (see that fn's docs): doing so
    // would let a strict policy refuse to write an otherwise-compliant
    // AES-256 document, and `AlgorithmUse` has no non-security variant. ~keep
    let id = {
        use md5::{Digest, Md5};
        let mut h = Md5::new();
        h.update(uuid_bytes);
        if let Some(b) = now_bytes {
            h.update(b);
        }
        h.finalize().to_vec()
    };

    // For new PDFs, both IDs are the same ~keep
    (id.clone(), id)
}

#[cfg(test)]
mod tests;
