//! Page extraction and tracking configuration.
//!
//! Controls how pages are extracted, tracked, and represented in extraction results.
//! When `None`, page tracking is disabled.

use serde::{Deserialize, Serialize};

/// Page extraction and tracking configuration.
///
/// Controls how pages are extracted, tracked, and represented in the extraction results.
/// When `None`, page tracking is disabled.
///
/// Page range tracking in chunk metadata (first_page/last_page) is automatically enabled
/// when page boundaries are available and chunking is configured.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PageConfig {
    /// Extract pages as separate array (ExtractedDocument.pages)
    #[serde(default)]
    pub extract_pages: bool,

    /// Insert page markers in main content string
    #[serde(default)]
    pub insert_page_markers: bool,

    /// Page marker format (use {page_num} placeholder)
    /// Default: "\n\n<!-- PAGE {page_num} -->\n\n"
    #[serde(default = "default_page_marker_format")]
    pub marker_format: String,

    /// Keep the text layer of a PDF page whose `content` OCR output replaces, for a caller that needs the exact
    /// characters of a short born-digital page. The text layer is returned in `PageContent::native_content`.
    /// Needs `extract_pages`.
    #[serde(default)]
    #[cfg_attr(feature = "alef-meta", alef(since = "1.3.8"))]
    pub keep_native_content: bool,
}

impl PageConfig {
    /// Validate the settings that depend on each other.
    ///
    /// # Errors
    ///
    /// Returns `XbergError::Validation` when `keep_native_content` is on and `extract_pages` is off.
    pub(crate) fn validate(&self) -> crate::Result<()> {
        if self.keep_native_content && !self.extract_pages {
            return Err(crate::XbergError::validation(
                "`pages.keep_native_content` needs `pages.extract_pages = true`. The kept text is returned on `pages[]`.",
            ));
        }
        Ok(())
    }
}

impl Default for PageConfig {
    fn default() -> Self {
        Self {
            extract_pages: false,
            insert_page_markers: false,
            marker_format: "\n\n<!-- PAGE {page_num} -->\n\n".to_string(),
            keep_native_content: false,
        }
    }
}

fn default_page_marker_format() -> String {
    "\n\n<!-- PAGE {page_num} -->\n\n".to_string()
}

/// Regex matching one whole line that is a rendered instance of `marker_format`
/// (every `{page_num}` placeholder replaced by digits). Used to recognize page
/// marker lines so renderers pass them through verbatim.
pub(crate) fn marker_line_regex(marker_format: &str) -> regex::Regex {
    let pattern = marker_format
        .trim()
        .split("{page_num}")
        .map(regex::escape)
        .collect::<Vec<_>>()
        .join(r"\d+");
    regex::Regex::new(&format!("^{pattern}$")).expect("escaped marker format is a valid regex")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_config_default() {
        let config = PageConfig::default();
        assert!(!config.extract_pages);
        assert!(!config.insert_page_markers);
        assert_eq!(config.marker_format, "\n\n<!-- PAGE {page_num} -->\n\n");
    }

    #[test]
    fn keep_native_content_without_extract_pages_is_rejected() {
        let rejected = PageConfig {
            keep_native_content: true,
            ..Default::default()
        };
        let error = rejected
            .validate()
            .expect_err("the kept text has no page to be returned on");
        assert!(matches!(error, crate::XbergError::Validation { .. }), "got: {error:?}");
        assert!(
            error.to_string().contains("`pages.extract_pages = true`"),
            "the error must name the setting to change; got: {error}"
        );

        let accepted = PageConfig {
            extract_pages: true,
            keep_native_content: true,
            ..Default::default()
        };
        accepted.validate().expect("both settings on is a valid config");
        PageConfig::default().validate().expect("the default config is valid");
    }

    #[test]
    fn keep_native_content_is_read_from_its_key_and_is_off_without_it() {
        let with_key: PageConfig = serde_json::from_str(r#"{"extract_pages": true, "keep_native_content": true}"#)
            .expect("a config with the key parses");
        assert!(with_key.keep_native_content);
        assert_ne!(with_key.keep_native_content, PageConfig::default().keep_native_content);

        let without_key: PageConfig =
            serde_json::from_str(r#"{"extract_pages": true}"#).expect("a config without the key parses");
        assert!(without_key.extract_pages);
        assert!(!without_key.keep_native_content);
    }
}
