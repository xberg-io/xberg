//! `LayoutDetectionConfig::pages` end to end: the listed pages, and only they, reach the layout
//! model on the markdown and OCR routes, and metadata records why for every page.
//!
//! Lives under `engine` because the stub model is installed in the private engine pool.

use std::sync::{Arc, Mutex};

use super::*;
use crate::core::config::acceleration::{AccelerationConfig, ExecutionProviderType};
use crate::core::config::layout::LayoutStrategy;
use crate::core::config::{ExtractionConfig, LayoutDetectionConfig};
use crate::layout::LayoutDetection;
use crate::plugins::InternalDocumentExtractor;

/// Raster widths at the default 150 dpi of the fixture's 300, 612 and 500 pt wide pages.
const PAGE_WIDTHS_PX: [u32; 3] = [625, 1275, 1042];

/// Records the width of every raster it is shown, so a test can tell which pages it ran on.
struct WidthRecordingModel {
    widths: Arc<Mutex<Vec<u32>>>,
}

impl LayoutModel for WidthRecordingModel {
    fn detect(&mut self, img: &RgbImage) -> Result<Vec<LayoutDetection>, LayoutError> {
        self.widths.lock().expect("widths lock").push(img.width());
        Ok(Vec::new())
    }

    fn detect_with_threshold(&mut self, img: &RgbImage, _threshold: f32) -> Result<Vec<LayoutDetection>, LayoutError> {
        self.detect(img)
    }

    fn name(&self) -> &str {
        "width-recording-stub"
    }
}

/// The confidence threshold keys each test's stub apart from every other pooled engine.
fn layout_config(confidence_threshold: f32) -> LayoutDetectionConfig {
    LayoutDetectionConfig {
        strategy: LayoutStrategy::Auto,
        pages: Some(vec![3, 2, 3, 0, 9]),
        confidence_threshold: Some(confidence_threshold),
        acceleration: Some(AccelerationConfig {
            provider: ExecutionProviderType::Cpu,
            device_id: 0,
        }),
        ..Default::default()
    }
}

fn install_stub_engine(layout: &LayoutDetectionConfig) -> Arc<Mutex<Vec<u32>>> {
    let widths = Arc::new(Mutex::new(Vec::new()));
    let engine = LayoutEngine {
        model: Box::new(WidthRecordingModel {
            widths: Arc::clone(&widths),
        }),
        config: crate::layout::config_from_extraction(layout),
        thread_budget: crate::core::config::concurrency::resolve_thread_budget(None).max(1),
    };
    let lease = crate::layout::CACHED_ENGINE
        .take_or_create_matching(|_| false, || Ok::<_, LayoutError>(engine))
        .expect("the stub engine is pooled");
    drop(lease);
    widths
}

/// Page 1 is blank and page 3 is blank, which `auto` alone runs; page 2 is single column prose,
/// which `auto` alone skips. Each page has its own width.
fn blank_prose_blank_pdf() -> Vec<u8> {
    use lopdf::{Document, Object, Stream, dictionary};

    let mut document = Document::with_version("1.5");
    let pages_id = document.new_object_id();
    let font_id = document.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
    });
    let mut prose = String::from("BT /F1 10 Tf 72 720 Td 14 TL\n");
    for line in 0..45 {
        prose.push_str(&format!(
            "(Line {line} of steady single column prose with enough characters to pass the floor.) Tj T*\n"
        ));
    }
    prose.push_str("ET");
    let mut kids: Vec<Object> = Vec::new();
    for (width, content) in [(300, String::new()), (612, prose), (500, String::new())] {
        let content_id = document.add_object(Stream::new(dictionary! {}, content.into_bytes()));
        let page_id = document.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "MediaBox" => vec![0.into(), 0.into(), width.into(), 792.into()],
            "Resources" => dictionary! { "Font" => dictionary! { "F1" => font_id } },
            "Contents" => content_id,
        });
        kids.push(page_id.into());
    }
    document.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => kids,
            "Count" => 3,
        }),
    );
    let catalog_id = document.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    document.trailer.set("Root", catalog_id);

    let mut bytes = Vec::new();
    document.save_to(&mut bytes).expect("fixture PDF must serialize");
    bytes
}

async fn extract_and_assert_listed_pages(config: ExtractionConfig, widths: &Mutex<Vec<u32>>) {
    let doc = crate::extractors::PdfExtractor::new()
        .extract_content(&blank_prose_blank_pdf(), "application/pdf", &config)
        .await
        .expect("extraction must succeed");

    let mut inferred = widths.lock().expect("widths lock").clone();
    inferred.sort_unstable();
    assert_eq!(
        inferred,
        vec![PAGE_WIDTHS_PX[2], PAGE_WIDTHS_PX[1]],
        "only listed pages 2 and 3 may reach the model, each once"
    );
    let Some(crate::types::FormatMetadata::Pdf(pdf_metadata)) = &doc.metadata.format else {
        panic!("a PDF carries PDF metadata");
    };
    assert_eq!(pdf_metadata.layout_gated_pages, Some(vec![1]));
    assert_eq!(
        pdf_metadata.layout_gate_reasons,
        Some(vec![
            "unlisted_page".to_string(),
            "listed_page".to_string(),
            "listed_page".to_string()
        ])
    );
}

#[tokio::test]
#[serial_test::serial]
async fn page_list_decides_layout_pages_on_the_markdown_route() {
    let layout = layout_config(0.43);
    let widths = install_stub_engine(&layout);
    let config = ExtractionConfig {
        use_layout_for_markdown: true,
        disable_ocr: true,
        layout: Some(layout),
        ..Default::default()
    };
    extract_and_assert_listed_pages(config, &widths).await;
}

#[cfg(feature = "ocr")]
#[tokio::test]
#[serial_test::serial]
async fn page_list_decides_layout_pages_on_the_ocr_route() {
    let layout = layout_config(0.47);
    let widths = install_stub_engine(&layout);
    let config = ExtractionConfig {
        force_ocr: true,
        layout: Some(layout),
        ..Default::default()
    };
    extract_and_assert_listed_pages(config, &widths).await;
}
