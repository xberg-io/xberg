//! URI types for extracted links, references, and resources.
//!
//! Provides a unified representation for all URI-like references found during
//! document extraction: hyperlinks, image references, citations, anchors, and emails.

use serde::{Deserialize, Serialize};

/// A URI extracted from a document.
///
/// Represents any link, reference, or resource pointer found during extraction.
/// The `kind` field classifies the URI semantically, while `label` carries
/// optional human-readable display text.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct Uri {
    /// The URL or path string.
    pub url: String,
    /// Optional display text / label for the link.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Optional page number where the URI was found (1-indexed).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    /// Semantic classification of the URI.
    pub kind: UriKind,
}

/// Semantic classification of an extracted URI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum UriKind {
    /// A clickable hyperlink (web URL, file link).
    Hyperlink,
    /// An image or media resource reference.
    Image,
    /// An internal anchor or cross-reference target.
    Anchor,
    /// A citation or bibliographic reference (DOI, academic ref).
    Citation,
    /// A general reference (e.g. `\ref{}` in LaTeX, `:ref:` in RST).
    Reference,
    /// An email address (`mailto:` link or bare email).
    Email,
}

/// Classify a URL string into the appropriate `UriKind`.
///
/// - `mailto:` → `Email`
/// - `#` prefix → `Anchor`
/// - everything else → `Hyperlink`
pub fn classify_uri(url: &str) -> UriKind {
    if url.starts_with("mailto:") {
        UriKind::Email
    } else if url.starts_with('#') {
        UriKind::Anchor
    } else {
        UriKind::Hyperlink
    }
}

impl Uri {
    /// Create a new hyperlink URI, auto-classifying `mailto:` as Email and `#` as Anchor.
    pub(crate) fn hyperlink(url: impl Into<String>, label: Option<String>) -> Self {
        let url = url.into();
        let kind = classify_uri(&url);
        Self {
            url,
            label,
            page: None,
            kind,
        }
    }

    /// Create a new image URI.
    pub(crate) fn image(url: impl Into<String>, label: Option<String>) -> Self {
        Self {
            url: url.into(),
            label,
            page: None,
            kind: UriKind::Image,
        }
    }

    /// Create a new citation URI (for DOIs, academic references).
    pub(crate) fn citation(url: impl Into<String>, label: Option<String>) -> Self {
        Self {
            url: url.into(),
            label,
            page: None,
            kind: UriKind::Citation,
        }
    }

    /// Create a new anchor/cross-reference URI.
    pub(crate) fn anchor(url: impl Into<String>, label: Option<String>) -> Self {
        Self {
            url: url.into(),
            label,
            page: None,
            kind: UriKind::Anchor,
        }
    }

    /// Create a new email URI.
    #[cfg(test)]
    pub(crate) fn email(url: impl Into<String>, label: Option<String>) -> Self {
        Self {
            url: url.into(),
            label,
            page: None,
            kind: UriKind::Email,
        }
    }

    /// Create a new reference URI.
    #[cfg(test)]
    pub(crate) fn reference(url: impl Into<String>, label: Option<String>) -> Self {
        Self {
            url: url.into(),
            label,
            page: None,
            kind: UriKind::Reference,
        }
    }

    /// Set the page number.
    #[must_use]
    pub(crate) fn with_page(mut self, page: u32) -> Self {
        self.page = Some(page);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uri_hyperlink() {
        let uri = Uri::hyperlink("https://example.com", Some("Example".to_string()));
        assert_eq!(uri.kind, UriKind::Hyperlink);
        assert_eq!(uri.url, "https://example.com");
        assert_eq!(uri.label, Some("Example".to_string()));
    }

    #[test]
    fn test_uri_mailto_auto_detects_email() {
        let uri = Uri::hyperlink("mailto:test@example.com", None);
        assert_eq!(uri.kind, UriKind::Email);
    }

    #[test]
    fn test_uri_citation() {
        let uri = Uri::citation("10.1234/test", Some("Smith 2024".to_string()));
        assert_eq!(uri.kind, UriKind::Citation);
    }

    #[test]
    fn test_uri_with_page() {
        let uri = Uri::hyperlink("https://example.com", None).with_page(5);
        assert_eq!(uri.page, Some(5));
    }

    #[test]
    fn test_uri_serialization() {
        let uri = Uri::hyperlink("https://example.com", Some("Example".to_string()));
        let json = serde_json::to_string(&uri).unwrap();
        assert!(json.contains("\"url\":\"https://example.com\""));
        assert!(json.contains("\"kind\":\"hyperlink\""));

        let deserialized: Uri = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, uri);
    }
}
