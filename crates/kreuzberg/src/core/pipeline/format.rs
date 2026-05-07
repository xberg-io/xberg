//! Output format conversion for extraction results.
//!
//! This module handles the final step of output format application: swapping
//! pre-rendered content into the result and recording format metadata.
//!
//! The heavy rendering work (Markdown, Djot, HTML) is now done earlier in the
//! pipeline inside `derive_extraction_result`, which populates
//! `ExtractionResult::formatted_content`. This function simply swaps that
//! pre-rendered content into the `content` field after post-processors have
//! operated on the plain-text version.

use crate::core::config::OutputFormat;
use crate::types::ExtractionResult;
#[cfg(test)]
use std::borrow::Cow;

/// Apply output format conversion to the extraction result.
///
/// Records the output format in metadata and swaps in pre-rendered content
/// (produced during `derive_extraction_result`) if available.
///
/// This runs as the final pipeline step, after post-processors have operated
/// on the plain-text `content` field.
///
/// # Arguments
///
/// * `result` - The extraction result to modify
/// * `output_format` - The desired output format
pub fn apply_output_format(result: ExtractionResult, output_format: OutputFormat) -> ExtractionResult {
    let mut result = result;
    let format_name = match output_format {
        OutputFormat::Plain => "plain",
        OutputFormat::Markdown => "markdown",
        OutputFormat::Djot => "djot",
        OutputFormat::Html => "html",
        OutputFormat::Json => "json",
        OutputFormat::Structured => "structured",
        OutputFormat::Custom(ref name) => name.as_str(),
    };
    result.metadata.output_format = Some(format_name.to_string());

    // Swap in pre-rendered content if available (populated by derive_extraction_result).
    if let Some(formatted) = result.formatted_content.take() {
        result.content = formatted;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Metadata;

    #[test]
    fn test_apply_output_format_plain() {
        let result = ExtractionResult {
            content: "Hello World".to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            ..Default::default()
        };

        let result = apply_output_format(result, OutputFormat::Plain);

        assert_eq!(result.content, "Hello World");
        assert_eq!(result.metadata.output_format, Some("plain".to_string()));
    }

    #[test]
    fn test_apply_output_format_markdown_no_prerender() {
        let result = ExtractionResult {
            content: "Hello World".to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            ..Default::default()
        };

        let result = apply_output_format(result, OutputFormat::Markdown);

        // Without pre-rendered content, content stays as-is
        assert_eq!(result.content, "Hello World");
        assert_eq!(result.metadata.output_format, Some("markdown".to_string()));
    }

    #[test]
    fn test_apply_output_format_swaps_formatted_content() {
        let result = ExtractionResult {
            content: "plain text".to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            formatted_content: Some("# Heading\n\nFormatted markdown".to_string()),
            ..Default::default()
        };

        let result = apply_output_format(result, OutputFormat::Markdown);

        assert_eq!(result.content, "# Heading\n\nFormatted markdown");
        assert!(result.formatted_content.is_none(), "formatted_content should be taken");
        assert_eq!(result.metadata.output_format, Some("markdown".to_string()));
    }

    #[test]
    fn test_apply_output_format_html_with_prerender() {
        let result = ExtractionResult {
            content: "plain text".to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            formatted_content: Some("<p>Hello World</p>".to_string()),
            ..Default::default()
        };

        let result = apply_output_format(result, OutputFormat::Html);

        assert_eq!(result.content, "<p>Hello World</p>");
        assert_eq!(result.metadata.output_format, Some("html".to_string()));
    }

    #[test]
    fn test_apply_output_format_djot_with_prerender() {
        let result = ExtractionResult {
            content: "plain text".to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            formatted_content: Some("# Djot heading".to_string()),
            ..Default::default()
        };

        let result = apply_output_format(result, OutputFormat::Djot);

        assert_eq!(result.content, "# Djot heading");
        assert_eq!(result.metadata.output_format, Some("djot".to_string()));
    }

    #[test]
    fn test_apply_output_format_structured() {
        let result = ExtractionResult {
            content: "Hello World".to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            ..Default::default()
        };

        let result = apply_output_format(result, OutputFormat::Structured);

        assert_eq!(result.content, "Hello World");
        assert_eq!(result.metadata.output_format, Some("structured".to_string()));
    }

    #[test]
    fn test_apply_output_format_preserves_metadata() {
        use ahash::AHashMap;
        let mut additional = AHashMap::new();
        additional.insert(Cow::Borrowed("custom_key"), serde_json::json!("custom_value"));
        let metadata = Metadata {
            title: Some("Test Title".to_string()),
            additional,
            ..Default::default()
        };

        let result = ExtractionResult {
            content: "Hello World".to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            metadata,
            ..Default::default()
        };

        let result = apply_output_format(result, OutputFormat::Markdown);

        assert_eq!(result.metadata.title, Some("Test Title".to_string()));
        assert_eq!(
            result.metadata.additional.get("custom_key"),
            Some(&serde_json::json!("custom_value"))
        );
    }

    #[test]
    fn test_apply_output_format_preserves_tables() {
        use crate::types::Table;

        let table = Table {
            cells: vec![vec!["A".to_string(), "B".to_string()]],
            markdown: "| A | B |".to_string(),
            page_number: 1,
            bounding_box: None,
        };

        let result = ExtractionResult {
            content: "Hello World".to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            tables: vec![table],
            ..Default::default()
        };

        let result = apply_output_format(result, OutputFormat::Html);

        assert_eq!(result.tables.len(), 1);
        assert_eq!(result.tables[0].cells[0][0], "A");
    }

    #[test]
    fn test_apply_output_format_sets_typed_field() {
        let result = ExtractionResult {
            content: "test".to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            ..Default::default()
        };

        let result = apply_output_format(result, OutputFormat::Djot);

        assert_eq!(result.metadata.output_format, Some("djot".to_string()));
    }
}
