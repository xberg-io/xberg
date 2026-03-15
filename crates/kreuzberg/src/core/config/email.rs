//! Email extraction configuration.

use serde::{Deserialize, Serialize};

/// Configuration for email extraction.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct EmailConfig {
    /// Windows codepage number to use when an MSG file contains no codepage property.
    /// Defaults to `None`, which falls back to windows-1252.
    ///
    /// If an unrecognized or invalid codepage number is supplied (including 0),
    /// the behavior silently falls back to windows-1252 — the same as when the
    /// MSG file itself contains an unrecognized codepage. No error or warning is
    /// emitted. Users should verify output when supplying unusual values.
    ///
    /// Common values:
    /// - 1250: Central European (Polish, Czech, Hungarian, etc.)
    /// - 1251: Cyrillic (Russian, Ukrainian, Bulgarian, etc.)
    /// - 1252: Western European (default)
    /// - 1253: Greek
    /// - 1254: Turkish
    /// - 1255: Hebrew
    /// - 1256: Arabic
    /// - 932:  Japanese (Shift-JIS)
    /// - 936:  Simplified Chinese (GBK)
    pub msg_fallback_codepage: Option<u32>,
}

impl Default for EmailConfig {
    fn default() -> Self {
        Self {
            msg_fallback_codepage: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_config_default() {
        let config = EmailConfig::default();
        assert!(config.msg_fallback_codepage.is_none());
    }

    #[test]
    fn test_email_config_serde_roundtrip() {
        let json = r#"{"msg_fallback_codepage": 1251}"#;
        let config: EmailConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.msg_fallback_codepage, Some(1251));

        let serialized = serde_json::to_string(&config).unwrap();
        let roundtripped: EmailConfig = serde_json::from_str(&serialized).unwrap();
        assert_eq!(roundtripped.msg_fallback_codepage, Some(1251));
    }

    #[test]
    fn test_email_config_serde_default_omitted() {
        let json = r#"{}"#;
        let config: EmailConfig = serde_json::from_str(json).unwrap();
        assert!(config.msg_fallback_codepage.is_none());
    }
}
