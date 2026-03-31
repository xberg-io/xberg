//! Source code extractor using tree-sitter language pack.
//!
//! Extracts content and structural analysis from source code files using
//! tree-sitter parsers. Language detection is performed via file extension
//! or shebang line.

use std::borrow::Cow;
use std::path::Path;

use async_trait::async_trait;
use tree_sitter_language_pack as tslp;

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::core::mime::SOURCE_CODE_MIME_TYPE;
use crate::extractors::SyncExtractor;
use crate::internal_builder::InternalDocumentBuilder;
use crate::plugins::DocumentExtractor;
use crate::plugins::Plugin;
use crate::types::internal::InternalDocument;
use crate::types::metadata::{FormatMetadata, Metadata};

/// Source code extractor using tree-sitter language pack.
///
/// Detects the programming language from the file extension or shebang line,
/// then uses tree-sitter to parse and extract structural information.
pub struct CodeExtractor;

impl Default for CodeExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeExtractor {
    pub fn new() -> Self {
        Self
    }

    /// Build a `tslp::ProcessConfig` from the kreuzberg `TreeSitterProcessConfig`.
    fn build_process_config(language: &str, config: &ExtractionConfig) -> tslp::ProcessConfig {
        if let Some(ref ts_config) = config.tree_sitter {
            let pc: tslp::ProcessConfig = (&ts_config.process).into();
            return tslp::ProcessConfig {
                language: Cow::Owned(language.to_string()),
                ..pc
            };
        }
        tslp::ProcessConfig::new(language)
    }

    /// Extract from source text with a known language.
    fn extract_with_language(source: &str, language: &str, config: &ExtractionConfig) -> Result<InternalDocument> {
        let process_config = Self::build_process_config(language, config);

        let result = tslp::process(source, &process_config).map_err(|e| crate::KreuzbergError::Parsing {
            message: format!("tree-sitter processing failed for language '{language}': {e}"),
            source: None,
        })?;

        let mut builder = InternalDocumentBuilder::new("code");

        for paragraph in source.split("\n\n") {
            let trimmed = paragraph.trim();
            if !trimmed.is_empty() {
                builder.push_paragraph(trimmed, vec![], None, None);
            }
        }

        let mut doc = builder.build();
        doc.metadata = Metadata {
            format: Some(FormatMetadata::Code(result)),
            ..Default::default()
        };
        doc.mime_type = Cow::Borrowed(SOURCE_CODE_MIME_TYPE);

        Ok(doc)
    }

    /// Detect language and read source from a file path.
    ///
    /// Returns `(language, source)`. Reads the file at most once.
    fn read_and_detect(path: &Path) -> Result<(String, String)> {
        let path_str = path.to_string_lossy();

        // Fast path: extension-based detection (no I/O)
        if let Some(lang) = tslp::detect_language_from_path(&path_str) {
            let source = std::fs::read_to_string(path)?;
            return Ok((lang.to_string(), source));
        }

        // Slow path: read file once, try shebang detection
        let source = std::fs::read_to_string(path)?;
        if let Some(lang) = tslp::detect_language_from_content(&source) {
            return Ok((lang.to_string(), source));
        }

        Err(crate::KreuzbergError::UnsupportedFormat(format!(
            "Cannot detect programming language for: {}",
            path.display()
        )))
    }
}

impl Plugin for CodeExtractor {
    fn name(&self) -> &str {
        "code-extractor"
    }

    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    fn initialize(&self) -> Result<()> {
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    fn description(&self) -> &str {
        "Extracts content and structure from source code files using tree-sitter"
    }

    fn author(&self) -> &str {
        "Kreuzberg Team"
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl DocumentExtractor for CodeExtractor {
    async fn extract_bytes(
        &self,
        content: &[u8],
        _mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<InternalDocument> {
        tracing::debug!(format = "code", size_bytes = content.len(), "extraction starting");
        let source = String::from_utf8_lossy(content);

        let language = tslp::detect_language_from_content(&source).ok_or_else(|| {
            crate::KreuzbergError::UnsupportedFormat(
                "Cannot detect programming language from content (no shebang line). \
                 Use extract_file with a file path for extension-based detection."
                    .to_string(),
            )
        })?;

        let doc = Self::extract_with_language(&source, language, config)?;
        tracing::debug!(
            element_count = doc.elements.len(),
            format = "code",
            "extraction complete"
        );
        Ok(doc)
    }

    async fn extract_file(&self, path: &Path, _mime_type: &str, config: &ExtractionConfig) -> Result<InternalDocument> {
        let (language, source) = Self::read_and_detect(path)?;
        Self::extract_with_language(&source, &language, config)
    }

    fn supported_mime_types(&self) -> &[&str] {
        &[SOURCE_CODE_MIME_TYPE]
    }

    fn priority(&self) -> i32 {
        50
    }

    fn as_sync_extractor(&self) -> Option<&dyn SyncExtractor> {
        Some(self)
    }
}

impl SyncExtractor for CodeExtractor {
    fn extract_sync(&self, content: &[u8], _mime_type: &str, config: &ExtractionConfig) -> Result<InternalDocument> {
        let source = String::from_utf8_lossy(content);

        let language = tslp::detect_language_from_content(&source).ok_or_else(|| {
            crate::KreuzbergError::UnsupportedFormat("Cannot detect programming language from content".to_string())
        })?;

        Self::extract_with_language(&source, language, config)
    }
}
