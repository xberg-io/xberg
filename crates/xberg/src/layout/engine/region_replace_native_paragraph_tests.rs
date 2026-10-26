//! A table region's VLM answer must replace the native body paragraphs inside it that it carries at least
//! 90% of the words of, and keep the others, end to end on a native PDF: stub layout engine, local VLM
//! stub, `extract_content`, markdown rendered from the result.
//!
//! Lives under `engine` because the stub model is installed in the private engine pool, which only
//! this module can construct an engine for; the VLM endpoint is a local wiremock stub.

use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;
use crate::core::config::acceleration::{AccelerationConfig, ExecutionProviderType};
use crate::core::config::layout::LayoutStrategy;
use crate::core::config::{
    ExtractionConfig, LayoutDetectionConfig, LlmConfig, OcrConfig, OutputFormat, TableModel, VlmFallbackPolicy,
};
use crate::layout::{BBox, LayoutClass, LayoutDetection};
use crate::plugins::InternalDocumentExtractor;

const PAGE_WIDTH: f32 = 612.0;
const PAGE_HEIGHT: f32 = 792.0;

/// A table region the stub engine reports, in PDF points (y up).
#[derive(Clone, Copy)]
struct Region {
    left: f32,
    bottom: f32,
    right: f32,
    top: f32,
    /// Keys the stub engine apart from any other pooled engine, so no other test can lease it.
    confidence_threshold: f32,
}

/// Holds the second and third paragraphs whole, and clears the first paragraph's last descender
/// (about 655) and the fourth's first ascender (about 492).
const COVERING: Region = Region {
    left: 60.0,
    bottom: 504.0,
    right: 560.0,
    top: 640.0,
    confidence_threshold: 0.41,
};

/// Blank page below the last paragraph, whose last descender is at about 367.
const BLANK: Region = Region {
    left: 60.0,
    bottom: 100.0,
    right: 560.0,
    top: 300.0,
    confidence_threshold: 0.42,
};

const HEADING: &str = "Quarterly Operations Review";
/// Body paragraphs, each with the baseline of its first line; lines sit 14 points apart.
const PARAGRAPHS: [(f32, [&str; 4]); 5] = [
    (
        700.0,
        [
            "The northern warehouse finished the quarter with steady staffing levels and no",
            "unplanned closures. Weekend shifts were filled from the regular roster, and the",
            "overtime budget stayed within the limit agreed at the start of the year by the",
            "regional managers.",
        ],
    ),
    (
        628.0,
        [
            "Inventory turnover improved for the third quarter in a row, driven mostly by",
            "faster replenishment of the fastest moving lines. Slow stock was reviewed in",
            "August and the oldest pallets were sold to a liquidator at a modest discount",
            "to free floor space.",
        ],
    ),
    (
        556.0,
        [
            "Freight costs rose sharply in September after two carriers revised their fuel",
            "surcharges. The team renegotiated one contract before the end of the month and",
            "expects the second to settle in October, which should return costs to the level",
            "seen earlier in the year.",
        ],
    ),
    (
        484.0,
        [
            "Customer complaints fell by a fifth compared with the same quarter last year.",
            "Most of the remaining complaints concern late deliveries to rural addresses,",
            "which the routing pilot planned for next quarter is meant to address directly",
            "for the affected postcodes.",
        ],
    ),
    (
        412.0,
        [
            "Next quarter the focus moves to the new packing line, which is due to start",
            "trial runs in November once the safety inspection has been completed and the",
            "operators have finished their training before the busy season begins in full.",
            "The site manager will report on progress at the next review.",
        ],
    ),
];
/// One phrase from each paragraph's first line, in order.
const PHRASES: [&str; 5] = [
    "steady staffing levels",
    "Inventory turnover improved",
    "Freight costs rose sharply",
    "Customer complaints fell",
    "new packing line",
];
/// The covered paragraph the stub answer transcribes; the other one the region covers is not in it.
const ANSWERED: usize = 1;

