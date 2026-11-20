use std::fmt;

/// Error type for HWP parsing.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug)]
pub enum HwpError {
    /// The file does not match the HWP 5.0 format.
    InvalidFormat(String),
    /// The HWP version or a feature is not supported (e.g. password-encrypted docs).
    UnsupportedVersion(String),
    /// An underlying I/O error occurred.
    Io(std::io::Error),
    /// A CFB compound-file error (stream not found, corrupt container, etc.).
    Cfb(String),
    /// Decompression of a zlib/deflate stream failed.
    CompressionError(String),
    /// The binary record stream could not be parsed.
    ParseError(String),
    /// A UTF-16LE string contained invalid data.
    EncodingError(String),
    /// A requested stream was not present in the compound file.
    NotFound(String),
    /// A configured security limit (decompressed size) was exceeded.
    Security(crate::extractors::security::SecurityError),
}

impl fmt::Display for HwpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HwpError::InvalidFormat(msg) => write!(f, "Invalid file format: {msg}"),
            HwpError::UnsupportedVersion(msg) => write!(f, "Unsupported version: {msg}"),
            HwpError::Io(e) => write!(f, "IO error: {e}"),
            HwpError::Cfb(msg) => write!(f, "CFB error: {msg}"),
            HwpError::CompressionError(msg) => write!(f, "Compression error: {msg}"),
            HwpError::ParseError(msg) => write!(f, "Parse error: {msg}"),
            HwpError::EncodingError(msg) => write!(f, "Encoding error: {msg}"),
            HwpError::NotFound(msg) => write!(f, "Not found: {msg}"),
            HwpError::Security(e) => write!(f, "Security limit exceeded: {e}"),
        }
    }
}

impl std::error::Error for HwpError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            HwpError::Io(e) => Some(e),
            HwpError::Security(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for HwpError {
    fn from(e: std::io::Error) -> Self {
        HwpError::Io(e)
    }
}

/// Convenience `Result` alias for HWP parser operations.
pub type Result<T> = std::result::Result<T, HwpError>;
