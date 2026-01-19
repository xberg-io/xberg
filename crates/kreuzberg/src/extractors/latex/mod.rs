//! Native Rust LaTeX text extractor.
//!
//! This extractor provides comprehensive LaTeX document parsing and text extraction.
//!
//! Features:
//! - Metadata extraction: title, author, date from \title{}, \author{}, \date{}
//! - Section hierarchy: \section{}, \subsection{}, \subsubsection{}, etc.
//! - Inline formatting: \emph{}, \textbf{}, \textit{}, \texttt{}, \underline{}
//! - Lists: itemize, enumerate, description environments
//! - Tables: tabular environment parsing
//! - Math: inline ($...$) and display (\[...\]) math preservation
//! - Unicode support
//!
//! Requires the `office` feature.

mod parser;
mod metadata;
mod commands;
mod environments;
mod utilities;

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::{ExtractionResult, Metadata, Table};
use async_trait::async_trait;

use parser::LatexParser;

/// LaTeX document extractor
pub struct LatexExtractor;

impl LatexExtractor {
    /// Create a new LaTeX extractor.
    pub fn new() -> Self {
        Self
    }

    /// Parse LaTeX content and extract text.
    fn extract_from_latex(content: &str) -> (String, Metadata, Vec<Table>) {
        let mut parser = LatexParser::new(content);
        parser.parse()
    }
}

impl Default for LatexExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for LatexExtractor {
    fn name(&self) -> &str {
        "latex-extractor"
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
        "Native Rust LaTeX document extractor with metadata and table support"
    }

    fn author(&self) -> &str {
        "Kreuzberg Team"
    }
}

#[async_trait]
impl DocumentExtractor for LatexExtractor {
    #[cfg_attr(feature = "otel", tracing::instrument(
        skip(self, content, _config),
        fields(
            extractor.name = self.name(),
            content.size_bytes = content.len(),
        )
    ))]
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        _config: &ExtractionConfig,
    ) -> Result<ExtractionResult> {
        let latex_str = String::from_utf8_lossy(content).to_string();
        let (text, metadata, tables) = Self::extract_from_latex(&latex_str);

        Ok(ExtractionResult {
            content: text,
            mime_type: mime_type.to_string(),
            metadata,
            tables,
            detected_languages: None,
            chunks: None,
            images: None,
            djot_content: None,
            pages: None,
            elements: None,
        })
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["application/x-latex", "text/x-tex"]
    }

    fn priority(&self) -> i32 {
        50
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_title_extraction() {
        let latex = r#"\title{Hello World}"#;
        let (_, metadata, _) = LatexExtractor::extract_from_latex(latex);
        assert_eq!(
            metadata.additional.get("title").and_then(|v| v.as_str()),
            Some("Hello World")
        );
    }

    #[test]
    fn test_author_extraction() {
        let latex = r#"\author{John Doe}"#;
        let (_, metadata, _) = LatexExtractor::extract_from_latex(latex);
        assert!(metadata.additional.contains_key("author"));
    }

    #[test]
    fn test_section_extraction() {
        let latex = r#"\begin{document}\section{Introduction}\end{document}"#;
        let (content, _, _) = LatexExtractor::extract_from_latex(latex);
        assert!(content.contains("Introduction"));
    }
}