/// The answered paragraph's words in capitals, so the answer is told apart from the native text.
fn region_markdown() -> String {
    PARAGRAPHS[ANSWERED].1.join(" ").to_uppercase()
}

/// Reports one table region, scaled to the page raster.
struct TableRegionModel(Region);

impl LayoutModel for TableRegionModel {
    fn detect(&mut self, img: &RgbImage) -> Result<Vec<LayoutDetection>, LayoutError> {
        let (sx, sy) = (img.width() as f32 / PAGE_WIDTH, img.height() as f32 / PAGE_HEIGHT);
        let region = self.0;
        Ok(vec![LayoutDetection {
            class_name: LayoutClass::Table,
            confidence: 0.9,
            bbox: BBox {
                x1: region.left * sx,
                y1: (PAGE_HEIGHT - region.top) * sy,
                x2: region.right * sx,
                y2: (PAGE_HEIGHT - region.bottom) * sy,
            },
        }])
    }

    fn detect_with_threshold(&mut self, img: &RgbImage, _threshold: f32) -> Result<Vec<LayoutDetection>, LayoutError> {
        self.detect(img)
    }

    fn name(&self) -> &str {
        "table-region-stub"
    }
}

/// `Always`, so the prose page is not skipped by the auto gate; no table model, so nothing is
/// downloaded and the structure pipeline takes its heuristic table path.
fn layout_config(region: Region) -> LayoutDetectionConfig {
    LayoutDetectionConfig {
        strategy: LayoutStrategy::Always,
        confidence_threshold: Some(region.confidence_threshold),
        table_model: TableModel::Disabled,
        acceleration: Some(AccelerationConfig {
            provider: ExecutionProviderType::Cpu,
            device_id: 0,
        }),
        ..Default::default()
    }
}

/// Pools a stub engine matching `layout`, so the markdown layout pass leases it instead of loading RT-DETR.
fn install_stub_engine(layout: &LayoutDetectionConfig, region: Region) {
    let engine = LayoutEngine {
        model: Box::new(TableRegionModel(region)),
        config: crate::layout::config_from_extraction(layout),
        thread_budget: crate::core::config::concurrency::resolve_thread_budget(None).max(1),
    };
    let lease = crate::layout::CACHED_ENGINE
        .take_or_create_matching(|_| false, || Ok::<_, LayoutError>(engine))
        .expect("the stub engine is pooled");
    drop(lease);
}

fn stub_completion(content: &str) -> serde_json::Value {
    serde_json::json!({
        "id": "chatcmpl-test",
        "object": "chat.completion",
        "created": 0,
        "model": "openai/gpt-4o-mini",
        "choices": [{
            "index": 0,
            "message": { "role": "assistant", "content": content },
            "finish_reason": "stop"
        }],
        "usage": { "prompt_tokens": 10, "completion_tokens": 10, "total_tokens": 20 }
    })
}

/// Escapes a line for a PDF literal string.
fn pdf_literal(line: &str) -> String {
    line.replace('\\', "\\\\").replace('(', "\\(").replace(')', "\\)")
}

