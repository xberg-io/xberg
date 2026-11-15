//! Certificate-based encryption for PDFs.
//!
//! This module implements public-key encryption using X.509 certificates
//! according to PDF specification Section 7.6.4 (Public-Key Security Handlers).
//!
//! ## Overview
//!
//! Certificate encryption differs from password-based encryption:
//! - Uses `/Filter /Adobe.PubSec` instead of `/Standard`
//! - Uses PKCS#7 enveloped data for key transport
//! - Supports multiple recipients (each can decrypt with their private key)
//! - The encryption key is encrypted with each recipient's public key
//!
//! ## SubFilter Types
//!
//! - `adbe.pkcs7.s4`: PKCS#7 enveloped data with 128-bit key
//! - `adbe.pkcs7.s5`: PKCS#7 enveloped data with 256-bit key
//!
//! ## Status: not implemented
//!
//! [`CertificateEncryption::build`] always returns `Err`. A
//! spec-conformant `/Recipients` entry (ISO 32000-2 §7.6.4.4, Table 25)
//! must be a PKCS#7 EnvelopedData structure wrapping the file
//! encryption key under the recipient's public key — building that
//! needs an RSA/X.509 crypto stack this crate does not currently pull
//! in. Use password-based encryption ([`super::EncryptDictBuilder`])
//! instead. ~keep

use super::Algorithm;
use crate::error::{Error, Result};
use crate::object::Object;
use std::collections::HashMap;

/// Sub-filter types for certificate encryption.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CertSubFilter {
    /// PKCS#7 with 128-bit key (adbe.pkcs7.s4)
    Pkcs7S4,
    /// PKCS#7 with 256-bit key (adbe.pkcs7.s5)
    #[default]
    Pkcs7S5,
}

impl CertSubFilter {
    /// Get the PDF name for this sub-filter.
    pub fn as_pdf_name(&self) -> &'static str {
        match self {
            CertSubFilter::Pkcs7S4 => "adbe.pkcs7.s4",
            CertSubFilter::Pkcs7S5 => "adbe.pkcs7.s5",
        }
    }

    /// Parse from PDF name.
    pub fn from_pdf_name(name: &str) -> Option<Self> {
        match name {
            "adbe.pkcs7.s4" => Some(CertSubFilter::Pkcs7S4),
            "adbe.pkcs7.s5" => Some(CertSubFilter::Pkcs7S5),
            _ => None,
        }
    }

    /// Get the encryption algorithm for this sub-filter.
    pub fn algorithm(&self) -> Algorithm {
        match self {
            CertSubFilter::Pkcs7S4 => Algorithm::Aes128,
            CertSubFilter::Pkcs7S5 => Algorithm::Aes256,
        }
    }

    /// Get the key length in bytes.
    pub fn key_length(&self) -> usize {
        match self {
            CertSubFilter::Pkcs7S4 => 16,
            CertSubFilter::Pkcs7S5 => 32,
        }
    }
}

/// Recipient information for certificate encryption.
#[derive(Debug, Clone)]
pub struct RecipientInfo {
    /// DER-encoded X.509 certificate
    pub certificate: Vec<u8>,
    /// Permissions for this recipient
    pub permissions: RecipientPermissions,
    /// Key transport algorithm (for PKCS#7)
    pub key_transport: KeyTransportAlgorithm,
}

impl RecipientInfo {
    /// Create a new recipient from a DER-encoded certificate.
    pub fn from_der(certificate: &[u8]) -> Result<Self> {
        if certificate.is_empty() {
            return Err(Error::InvalidPdf("Empty certificate data".to_string()));
        }

        // Basic validation: X.509 certificates start with SEQUENCE tag ~keep
        if certificate[0] != 0x30 {
            return Err(Error::InvalidPdf("Invalid certificate format".to_string()));
        }

        Ok(Self {
            certificate: certificate.to_vec(),
            permissions: RecipientPermissions::default(),
            key_transport: KeyTransportAlgorithm::default(),
        })
    }

    /// Create a recipient with specific permissions.
    pub fn with_permissions(mut self, permissions: RecipientPermissions) -> Self {
        self.permissions = permissions;
        self
    }

    /// Set the key transport algorithm.
    pub fn with_key_transport(mut self, algo: KeyTransportAlgorithm) -> Self {
        self.key_transport = algo;
        self
    }
}

