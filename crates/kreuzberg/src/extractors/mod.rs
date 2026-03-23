//! Built-in document extractors.
//!
//! This module contains the default extractors that ship with Kreuzberg.
//! All extractors implement the `DocumentExtractor` plugin trait.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::plugins::registry::get_document_extractor_registry;
use crate::types::ExtractionResult;
use once_cell::sync::Lazy;
use std::sync::Arc;

/// Trait for extractors that can work synchronously (WASM-compatible).
///
/// This trait defines the synchronous extraction interface for WASM targets and other
/// environments where async/tokio runtimes are not available or desirable.
///
/// # Implementation
///
/// Extractors that need to support WASM should implement this trait in addition to
/// the async `DocumentExtractor` trait. This allows the same extractor to work in both
/// environments by delegating to the sync implementation.
///
/// # MIME Type Validation
///
/// The `mime_type` parameter is guaranteed to be already validated.
///
/// # Example
///
/// ```rust,ignore
/// impl SyncExtractor for PlainTextExtractor {
///     fn extract_sync(&self, content: &[u8], config: &ExtractionConfig) -> Result<ExtractionResult> {
///         let text = String::from_utf8_lossy(content).to_string();
///         Ok(ExtractionResult {
///             content: text,
///             mime_type: "text/plain".to_string(),
///             metadata: Metadata::default(),
///             tables: vec![],
///             detected_languages: None,
///             chunks: None,
///             images: None,
///         })
///     }
/// }
/// ```
pub trait SyncExtractor {
    /// Extract content from a byte array synchronously.
    ///
    /// This method performs extraction without requiring an async runtime.
    /// It is called by `extract_bytes_sync()` when the `tokio-runtime` feature is disabled.
    ///
    /// # Arguments
    ///
    /// * `content` - Raw document bytes
    /// * `mime_type` - MIME type of the document (already validated)
    /// * `config` - Extraction configuration
    ///
    /// # Returns
    ///
    /// An `ExtractionResult` containing the extracted content and metadata.
    fn extract_sync(&self, content: &[u8], mime_type: &str, config: &ExtractionConfig) -> Result<ExtractionResult>;
}

pub mod csv;
pub mod structured;
pub mod text;

pub mod djot_format;
pub mod frontmatter_utils;

#[cfg(feature = "archives")]
pub mod security;

#[cfg(any(feature = "ocr", feature = "ocr-wasm"))]
pub mod image;

#[cfg(feature = "archives")]
pub mod archive;

#[cfg(feature = "email")]
pub mod email;

#[cfg(any(feature = "excel", feature = "excel-wasm"))]
pub mod excel;

#[cfg(feature = "hwp")]
pub mod hwp;

#[cfg(feature = "iwork")]
pub mod iwork;

#[cfg(feature = "html")]
pub mod html;

#[cfg(feature = "office")]
pub mod bibtex;

#[cfg(feature = "office")]
pub mod citation;

#[cfg(feature = "office")]
pub mod doc;

#[cfg(feature = "office")]
pub mod dbf;

#[cfg(feature = "office")]
pub mod docx;

#[cfg(feature = "office")]
pub mod epub;

#[cfg(feature = "office")]
pub mod fictionbook;

#[cfg(feature = "office")]
pub mod markdown;

#[cfg(feature = "mdx")]
pub mod mdx;

#[cfg(feature = "office")]
pub mod rst;

#[cfg(feature = "office")]
pub mod latex;

#[cfg(feature = "office")]
pub mod jupyter;

#[cfg(feature = "office")]
pub mod orgmode;

#[cfg(feature = "office")]
pub mod odt;

#[cfg(feature = "office")]
pub mod opml;

#[cfg(feature = "office")]
pub mod typst;

#[cfg(feature = "xml")]
pub mod jats;

#[cfg(feature = "pdf")]
pub mod pdf;

#[cfg(feature = "office")]
pub mod ppt;

#[cfg(feature = "office")]
pub mod pptx;

#[cfg(feature = "office")]
pub mod rtf;

#[cfg(feature = "xml")]
pub mod xml;

#[cfg(feature = "xml")]
pub mod docbook;

