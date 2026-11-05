use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::ExtractInput;
use crate::core::config::{ExtractionConfig, FileExtractionConfig, LayoutDetectionConfig, PageConfig};
use crate::engine::Engine;
use crate::engine::seams::CacheBackend;

const NATIVE_TEXT: &str = "Explicit layout selection preserves the complete native paragraph.";

fn native_pdf() -> Vec<u8> {
    use lopdf::{Document, Object, Stream, dictionary};
    let mut document = Document::with_version("1.5");
    let pages = document.new_object_id();
    let font = document.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica",
    });
    let content = document.add_object(Stream::new(
        dictionary! {},
        format!("BT /F1 12 Tf 72 720 Td ({NATIVE_TEXT}) Tj ET").into_bytes(),
    ));
    let page = document.add_object(dictionary! {
        "Type" => "Page", "Parent" => pages, "Contents" => content,
        "Resources" => dictionary! { "Font" => dictionary! { "F1" => font } },
        "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
    });
    document.objects.insert(
        pages,
        Object::Dictionary(dictionary! {
            "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1,
        }),
    );
    let catalog = document.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
    document.trailer.set("Root", catalog);
    let mut bytes = Vec::new();
    document.save_to(&mut bytes).expect("native PDF must serialize");
    assert_eq!(Document::load_mem(&bytes).unwrap().get_pages().len(), 1);
    bytes
}

