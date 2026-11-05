use super::*;
use crate::core::config::OutputFormat;
use crate::types::builder::superscript;
use crate::types::internal_builder::InternalDocumentBuilder;

const HEADING: &str = "Cafe\u{301}";
const BODY: &str = "x2 belongs to ACCOUNT-1234.";
const MARKDOWN: &str = "# Cafe\u{301}\n\nx<sup>2</sup> belongs to ACCOUNT-1234.\n";

fn document() -> InternalDocument {
    let mut builder = InternalDocumentBuilder::new("test");
    builder.push_heading(1, HEADING, Some(1), None);
    builder.push_paragraph(BODY, vec![superscript(1, 2)], Some(1), None);
    builder.build()
}

fn config() -> ExtractionConfig {
    ExtractionConfig {
        output_format: OutputFormat::Markdown,
        enable_quality_processing: false,
        use_cache: false,
        disable_ocr: true,
        ..Default::default()
    }
}

fn assert_markup_survives(result: &ExtractedDocument) {
    assert_eq!(result.content, MARKDOWN);
    assert!(result.processing_warnings.iter().all(|w| w.source != "output_format"));
}

#[cfg(feature = "tokio-runtime")]
mod asynchronous {
    use super::*;
    use crate::plugins::{Plugin, PostProcessor, ProcessingStage};
    use std::sync::Arc;

    struct ReplaceAccount {
        both_surfaces: bool,
    }

    impl Plugin for ReplaceAccount {
        fn name(&self) -> &str {
            "normalization-account-replacement"
        }
    }

    #[async_trait::async_trait]
    impl PostProcessor for ReplaceAccount {
        async fn process(&self, result: &mut ExtractedDocument, _: &ExtractionConfig) -> Result<()> {
            result.content = result.content.replace("ACCOUNT-1234", "[REDACTED]");
            if self.both_surfaces
                && let Some(formatted) = result.formatted_content.as_mut()
            {
                *formatted = formatted.replace("ACCOUNT-1234", "[REDACTED]");
            }
            Ok(())
        }

        fn processing_stage(&self) -> ProcessingStage {
            ProcessingStage::Late
        }
    }

    async fn processed(edit: Option<bool>) -> ExtractedDocument {
        let state = Arc::new(initialization::ProcessorRegistryState::new_isolated());
        if let Some(both_surfaces) = edit {
            state.register(Arc::new(ReplaceAccount { both_surfaces })).unwrap();
        }
        run_pipeline_with_isolated_registry(document(), &config(), state)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn canonical_normalization_preserves_rendered_heading_and_script() {
        assert_markup_survives(&processed(None).await);
    }

    #[tokio::test]
    async fn a_plain_text_edit_still_discards_the_stale_rendering() {
        let result = processed(Some(false)).await;
        assert_eq!(result.content, "Café\nx2 belongs to [REDACTED].");
        assert!(result.processing_warnings.iter().any(|w| w.source == "output_format"));
        assert!(!result.content.contains("ACCOUNT-1234"));
    }

    #[tokio::test]
    async fn a_processor_that_updates_both_surfaces_retains_its_rendering() {
        let result = processed(Some(true)).await;
        assert_eq!(result.content, MARKDOWN.replace("ACCOUNT-1234", "[REDACTED]"));
        assert!(result.processing_warnings.iter().all(|w| w.source != "output_format"));
    }
}

#[cfg(not(feature = "tokio-runtime"))]
#[test]
fn synchronous_canonical_normalization_preserves_rendered_heading_and_script() {
    assert_markup_survives(&run_pipeline_sync(document(), &config()).unwrap());
}

#[test]
fn a_canonical_plain_text_change_keeps_formatted_identifiers_byte_exact() {
    let rendered = "# Cafe\u{301}\n\n[resource](https://example.test/e\u{301})\n";
    let source = ("Cafe\u{301}".to_owned(), rendered.to_owned());
    let mut result = ExtractedDocument {
        content: "Café".into(),
        formatted_content: Some(rendered.into()),
        ..Default::default()
    };
    discard_diverged_formatted_content(&mut result, Some(&source));
    assert_eq!(result.formatted_content.as_deref(), Some(rendered));
    assert!(result.processing_warnings.is_empty());
}

#[test]
fn compatibility_characters_are_not_treated_as_canonical_equivalents() {
    let rendered = "x<sup>2</sup>\n";
    let source = ("x²".to_owned(), rendered.to_owned());
    let mut result = ExtractedDocument {
        content: "x2".into(),
        formatted_content: Some(rendered.into()),
        ..Default::default()
    };
    discard_diverged_formatted_content(&mut result, Some(&source));
    assert!(result.formatted_content.is_none());
    assert_eq!(result.processing_warnings.len(), 1);
    assert_eq!(result.processing_warnings[0].source, "output_format");
}