pub use csv::CsvExtractor;
pub use structured::StructuredExtractor;
pub use text::{MarkdownExtractor, PlainTextExtractor};

#[cfg(any(feature = "ocr", feature = "ocr-wasm"))]
pub use image::ImageExtractor;

#[cfg(feature = "archives")]
pub use archive::{GzipExtractor, SevenZExtractor, TarExtractor, ZipExtractor};

#[cfg(feature = "email")]
pub use email::EmailExtractor;

#[cfg(any(feature = "excel", feature = "excel-wasm"))]
pub use excel::ExcelExtractor;

#[cfg(feature = "hwp")]
pub use hwp::HwpExtractor;

#[cfg(feature = "iwork")]
pub use iwork::{keynote::KeynoteExtractor, numbers::NumbersExtractor, pages::PagesExtractor};

#[cfg(feature = "html")]
pub use html::HtmlExtractor;

#[cfg(feature = "office")]
pub use bibtex::BibtexExtractor;

#[cfg(feature = "office")]
pub use citation::CitationExtractor;

#[cfg(feature = "office")]
pub use dbf::DbfExtractor;

#[cfg(feature = "office")]
pub use doc::DocExtractor;

#[cfg(feature = "office")]
pub use docx::DocxExtractor;

#[cfg(feature = "office")]
pub use epub::EpubExtractor;

#[cfg(feature = "office")]
pub use fictionbook::FictionBookExtractor;

pub use djot_format::DjotExtractor;

#[cfg(feature = "office")]
pub use markdown::MarkdownExtractor as EnhancedMarkdownExtractor;

#[cfg(feature = "mdx")]
pub use mdx::MdxExtractor;

#[cfg(feature = "office")]
pub use rst::RstExtractor;

#[cfg(feature = "office")]
pub use latex::LatexExtractor;

#[cfg(feature = "office")]
pub use jupyter::JupyterExtractor;

#[cfg(feature = "office")]
pub use orgmode::OrgModeExtractor;

#[cfg(feature = "office")]
pub use odt::OdtExtractor;

#[cfg(feature = "xml")]
pub use jats::JatsExtractor;

#[cfg(feature = "office")]
pub use opml::OpmlExtractor;

#[cfg(feature = "office")]
pub use typst::TypstExtractor;

#[cfg(feature = "pdf")]
pub use pdf::PdfExtractor;

#[cfg(feature = "office")]
pub use ppt::PptExtractor;

#[cfg(feature = "office")]
pub use pptx::PptxExtractor;

#[cfg(feature = "office")]
pub use rtf::RtfExtractor;

#[cfg(feature = "xml")]
pub use xml::XmlExtractor;

#[cfg(feature = "xml")]
pub use docbook::DocbookExtractor;

/// Lazy-initialized flag that ensures extractors are registered exactly once.
///
/// This static is accessed on first extraction operation to automatically
/// register all built-in extractors with the plugin registry.
static EXTRACTORS_INITIALIZED: Lazy<Result<()>> = Lazy::new(register_default_extractors);

/// Ensure built-in extractors are registered.
///
/// This function is called automatically on first extraction operation.
/// It's safe to call multiple times - registration only happens once,
/// unless the registry was cleared, in which case extractors are re-registered.
pub fn ensure_initialized() -> Result<()> {
    EXTRACTORS_INITIALIZED
        .as_ref()
        .map(|_| ())
        .map_err(|e| crate::KreuzbergError::Plugin {
            message: format!("Failed to register default extractors: {}", e),
            plugin_name: "built-in-extractors".to_string(),
        })?;

    let registry = get_document_extractor_registry();
    let registry_guard = registry
        .read()
        .map_err(|e| crate::KreuzbergError::Other(format!("Document extractor registry lock poisoned: {}", e)))?;

    if registry_guard.list().is_empty() {
        drop(registry_guard);
        register_default_extractors()?;
    }

    Ok(())
}