/// Permissions for certificate-encrypted documents.
///
/// Different from password-based permissions, these are per-recipient.
#[derive(Debug, Clone, Copy)]
pub struct RecipientPermissions {
    /// Allow printing
    pub print: bool,
    /// Allow modifying the document
    pub modify: bool,
    /// Allow copying text and graphics
    pub copy: bool,
    /// Allow adding/modifying annotations
    pub annotate: bool,
    /// Allow filling form fields
    pub fill_forms: bool,
    /// Allow content extraction for accessibility
    pub accessibility: bool,
    /// Allow assembling the document
    pub assemble: bool,
    /// Allow high-quality printing
    pub print_high_quality: bool,
}

impl Default for RecipientPermissions {
    fn default() -> Self {
        Self {
            print: true,
            modify: true,
            copy: true,
            annotate: true,
            fill_forms: true,
            accessibility: true,
            assemble: true,
            print_high_quality: true,
        }
    }
}

impl RecipientPermissions {
    /// Create permissions with all access.
    pub fn full_access() -> Self {
        Self::default()
    }

    /// Create read-only permissions.
    pub fn read_only() -> Self {
        Self {
            print: false,
            modify: false,
            copy: false,
            annotate: false,
            fill_forms: false,
            accessibility: true,
            assemble: false,
            print_high_quality: false,
        }
    }

    /// Create print-only permissions.
    pub fn print_only() -> Self {
        Self {
            print: true,
            modify: false,
            copy: false,
            annotate: false,
            fill_forms: false,
            accessibility: true,
            assemble: false,
            print_high_quality: true,
        }
    }

    /// Convert to permission bits (P value).
    pub fn to_bits(&self) -> i32 {
        let mut bits: i32 = 0xFFFFF0C0u32 as i32;
        if self.print {
            bits |= 1 << 2;
        }
        if self.modify {
            bits |= 1 << 3;
        }
        if self.copy {
            bits |= 1 << 4;
        }
        if self.annotate {
            bits |= 1 << 5;
        }
        if self.fill_forms {
            bits |= 1 << 8;
        }
        if self.accessibility {
            bits |= 1 << 9;
        }
        if self.assemble {
            bits |= 1 << 10;
        }
        if self.print_high_quality {
            bits |= 1 << 11;
        }
        bits
    }

    /// Parse from permission bits.
    pub fn from_bits(bits: i32) -> Self {
        Self {
            print: (bits & (1 << 2)) != 0,
            modify: (bits & (1 << 3)) != 0,
            copy: (bits & (1 << 4)) != 0,
            annotate: (bits & (1 << 5)) != 0,
            fill_forms: (bits & (1 << 8)) != 0,
            accessibility: (bits & (1 << 9)) != 0,
            assemble: (bits & (1 << 10)) != 0,
            print_high_quality: (bits & (1 << 11)) != 0,
        }
    }
}

/// Key transport algorithm for encrypting the file key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KeyTransportAlgorithm {
    /// RSA-OAEP (recommended)
    #[default]
    RsaOaep,
    /// RSA PKCS#1 v1.5 (legacy)
    RsaPkcs1v15,
}

/// Certificate-based encryption builder and handler.
#[derive(Debug)]
pub struct CertificateEncryption {
    /// Recipients who can decrypt the document
    recipients: Vec<RecipientInfo>,
    /// Sub-filter (determines key length)
    sub_filter: CertSubFilter,
    /// Whether to encrypt metadata
    encrypt_metadata: bool,
    /// The file encryption key (randomly generated)
    file_key: Vec<u8>,
}

impl CertificateEncryption {
    /// Create a new certificate encryption builder.
    pub fn new() -> Self {
        Self {
            recipients: Vec::new(),
            sub_filter: CertSubFilter::default(),
            encrypt_metadata: true,
            file_key: Vec::new(),
        }
    }

    /// Add a recipient who can decrypt the document.
    pub fn add_recipient(mut self, recipient: RecipientInfo) -> Self {
        self.recipients.push(recipient);
        self
    }

    /// Add multiple recipients.
    pub fn add_recipients(mut self, recipients: impl IntoIterator<Item = RecipientInfo>) -> Self {
        self.recipients.extend(recipients);
        self
    }