/// One native-text page: a bold heading over five body paragraphs of four lines each.
fn report_pdf() -> Vec<u8> {
    let mut stream = format!("BT /F2 16 Tf 72 730 Td ({}) Tj ET\n", pdf_literal(HEADING));
    for (first_baseline, lines) in PARAGRAPHS {
        for (index, line) in lines.iter().enumerate() {
            let baseline = first_baseline - 14.0 * index as f32;
            stream.push_str(&format!(
                "BT /F1 11 Tf 72 {baseline} Td ({}) Tj ET\n",
                pdf_literal(line)
            ));
        }
    }

    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets: Vec<usize> = Vec::new();
    let mut object = |pdf: &mut Vec<u8>, body: &[u8]| {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n", offsets.len()).as_bytes());
        pdf.extend_from_slice(body);
        pdf.extend_from_slice(b"\nendobj\n");
    };
    object(&mut pdf, b"<< /Type /Catalog /Pages 2 0 R >>");
    object(&mut pdf, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>");
    object(
        &mut pdf,
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R \
          /Resources << /Font << /F1 5 0 R /F2 6 0 R >> >> >>",
    );
    object(
        &mut pdf,
        format!("<< /Length {} >>\nstream\n{stream}endstream", stream.len()).as_bytes(),
    );
    object(
        &mut pdf,
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
    );
    object(
        &mut pdf,
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >>",
    );

    let xref_pos = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f\r\n", offsets.len() + 1).as_bytes());
    for offset in &offsets {
        pdf.extend_from_slice(format!("{offset:010} 00000 n\r\n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_pos}\n%%EOF\n",
            offsets.len() + 1
        )
        .as_bytes(),
    );
    pdf
}

fn config(server: &MockServer, region: Region, output_format: OutputFormat) -> ExtractionConfig {
    let layout = layout_config(region);
    install_stub_engine(&layout, region);
    ExtractionConfig {
        use_layout_for_markdown: true,
        disable_ocr: true,
        output_format,
        layout: Some(layout),
        ocr: Some(OcrConfig {
            vlm_fallback: VlmFallbackPolicy::OnLowQuality { quality_threshold: 0.5 },
            vlm_config: Some(LlmConfig {
                model: "openai/gpt-4o-mini".to_string(),
                api_key: Some("test-key".to_string()),
                base_url: Some(server.uri()),
                timeout_secs: Some(10),
                max_retries: Some(0),
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    }
}

async fn stub_vlm() -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(stub_completion(&region_markdown())))
        .mount(&server)
        .await;
    server
}

/// Extracts the report and renders it to markdown.
async fn extract_markdown(config: &ExtractionConfig) -> String {
    let doc = crate::extractors::PdfExtractor::new()
        .extract_content(&report_pdf(), "application/pdf", config)
        .await
        .expect("extraction must succeed");
    crate::rendering::render_markdown(&doc)
}

/// The answer once, the answered paragraph gone, the heading and every other paragraph kept.
fn assert_region_replaced(markdown: &str) {
    assert_eq!(
        markdown.matches(region_markdown().as_str()).count(),
        1,
        "the VLM answer must appear exactly once:\n{markdown}"
    );
    for (index, phrase) in PHRASES.iter().enumerate() {
        if index == ANSWERED {
            assert!(
                !markdown.contains(phrase),
                "paragraph {index} lies inside the table region and the answer carries it, so it must go:\n{markdown}"
            );
        } else {
            assert!(
                markdown.contains(phrase),
                "paragraph {index} lies outside the region or is not in the answer, so it must be kept:\n{markdown}"
            );
        }
    }
    assert!(markdown.contains(HEADING), "the heading must be kept:\n{markdown}");
}

#[tokio::test]
#[serial_test::serial]
async fn table_region_answer_replaces_native_paragraphs_in_markdown() {
    let server = stub_vlm().await;
    let markdown = extract_markdown(&config(&server, COVERING, OutputFormat::Markdown)).await;
    let requests = server.received_requests().await.expect("wiremock records requests");
    assert_eq!(
        requests.len(),
        1,
        "the table detection must reach one VLM region request"
    );
    assert_region_replaced(&markdown);
}

#[tokio::test]
#[serial_test::serial]
async fn table_region_over_blank_space_removes_no_text() {
    let server = stub_vlm().await;
    let markdown = extract_markdown(&config(&server, BLANK, OutputFormat::Markdown)).await;
    assert_eq!(
        server.received_requests().await.map(|requests| requests.len()),
        Some(1),
        "the blank region must still reach the VLM"
    );
    assert_eq!(markdown.matches(region_markdown().as_str()).count(), 1, "{markdown}");
    for phrase in PHRASES.iter().chain([&HEADING]) {
        assert!(
            markdown.contains(phrase),
            "no text lies in the region, so none may go:\n{markdown}"
        );
    }
}
