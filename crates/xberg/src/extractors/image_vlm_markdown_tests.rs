use super::*;
use crate::core::config::{OcrConfig, OcrPipelineConfig, OcrPipelineStage, OcrQualityThresholds, OutputFormat};
use crate::plugins::{OcrBackend, OcrBackendType, Plugin};
use crate::types::ExtractedDocument;

/// The VLM backend with the model call replaced by a canned response: the response goes through
/// the same `build_vlm_result` that `VlmOcrBackend::process_image` returns.
struct CannedVlmBackend {
    response: &'static str,
}

impl Plugin for CannedVlmBackend {
    fn name(&self) -> &str {
        "vlm"
    }
    fn version(&self) -> String {
        "1.0.0".into()
    }
    fn initialize(&self) -> crate::Result<()> {
        Ok(())
    }
    fn shutdown(&self) -> crate::Result<()> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl OcrBackend for CannedVlmBackend {
    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }
    fn supports_language(&self, _: &str) -> bool {
        true
    }
    async fn process_image(&self, image: &[u8], config: &OcrConfig) -> crate::Result<ExtractedDocument> {
        let structure = matches!(config.output_format, Some(OutputFormat::Markdown));
        Ok(crate::llm::vlm_ocr::build_vlm_result(
            self.response.to_string(),
            None,
            image,
            vec!["eng".to_string()],
            structure,
        ))
    }
}

/// Stands in for the built-in `vlm` backend, the name the pipeline reads as a VLM stage, and
/// puts it back when dropped.
struct BackendRegistration;

impl Drop for BackendRegistration {
    fn drop(&mut self) {
        crate::plugins::unregister_ocr_backend("vlm").expect("remove owned test backend");
        crate::plugins::register_ocr_backend(std::sync::Arc::new(crate::llm::vlm_ocr::VlmOcrBackend))
            .expect("restore the built-in vlm backend");
    }
}