    /// Set the sub-filter (determines encryption strength).
    pub fn sub_filter(mut self, sub_filter: CertSubFilter) -> Self {
        self.sub_filter = sub_filter;
        self
    }

    /// Set whether to encrypt metadata.
    pub fn encrypt_metadata(mut self, encrypt: bool) -> Self {
        self.encrypt_metadata = encrypt;
        self
    }

    /// Build the certificate encryption, generating keys.
    ///
    /// This generates a random file encryption key and attempts to
    /// create the PKCS#7 enveloped data for each recipient.
    ///
    /// # Errors
    ///
    /// Always returns `Err` (via [`Self::create_recipient_entries`]):
    /// certificate (public-key) encryption is not implemented. See the
    /// module docs. ~keep
    pub fn build(mut self) -> Result<CertificateEncryptionHandler> {
        if self.recipients.is_empty() {
            return Err(Error::InvalidPdf(
                "At least one recipient is required for certificate encryption".to_string(),
            ));
        }

        let key_length = self.sub_filter.key_length();
        self.file_key = generate_random_key(key_length)?;

        let recipient_entries = self.create_recipient_entries()?;

        Ok(CertificateEncryptionHandler {
            sub_filter: self.sub_filter,
            encrypt_metadata: self.encrypt_metadata,
            file_key: self.file_key,
            recipient_entries,
            algorithm: self.sub_filter.algorithm(),
        })
    }

    /// Create PKCS#7 enveloped data for each recipient.
    fn create_recipient_entries(&self) -> Result<Vec<Vec<u8>>> {
        let mut entries = Vec::new();

        for recipient in &self.recipients {
            let entry = self.create_recipient_entry(recipient)?;
            entries.push(entry);
        }

        Ok(entries)
    }

    /// Create a single recipient entry (PKCS#7 enveloped data).
    ///
    /// This would encrypt the file key with the recipient's public key:
    /// 1. Extract the public key from the X.509 certificate
    /// 2. Encrypt the file key using RSA-OAEP or RSA PKCS#1 v1.5
    /// 3. Build the PKCS#7 EnvelopedData structure
    ///
    /// # Errors
    ///
    /// Always returns `Err` — none of the above is implemented. An
    /// empty placeholder here would let [`Self::build`] succeed while
    /// emitting a `/Recipients` entry that wraps the file key for
    /// nobody: the document is genuinely AES-encrypted with that key,
    /// so the result would be permanently undecryptable by every
    /// recipient, including the certificate holder. That is silent,
    /// irrecoverable data loss, which this crate refuses to produce —
    /// fail loudly instead. Use password-based encryption
    /// ([`super::EncryptDictBuilder`]). ~keep
    fn create_recipient_entry(&self, recipient: &RecipientInfo) -> Result<Vec<u8>> {
        let _ = recipient;
        Err(Error::InvalidPdf(
            "certificate (public-key) encryption is not implemented: wrapping the file \
             encryption key under a recipient's public key (PKCS#7 EnvelopedData, ISO \
             32000-2 §7.6.4.4) is not supported. Use password-based encryption \
             (EncryptDictBuilder) instead."
                .to_string(),
        ))
    }
}

impl Default for CertificateEncryption {
    fn default() -> Self {
        Self::new()
    }
}

/// Handler for certificate-encrypted PDFs.
#[derive(Debug)]
pub struct CertificateEncryptionHandler {
    /// Sub-filter type
    sub_filter: CertSubFilter,
    /// Whether to encrypt metadata
    encrypt_metadata: bool,
    /// The file encryption key
    file_key: Vec<u8>,
    /// PKCS#7 EnvelopedData for each recipient
    recipient_entries: Vec<Vec<u8>>,
    /// Encryption algorithm
    algorithm: Algorithm,
}

impl CertificateEncryptionHandler {
    /// Get the encryption dictionary for the PDF.
    pub fn encrypt_dict(&self) -> CertEncryptDict {
        CertEncryptDict {
            filter: "Adobe.PubSec".to_string(),
            sub_filter: self.sub_filter,
            version: match self.sub_filter {
                CertSubFilter::Pkcs7S4 => 4,
                CertSubFilter::Pkcs7S5 => 5,
            },
            revision: match self.sub_filter {
                CertSubFilter::Pkcs7S4 => 4,
                CertSubFilter::Pkcs7S5 => 6,
            },
            encrypt_metadata: self.encrypt_metadata,
            recipients: self.recipient_entries.clone(),
        }
    }