/// Register all built-in extractors with the global registry.
///
/// This function should be called once at application startup to register
/// the default extractors (PlainText, Markdown, XML, etc.).
///
/// **Note:** This is called automatically on first extraction operation.
/// Explicit calling is optional.
///
/// # Example
///
/// ```rust
/// use kreuzberg::extractors::register_default_extractors;
///
/// # fn main() -> kreuzberg::Result<()> {
/// register_default_extractors()?;
/// # Ok(())
/// # }
/// ```
pub fn register_default_extractors() -> Result<()> {
    let registry = get_document_extractor_registry();
    let mut registry = registry
        .write()
        .map_err(|e| crate::KreuzbergError::Other(format!("Document extractor registry lock poisoned: {}", e)))?;

    registry.register(Arc::new(PlainTextExtractor::new()))?;
    registry.register(Arc::new(MarkdownExtractor::new()))?;
    registry.register(Arc::new(StructuredExtractor::new()))?;
    registry.register(Arc::new(CsvExtractor::new()))?;

    #[cfg(any(feature = "ocr", feature = "ocr-wasm"))]
    registry.register(Arc::new(ImageExtractor::new()))?;

    #[cfg(feature = "xml")]
    {
        registry.register(Arc::new(XmlExtractor::new()))?;
        registry.register(Arc::new(JatsExtractor::new()))?;
        registry.register(Arc::new(DocbookExtractor::new()))?;
    }

    #[cfg(feature = "pdf")]
    registry.register(Arc::new(PdfExtractor::new()))?;

    #[cfg(any(feature = "excel", feature = "excel-wasm"))]
    registry.register(Arc::new(ExcelExtractor::new()))?;

    registry.register(Arc::new(DjotExtractor::new()))?;

    #[cfg(feature = "office")]
    {
        registry.register(Arc::new(EnhancedMarkdownExtractor::new()))?;
        registry.register(Arc::new(BibtexExtractor::new()))?;
        registry.register(Arc::new(CitationExtractor::new()))?;
        registry.register(Arc::new(EpubExtractor::new()))?;
        registry.register(Arc::new(FictionBookExtractor::new()))?;
        registry.register(Arc::new(RtfExtractor::new()))?;
        registry.register(Arc::new(RstExtractor::new()))?;
        registry.register(Arc::new(LatexExtractor::new()))?;
        registry.register(Arc::new(JupyterExtractor::new()))?;
        registry.register(Arc::new(OrgModeExtractor::new()))?;
        registry.register(Arc::new(OpmlExtractor::new()))?;
        registry.register(Arc::new(TypstExtractor::new()))?;
        registry.register(Arc::new(DocExtractor::new()))?;
        registry.register(Arc::new(DocxExtractor::new()))?;
        registry.register(Arc::new(PptExtractor::new()))?;
        registry.register(Arc::new(PptxExtractor::new()))?;
        registry.register(Arc::new(OdtExtractor::new()))?;
        registry.register(Arc::new(DbfExtractor::new()))?;
    }

    #[cfg(feature = "hwp")]
    {
        registry.register(Arc::new(HwpExtractor::new()))?;
    }

    #[cfg(feature = "iwork")]
    {
        registry.register(Arc::new(PagesExtractor::new()))?;
        registry.register(Arc::new(NumbersExtractor::new()))?;
        registry.register(Arc::new(KeynoteExtractor::new()))?;
    }

    #[cfg(feature = "mdx")]
    registry.register(Arc::new(MdxExtractor::new()))?;

    #[cfg(feature = "email")]
    registry.register(Arc::new(EmailExtractor::new()))?;

    #[cfg(feature = "html")]
    registry.register(Arc::new(HtmlExtractor::new()))?;

    #[cfg(feature = "archives")]
    {
        registry.register(Arc::new(ZipExtractor::new()))?;
        registry.register(Arc::new(TarExtractor::new()))?;
        registry.register(Arc::new(SevenZExtractor::new()))?;
        registry.register(Arc::new(GzipExtractor::new()))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_default_extractors() {
        let registry = get_document_extractor_registry();
        {
            let mut reg = registry
                .write()
                .expect("Failed to acquire write lock on registry in test");
            *reg = crate::plugins::registry::DocumentExtractorRegistry::new();
        }

        register_default_extractors().expect("Failed to register extractors");

        let reg = registry
            .read()
            .expect("Failed to acquire read lock on registry in test");
        let extractor_names = reg.list();

        #[allow(unused_mut)]
        let mut expected_count = 5; // plain-text, markdown, structured, djot, csv
        assert!(extractor_names.contains(&"plain-text-extractor".to_string()));
        assert!(extractor_names.contains(&"markdown-extractor".to_string()));
        assert!(extractor_names.contains(&"structured-extractor".to_string()));
        assert!(extractor_names.contains(&"djot-extractor".to_string()));
        assert!(extractor_names.contains(&"csv-extractor".to_string()));

        #[cfg(any(feature = "ocr", feature = "ocr-wasm"))]
        {
            expected_count += 1;
            assert!(extractor_names.contains(&"image-extractor".to_string()));
        }

        #[cfg(feature = "xml")]
        {
            expected_count += 3;
            assert!(extractor_names.contains(&"xml-extractor".to_string()));
            assert!(extractor_names.contains(&"jats-extractor".to_string()));
            assert!(extractor_names.contains(&"docbook-extractor".to_string()));
        }

        #[cfg(feature = "pdf")]
        {
            expected_count += 1;
            assert!(extractor_names.contains(&"pdf-extractor".to_string()));
        }

        #[cfg(any(feature = "excel", feature = "excel-wasm"))]
        {
            expected_count += 1;
            assert!(extractor_names.contains(&"excel-extractor".to_string()));
        }

        #[cfg(feature = "office")]
        {
            expected_count += 18;
            assert!(extractor_names.contains(&"markdown-extractor".to_string()));
            assert!(extractor_names.contains(&"bibtex-extractor".to_string()));
            assert!(extractor_names.contains(&"citation-extractor".to_string()));
            assert!(extractor_names.contains(&"epub-extractor".to_string()));
            assert!(extractor_names.contains(&"fictionbook-extractor".to_string()));
            assert!(extractor_names.contains(&"rtf-extractor".to_string()));
            assert!(extractor_names.contains(&"rst-extractor".to_string()));
            assert!(extractor_names.contains(&"latex-extractor".to_string()));
            assert!(extractor_names.contains(&"jupyter-extractor".to_string()));
            assert!(extractor_names.contains(&"orgmode-extractor".to_string()));
            assert!(extractor_names.contains(&"opml-extractor".to_string()));
            assert!(extractor_names.contains(&"typst-extractor".to_string()));
            assert!(extractor_names.contains(&"dbf-extractor".to_string()));
            assert!(extractor_names.contains(&"doc-extractor".to_string()));
            assert!(extractor_names.contains(&"docx-extractor".to_string()));
            assert!(extractor_names.contains(&"ppt-extractor".to_string()));
            assert!(extractor_names.contains(&"pptx-extractor".to_string()));
            assert!(extractor_names.contains(&"odt-extractor".to_string()));
        }

        #[cfg(feature = "hwp")]
        {
            expected_count += 1;
            assert!(extractor_names.contains(&"hwp-extractor".to_string()));
        }

        #[cfg(feature = "iwork")]
        {
            expected_count += 3;
            assert!(extractor_names.contains(&"pages-extractor".to_string()));
            assert!(extractor_names.contains(&"numbers-extractor".to_string()));
            assert!(extractor_names.contains(&"keynote-extractor".to_string()));
        }

        #[cfg(feature = "mdx")]
        {
            expected_count += 1;
            assert!(extractor_names.contains(&"mdx-extractor".to_string()));
        }

        #[cfg(feature = "email")]
        {
            expected_count += 1;
            assert!(extractor_names.contains(&"email-extractor".to_string()));
        }

        #[cfg(feature = "html")]
        {
            expected_count += 1;
            assert!(extractor_names.contains(&"html-extractor".to_string()));
        }

        #[cfg(feature = "archives")]
        {
            expected_count += 4;
            assert!(extractor_names.contains(&"zip-extractor".to_string()));
            assert!(extractor_names.contains(&"tar-extractor".to_string()));
            assert!(extractor_names.contains(&"7z-extractor".to_string()));
            assert!(extractor_names.contains(&"gzip-extractor".to_string()));
        }

        assert_eq!(
            extractor_names.len(),
            expected_count,
            "Expected {} extractors based on enabled features",
            expected_count
        );
    }

    #[test]
    fn test_ensure_initialized() {
        ensure_initialized().expect("Failed to ensure extractors initialized");
    }
}