fn config() -> ExtractionConfig {
    ExtractionConfig {
        disable_ocr: true,
        use_cache: true,
        use_layout_for_markdown: true,
        layout: Some(LayoutDetectionConfig::default()),
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn input() -> ExtractInput {
    ExtractInput::from_bytes(native_pdf(), "application/pdf", Some("selected.pdf".to_string()))
}

#[tokio::test]
async fn should_preserve_native_text_when_no_layout_pages_are_selected_gh2255() {
    let output = crate::extract_with_layout_pages(input(), &config(), Vec::new())
        .await
        .unwrap();
    assert_eq!(output.results.len(), 1);
    assert_eq!(output.results[0].content.matches(NATIVE_TEXT).count(), 1);
    assert_eq!(output.results[0].pages.as_ref().unwrap().len(), 1);
    assert_eq!(
        output.results[0].pages.as_ref().unwrap()[0]
            .content
            .matches(NATIVE_TEXT)
            .count(),
        1
    );
}

#[tokio::test]
async fn should_warn_once_per_invalid_page_without_losing_native_text_gh2255() {
    let output = crate::extract_with_layout_pages(input(), &config(), vec![0, 2, 2])
        .await
        .unwrap();
    assert_eq!(output.results.len(), 1);
    assert_eq!(output.results[0].content.matches(NATIVE_TEXT).count(), 1);
    let warnings: Vec<_> = output.results[0]
        .processing_warnings
        .iter()
        .filter(|warning| warning.source == "layout-pages")
        .map(|warning| warning.message.as_ref())
        .collect();
    assert_eq!(
        warnings,
        vec![
            "layout page 0 is outside the 1-based range 1..=1; ignored",
            "layout page 2 is outside the 1-based range 1..=1; ignored",
        ]
    );
}

#[tokio::test]
async fn should_reject_uri_input_without_silently_ignoring_layout_selection_gh2255() {
    let error = crate::extract_with_layout_pages(ExtractInput::from_uri("/not-read.pdf"), &config(), vec![1])
        .await
        .unwrap_err();
    assert!(error.to_string().contains("accepts one PDF bytes input"));
}

#[tokio::test]
async fn should_reject_non_pdf_bytes_without_ignoring_layout_selection_gh2255() {
    let error = crate::extract_with_layout_pages(
        ExtractInput::from_bytes(b"plain text".to_vec(), "text/plain", None),
        &config(),
        vec![1],
    )
    .await
    .unwrap_err();
    assert!(error.to_string().contains("accepts PDF input only"));
}

#[derive(Default)]
struct CountingCache {
    gets: AtomicUsize,
    puts: AtomicUsize,
}

#[async_trait::async_trait]
impl CacheBackend for CountingCache {
    async fn get(&self, _key: &str) -> Option<Vec<u8>> {
        self.gets.fetch_add(1, Ordering::SeqCst);
        let document = crate::ExtractedDocument {
            content: "WRONG CACHED DOCUMENT".to_string(),
            ..Default::default()
        };
        Some(serde_json::to_vec(&crate::ExtractionResult::single(document)).unwrap())
    }

    async fn put(&self, _key: &str, _value: Vec<u8>, _ttl: Option<std::time::Duration>) {
        self.puts.fetch_add(1, Ordering::SeqCst);
    }
}

#[tokio::test]
async fn should_bypass_the_engine_cache_after_resolving_input_overrides_gh2255() {
    let cache = Arc::new(CountingCache::default());
    let engine = Engine::builder().with_cache_backend(cache.clone()).build();
    let control = engine
        .extract(
            ExtractInput::from_bytes(b"cache wiring probe".to_vec(), "text/plain", None),
            &ExtractionConfig::default(),
        )
        .await
        .unwrap();
    assert_eq!(cache.gets.load(Ordering::SeqCst), 1);
    assert_eq!(control.results[0].content, "WRONG CACHED DOCUMENT");
    let mut request = input();
    request.config = Some(FileExtractionConfig {
        layout: Some(LayoutDetectionConfig::default()),
        ..Default::default()
    });
    let output = engine
        .extract_with_layout_pages(request, &config(), Vec::new())
        .await
        .unwrap();
    assert_eq!(output.results.len(), 1);
    assert_eq!(output.results[0].content.matches(NATIVE_TEXT).count(), 1);
    assert_eq!(cache.gets.load(Ordering::SeqCst), 1);
    assert_eq!(cache.puts.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn should_keep_concurrent_layout_selections_and_warnings_request_local_gh2255() {
    let engine = Engine::new_default();
    let left_config = config();
    let right_config = config();
    let (left, right) = tokio::join!(
        engine.extract_with_layout_pages(input(), &left_config, vec![0]),
        engine.extract_with_layout_pages(input(), &right_config, vec![2]),
    );
    for (output, invalid_page) in [(left.unwrap(), 0), (right.unwrap(), 2)] {
        assert_eq!(output.results.len(), 1);
        assert_eq!(output.results[0].content.matches(NATIVE_TEXT).count(), 1);
        let warnings: Vec<_> = output.results[0]
            .processing_warnings
            .iter()
            .filter(|warning| warning.source == "layout-pages")
            .collect();
        assert_eq!(warnings.len(), 1);
        assert_eq!(
            warnings[0].message,
            format!("layout page {invalid_page} is outside the 1-based range 1..=1; ignored")
        );
    }
}

struct PriorityPdfExtractor;

impl crate::plugins::Plugin for PriorityPdfExtractor {
    fn name(&self) -> &str {
        "priority-pdf"
    }
    fn version(&self) -> String {
        "1.0.0".to_string()
    }
    fn initialize(&self) -> crate::Result<()> {
        Ok(())
    }
    fn shutdown(&self) -> crate::Result<()> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl crate::plugins::DocumentExtractor for PriorityPdfExtractor {
    async fn extract(
        &self,
        _input: ExtractInput,
        _config: &ExtractionConfig,
    ) -> crate::Result<crate::ExtractedDocument> {
        Err(crate::XbergError::Other(
            "priority extractor must not be bypassed".to_string(),
        ))
    }
    fn supported_mime_types(&self) -> &[&str] {
        &["application/pdf"]
    }
    fn priority(&self) -> i32 {
        200
    }
}

#[test]
fn should_reject_unsupported_selection_on_the_highest_priority_extractor_gh2255() {
    let mut registry = crate::plugins::registry::DocumentExtractorRegistry::new();
    registry
        .register_internal_with_layout_pages(Arc::new(super::PdfExtractor::new()))
        .unwrap();
    registry.register(Arc::new(PriorityPdfExtractor)).unwrap();
    let selected = registry.get_registered("application/pdf").unwrap();
    assert_eq!(selected.plugin().name(), "priority-pdf");
    let pages = crate::core::layout_pages::LayoutPageSelection::new(vec![1]);
    let error = match selected.with_layout_pages(&pages) {
        Ok(_) => panic!("unsupported custom extractor must not silently receive an ignored selection"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("priority-pdf"));
    assert!(
        error
            .to_string()
            .contains("does not support explicit layout-page selection")
    );
    assert_eq!(
        registry.get_registered("application/pdf").unwrap().plugin().name(),
        "priority-pdf"
    );
}

#[tokio::test]
async fn should_accept_default_native_settings_for_explicit_layout_pages_gh2255() {
    let config = ExtractionConfig {
        disable_ocr: true,
        ..Default::default()
    };
    assert!(!config.use_layout_for_markdown);
    assert!(config.layout.is_none());
    let output = crate::extract_with_layout_pages(input(), &config, vec![0])
        .await
        .unwrap();
    assert_eq!(output.results.len(), 1);
    assert_eq!(output.results[0].content.matches(NATIVE_TEXT).count(), 1);
    let warnings: Vec<_> = output.results[0]
        .processing_warnings
        .iter()
        .filter(|warning| warning.source == "layout-pages")
        .collect();
    assert_eq!(warnings.len(), 1);
    assert_eq!(
        warnings[0].message,
        "layout page 0 is outside the 1-based range 1..=1; ignored"
    );
}

#[test]
fn should_bound_the_public_explicit_layout_future_size_gh2255() {
    const MAX_ENTRYPOINT_FUTURE_RATIO: usize = 2;
    let config = ExtractionConfig::default();
    let operation = crate::extract_with_layout_pages(input(), &config, vec![0]);
    let future_bytes = std::mem::size_of_val(&operation);
    let ordinary = crate::extract(input(), &config);
    let ordinary_bytes = std::mem::size_of_val(&ordinary);
    // ~keep Bound the new wrapper relative to the existing API, excluding the megabyte-scale native future.
    assert!(
        future_bytes <= ordinary_bytes * MAX_ENTRYPOINT_FUTURE_RATIO,
        "explicit-layout future uses {future_bytes} bytes versus ordinary extraction {ordinary_bytes}; keep native work boxed"
    );
}

#[tokio::test]
async fn region_text_opt_in_preserves_legacy_result_for_an_unselected_native_page() {
    let engine = Engine::new_default();
    let mut config = config();
    config.pdf_options.get_or_insert_with(Default::default).reading_order = true;
    let ordinary = engine
        .extract_with_layout_pages(input(), &config, vec![])
        .await
        .unwrap();
    let opted_in = engine
        .extract_with_layout_region_text(input(), &config, vec![])
        .await
        .unwrap();
    assert_eq!(ordinary.results.len(), 1);
    assert_eq!(
        serde_json::to_value(ordinary).unwrap(),
        serde_json::to_value(opted_in.extraction).unwrap()
    );
    assert!(opted_in.regions.is_empty());
}
