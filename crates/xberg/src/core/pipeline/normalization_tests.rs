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

fn nfc_page(content: &str, native_content: Option<&str>) -> crate::types::PageContent {
    crate::types::PageContent {
        page_number: 1,
        content: content.to_owned(),
        tables: Vec::new(),
        image_indices: Vec::new(),
        image_preprocessing: None,
        hierarchy: None,
        is_blank: None,
        layout_regions: None,
        speaker_notes: None,
        section_name: None,
        sheet_name: None,
        ocr_confidence: None,
        native_content: native_content.map(str::to_owned),
        geometry: None,
        viewports: None,
    }
}

#[test]
fn normalize_nfc_keeps_already_nfc_text_in_its_original_buffers() {
    let text = "Café, naïve, Straße 東京";
    let mut result = ExtractedDocument {
        content: text.to_owned(),
        pages: Some(vec![nfc_page(text, Some(text))]),
        ..Default::default()
    };
    let content_buffer = result.content.as_ptr();
    let pages = result.pages.as_ref().unwrap();
    let page_buffer = pages[0].content.as_ptr();
    let native_buffer = pages[0].native_content.as_ref().unwrap().as_ptr();

    normalize_nfc(&mut result);

    assert_eq!(result.content, text);
    assert_eq!(result.content.as_ptr(), content_buffer);
    let page = &result.pages.as_ref().unwrap()[0];
    assert_eq!(page.content, text);
    assert_eq!(page.content.as_ptr(), page_buffer);
    assert_eq!(page.native_content.as_deref(), Some(text));
    assert_eq!(page.native_content.as_ref().unwrap().as_ptr(), native_buffer);
}

#[test]
fn normalize_nfc_still_composes_decomposed_content_and_pages() {
    let mut result = ExtractedDocument {
        content: "Cafe\u{301} na\u{131}ve".to_owned(),
        pages: Some(vec![nfc_page("Cafe\u{301}", Some("e\u{301}t\u{e9}"))]),
        ..Default::default()
    };

    normalize_nfc(&mut result);

    assert_eq!(result.content, "Caf\u{e9} na\u{131}ve");
    let page = &result.pages.as_ref().unwrap()[0];
    assert_eq!(page.content, "Caf\u{e9}");
    assert_eq!(page.native_content.as_deref(), Some("\u{e9}t\u{e9}"));
}

#[test]
fn normalize_nfc_skips_an_nfc_page_beside_a_decomposed_one() {
    let mut result = ExtractedDocument {
        content: "Caf\u{e9}".to_owned(),
        pages: Some(vec![nfc_page("Caf\u{e9}", None), nfc_page("Cafe\u{301}", None)]),
        ..Default::default()
    };
    let nfc_page_buffer = result.pages.as_ref().unwrap()[0].content.as_ptr();

    normalize_nfc(&mut result);

    let pages = result.pages.as_ref().unwrap();
    assert_eq!(pages[0].content.as_ptr(), nfc_page_buffer);
    assert_eq!(pages[1].content, "Caf\u{e9}");
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