    /// Get the file encryption key for encrypting content.
    pub fn encryption_key(&self) -> &[u8] {
        &self.file_key
    }

    /// Get the encryption algorithm.
    pub fn algorithm(&self) -> Algorithm {
        self.algorithm
    }

    /// Check if metadata should be encrypted.
    pub fn should_encrypt_metadata(&self) -> bool {
        self.encrypt_metadata
    }

    /// Encrypt a string with object-specific key derivation.
    ///
    /// Uses the same mechanism as password-based encryption.
    pub fn encrypt_string(&self, data: &[u8], obj_num: u32, gen_num: u16) -> Vec<u8> {
        use super::write_handler::EncryptionWriteHandler;

        let handler = EncryptionWriteHandler::from_key(self.file_key.clone(), self.algorithm, self.encrypt_metadata);
        handler.encrypt_string(data, obj_num, gen_num)
    }

    /// Encrypt a stream with object-specific key derivation.
    pub fn encrypt_stream(&self, data: &[u8], obj_num: u32, gen_num: u16) -> Vec<u8> {
        use super::write_handler::EncryptionWriteHandler;

        let handler = EncryptionWriteHandler::from_key(self.file_key.clone(), self.algorithm, self.encrypt_metadata);
        handler.encrypt_stream(data, obj_num, gen_num)
    }
}

/// Certificate encryption dictionary.
#[derive(Debug, Clone)]
pub struct CertEncryptDict {
    /// Filter name (should be "Adobe.PubSec")
    pub filter: String,
    /// Sub-filter type
    pub sub_filter: CertSubFilter,
    /// Algorithm version (V)
    pub version: u32,
    /// Revision number (R)
    pub revision: u32,
    /// Whether to encrypt metadata
    pub encrypt_metadata: bool,
    /// PKCS#7 EnvelopedData for each recipient
    pub recipients: Vec<Vec<u8>>,
}

impl CertEncryptDict {
    /// Convert to a PDF Object.
    pub fn to_object(&self) -> Object {
        let mut dict: HashMap<String, Object> = HashMap::new();

        dict.insert("Filter".to_string(), Object::Name(self.filter.clone()));
        dict.insert(
            "SubFilter".to_string(),
            Object::Name(self.sub_filter.as_pdf_name().to_string()),
        );
        dict.insert("V".to_string(), Object::Integer(self.version as i64));
        dict.insert("R".to_string(), Object::Integer(self.revision as i64));

        if !self.encrypt_metadata {
            dict.insert("EncryptMetadata".to_string(), Object::Boolean(false));
        }

        let recipients_array: Vec<Object> = self.recipients.iter().map(|r| Object::String(r.clone())).collect();
        dict.insert("Recipients".to_string(), Object::Array(recipients_array));

        // Add crypt filter for V=4/5 ~keep
        if self.version >= 4 {
            let cfm = match self.sub_filter {
                CertSubFilter::Pkcs7S4 => "AESV2",
                CertSubFilter::Pkcs7S5 => "AESV3",
            };
            let key_length = match self.sub_filter {
                CertSubFilter::Pkcs7S4 => 16,
                CertSubFilter::Pkcs7S5 => 32,
            };

            let mut cf_dict: HashMap<String, Object> = HashMap::new();
            let mut std_cf: HashMap<String, Object> = HashMap::new();
            std_cf.insert("CFM".to_string(), Object::Name(cfm.to_string()));
            std_cf.insert("AuthEvent".to_string(), Object::Name("DocOpen".to_string()));
            std_cf.insert("Length".to_string(), Object::Integer(key_length));
            cf_dict.insert("DefaultCryptFilter".to_string(), Object::Dictionary(std_cf));
            dict.insert("CF".to_string(), Object::Dictionary(cf_dict));
            dict.insert("StmF".to_string(), Object::Name("DefaultCryptFilter".to_string()));
            dict.insert("StrF".to_string(), Object::Name("DefaultCryptFilter".to_string()));
        }

        Object::Dictionary(dict)
    }
}