/// Run `response` through the standalone-image OCR route and render it.
async fn extract(response: &'static str, format: &OutputFormat) -> InternalDocument {
    let _ = crate::plugins::unregister_ocr_backend("vlm");
    crate::plugins::register_ocr_backend(std::sync::Arc::new(CannedVlmBackend { response })).unwrap();
    let _registration = BackendRegistration;
    let pipeline = OcrPipelineConfig {
        stages: vec![OcrPipelineStage {
            backend: "vlm".into(),
            priority: 100,
            language: None,
            tesseract_config: None,
            paddle_ocr_config: None,
            paddle_ocr_settings: None,
            vlm_config: None,
            backend_options: None,
        }],
        quality_thresholds: OcrQualityThresholds {
            pipeline_min_quality: 0.0,
            ..Default::default()
        },
    };
    let config = ExtractionConfig {
        use_cache: false,
        output_format: format.clone(),
        ocr: Some(OcrConfig {
            pipeline: Some(pipeline.clone()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let mut png = std::io::Cursor::new(Vec::new());
    image::RgbImage::from_pixel(16, 16, image::Rgb([255, 255, 255]))
        .write_to(&mut png, image::ImageFormat::Png)
        .unwrap();
    ImageExtractor::new()
        .extract_with_ocr_pipeline(&png.into_inner(), &config, &pipeline)
        .await
        .expect("image OCR pipeline")
}

async fn render(response: &'static str, format: OutputFormat, escape_markdown: bool) -> String {
    let mut document = extract(response, &format).await;
    document.escape_markdown = escape_markdown;
    match format {
        OutputFormat::Markdown => crate::rendering::render_markdown(&document),
        _ => crate::rendering::render_plain(&document),
    }
}

#[cfg(feature = "html")]
const HTML_TABLE_PAGE: &str = "# ANNUAL ACCOUNTS — INCOME STATEMENT\n\n<table>\n  <tr>\n    <th rowspan=\"2\">Account</th>\n    <th colspan=\"2\">Fiscal Year (EUR)</th>\n  </tr>\n  <tr>\n    <th>2025</th>\n    <th>2024</th>\n  </tr>\n  <tr>\n    <td>Revenue</td>\n    <td>1 482 350,40</td>\n    <td>1 319 820,15</td>\n  </tr>\n</table>";
const GFM_TABLE_PAGE: &str = "# MINUTES\n\nIntro paragraph.\n\n| Item | For | Against | Decision |\n|---|---|---|---|\n| 1. Accounts 2025 | 8 450 | 320 | Approved |\n| 2. Capital increase | 6 120 | 2 650 | Rejected (< 75 %) |\n\nClosing paragraph.";
const LIST_PAGE: &str = "- [X] Ordinary unsecured claim: 14 820,45 EUR\n- [ ] Preferred claim: ____________ EUR\n- [X] Accrued interest: 1 104,80 EUR";

#[cfg(feature = "html")]
#[tokio::test]
#[serial_test::serial]
async fn vlm_html_table_renders_as_one_markdown_table_with_merged_cells_flattened() {
    let expected = "# ANNUAL ACCOUNTS — INCOME STATEMENT\n\n| Account | Fiscal Year (EUR) |  |\n| --- | --- | --- |\n|  | 2025 | 2024 |\n| Revenue | 1 482 350,40 | 1 319 820,15 |\n";
    for escape_markdown in [true, false] {
        assert_eq!(
            render(HTML_TABLE_PAGE, OutputFormat::Markdown, escape_markdown).await,
            expected,
            "escape_markdown={escape_markdown}"
        );
    }
}

#[tokio::test]
#[serial_test::serial]
async fn vlm_gfm_table_renders_once_in_place_between_its_paragraphs() {
    // A `<` stays escaped with `escape_markdown` off, as it does in prose (#2275).
    for escape_markdown in [true, false] {
        let expected = "# MINUTES\n\nIntro paragraph.\n\n| Item | For | Against | Decision |\n| --- | --- | --- | --- |\n| 1. Accounts 2025 | 8 450 | 320 | Approved |\n| 2. Capital increase | 6 120 | 2 650 | Rejected (\\< 75 %) |\n\nClosing paragraph.\n";
        assert_eq!(
            render(GFM_TABLE_PAGE, OutputFormat::Markdown, escape_markdown).await,
            expected,
            "escape_markdown={escape_markdown}"
        );
    }
}

#[tokio::test]
#[serial_test::serial]
async fn vlm_task_list_keeps_one_item_per_line_unescaped() {
    let expected = "- [x] Ordinary unsecured claim: 14 820,45 EUR\n- [ ] Preferred claim: ____________ EUR\n- [x] Accrued interest: 1 104,80 EUR\n";
    for escape_markdown in [true, false] {
        assert_eq!(
            render(LIST_PAGE, OutputFormat::Markdown, escape_markdown).await,
            expected,
            "escape_markdown={escape_markdown}"
        );
    }
}

#[tokio::test]
#[serial_test::serial]
async fn vlm_plain_output_keeps_the_response_text() {
    assert_eq!(render(LIST_PAGE, OutputFormat::Plain, true).await.trim_end(), LIST_PAGE);
}

#[tokio::test]
#[serial_test::serial]
async fn vlm_prose_without_markdown_renders_as_before() {
    let prose = "First paragraph of a letter.\n\nSecond paragraph, paid 5 EUR.";
    assert_eq!(
        render(prose, OutputFormat::Markdown, true).await,
        "First paragraph of a letter.\n\nSecond paragraph, paid 5 EUR.\n"
    );
}

#[cfg(feature = "html")]
#[tokio::test]
#[serial_test::serial]
async fn vlm_mixed_page_keeps_every_block_in_order_with_each_table_once() {
    let page = "# Report\n\nIntro.\n\n<table><tr><th>A</th><th>B</th></tr><tr><td>1</td><td>2</td></tr></table>\n\n- one\n- two\n\n| X | Y |\n|---|---|\n| 3 | 4 |\n\nEnd.";
    let expected = "# Report\n\nIntro.\n\n| A | B |\n| --- | --- |\n| 1 | 2 |\n\n- one\n- two\n\n| X | Y |\n| --- | --- |\n| 3 | 4 |\n\nEnd.\n";
    assert_eq!(render(page, OutputFormat::Markdown, true).await, expected);
}

#[tokio::test]
#[serial_test::serial]
async fn vlm_gfm_table_is_listed_on_its_page() {
    let document = extract(GFM_TABLE_PAGE, &OutputFormat::Markdown).await;
    let result = crate::extraction::derive::derive_extraction_result(document, true, OutputFormat::Markdown);
    assert_eq!(result.tables.len(), 1);
    let pages = result.pages.as_ref().expect("standalone image page");
    assert_eq!(pages[0].tables.len(), 1);
    assert_eq!(pages[0].tables[0].cells[0], ["Item", "For", "Against", "Decision"]);
}