/// Generate a cryptographically secure random key.
fn generate_random_key(length: usize) -> Result<Vec<u8>> {
    let mut key = vec![0u8; length];
    crate::crypto::active()
        .random_bytes(&mut key)
        .map_err(|e| crate::Error::InvalidPdf(format!("failed to generate random key: {e}")))?;
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cert_sub_filter() {
        assert_eq!(CertSubFilter::Pkcs7S4.as_pdf_name(), "adbe.pkcs7.s4");
        assert_eq!(CertSubFilter::Pkcs7S5.as_pdf_name(), "adbe.pkcs7.s5");

        assert_eq!(
            CertSubFilter::from_pdf_name("adbe.pkcs7.s4"),
            Some(CertSubFilter::Pkcs7S4)
        );
        assert_eq!(
            CertSubFilter::from_pdf_name("adbe.pkcs7.s5"),
            Some(CertSubFilter::Pkcs7S5)
        );
        assert_eq!(CertSubFilter::from_pdf_name("unknown"), None);
    }

    #[test]
    fn test_recipient_permissions_bits() {
        let perms = RecipientPermissions::full_access();
        let bits = perms.to_bits();

        assert!(bits & (1 << 2) != 0);
        assert!(bits & (1 << 3) != 0);
        assert!(bits & (1 << 4) != 0);

        let restored = RecipientPermissions::from_bits(bits);
        assert!(restored.print);
        assert!(restored.modify);
        assert!(restored.copy);
    }

    #[test]
    fn test_recipient_permissions_read_only() {
        let perms = RecipientPermissions::read_only();
        assert!(!perms.print);
        assert!(!perms.modify);
        assert!(!perms.copy);
        assert!(perms.accessibility);
    }

    #[test]
    fn test_recipient_info_creation() {
        // Valid DER-encoded certificate starts with SEQUENCE (0x30) ~keep
        let mock_cert = vec![0x30, 0x82, 0x01, 0x00, 0x00];
        let recipient = RecipientInfo::from_der(&mock_cert);
        assert!(recipient.is_ok());

        let invalid_cert = vec![0xFF, 0xFF];
        let invalid = RecipientInfo::from_der(&invalid_cert);
        assert!(invalid.is_err());

        let empty = RecipientInfo::from_der(&[]);
        assert!(empty.is_err());
    }

    #[test]
    fn build_fails_loudly_when_recipient_key_wrapping_is_unimplemented() {
        let mock_cert = vec![0x30, 0x82, 0x01, 0x00, 0x00];
        let recipient = RecipientInfo::from_der(&mock_cert).unwrap();

        let encryptor = CertificateEncryption::new()
            .add_recipient(recipient)
            .sub_filter(CertSubFilter::Pkcs7S5)
            .encrypt_metadata(true)
            .build();

        // Certificate encryption cannot wrap the file key for any
        // recipient yet; succeeding here would silently produce an
        // undecryptable PDF, so build() must fail instead. ~keep
        assert!(encryptor.is_err());
    }

    #[test]
    fn test_certificate_encryption_no_recipients() {
        let result = CertificateEncryption::new().build();
        assert!(result.is_err());
    }

    #[test]
    fn test_cert_encrypt_dict_to_object() {
        let dict = CertEncryptDict {
            filter: "Adobe.PubSec".to_string(),
            sub_filter: CertSubFilter::Pkcs7S5,
            version: 5,
            revision: 6,
            encrypt_metadata: true,
            recipients: vec![vec![1, 2, 3]],
        };

        let obj = dict.to_object();
        if let Object::Dictionary(d) = obj {
            assert!(d.contains_key("Filter"));
            assert!(d.contains_key("SubFilter"));
            assert!(d.contains_key("V"));
            assert!(d.contains_key("R"));
            assert!(d.contains_key("Recipients"));
            assert!(d.contains_key("CF"));
        } else {
            panic!("Expected dictionary");
        }
    }

    #[test]
    fn test_generate_random_key() {
        let key16 = generate_random_key(16).unwrap();
        assert_eq!(key16.len(), 16);

        let key32 = generate_random_key(32).unwrap();
        assert_eq!(key32.len(), 32);

        let key1 = generate_random_key(16).unwrap();
        let key2 = generate_random_key(16).unwrap();
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_key_transport_algorithm_default() {
        let algo = KeyTransportAlgorithm::default();
        assert_eq!(algo, KeyTransportAlgorithm::RsaOaep);
    }
}
