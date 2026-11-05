#![cfg(all(feature = "pdf", feature = "ocr-pipeline"))]

mod helpers;

use async_trait::async_trait;
use lopdf::{Document, Object, ObjectId, Stream, dictionary};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use xberg::ExtractedDocument;
use xberg::core::config::{
    ExtractionConfig, OcrConfig, OcrPipelineConfig, OcrQualityThresholds, OutputFormat, PageConfig,
};
use xberg::plugins::{OcrBackend, OcrBackendType, Plugin, register_ocr_backend, unregister_ocr_backend};
use xberg::types::document_structure::{DocumentNode, DocumentStructure, NodeContent};

const BACKEND: &str = "type0-unicode-recording";
const NATIVE: &str = "Reliable native source words remain available beside the missing character boundary.";
const OCR: &str = "qq ww ee rr tt yy uu ii oo pp ss dd ff gg hh jj kk ll zz xx OCR\u{FFFD}TOKEN";

#[derive(Clone, Copy)]
enum Fixture {
    Retained,
    Boundaries,
    AllLost,
    Mixed,
}

fn type0_font(pdf: &mut Document) -> ObjectId {
    let cmap = b"begincmap /CMapName /LossProbe def /CMapType 2 def \
        1 begincodespacerange <0000> <FFFF> endcodespacerange \
        5 beginbfchar <0041> <0041> <0042> <0042> <0031> <0031> <0032> <0032> <0044> <0000> \
        endbfchar endcmap";
    let unicode = pdf.add_object(Stream::new(dictionary! {}, cmap.to_vec()));
    let descriptor = pdf.add_object(dictionary! {
        "Type" => "FontDescriptor", "FontName" => "ABCDEF+LossProbe", "Flags" => 4,
        "FontBBox" => vec![0.into(), (-200).into(), 1000.into(), 800.into()],
        "ItalicAngle" => 0, "Ascent" => 800, "Descent" => -200, "CapHeight" => 700, "StemV" => 80,
    });
    let cid = pdf.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "CIDFontType2", "BaseFont" => "ABCDEF+LossProbe",
        "CIDSystemInfo" => dictionary! {"Registry" => Object::string_literal("Adobe"),
            "Ordering" => Object::string_literal("Identity"), "Supplement" => 0},
        "FontDescriptor" => descriptor, "DW" => 600, "CIDToGIDMap" => "Identity",
    });
    pdf.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type0", "BaseFont" => "ABCDEF+LossProbe",
        "Encoding" => "Identity-H", "DescendantFonts" => vec![cid.into()], "ToUnicode" => unicode,
    })
}

fn source_pdf(fixture: Fixture) -> Vec<u8> {
    let mut pdf = Document::with_version("1.7");
    let pages = pdf.new_object_id();
    let ordinary = pdf.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica", "Encoding" => "WinAnsiEncoding",
    });
    let type0 = type0_font(&mut pdf);
    let mut text = String::from("q 0.3 G 3 w 30 30 m 250 30 l S Q\nBT /F1 10 Tf 30 220 Td ");
    if !matches!(fixture, Fixture::AllLost) {
        text.push_str(&format!("({NATIVE}) Tj 0 -24 Td "));
    }
    text.push_str("/F2 12 Tf ");
    text.push_str(match fixture {
        Fixture::Retained | Fixture::Mixed => "<004100440042> Tj ET",
        Fixture::Boundaries => "<004100440042> Tj 0 -24 Td <003100440032> Tj ET",
        Fixture::AllLost => "<0044004400440044004400440044> Tj ET",
    });
    let stream = pdf.add_object(Stream::new(dictionary! {}, text.into_bytes()));
    let page = pdf.add_object(dictionary! {
        "Type" => "Page", "Parent" => pages, "MediaBox" => vec![0.into(), 0.into(), 600.into(), 300.into()],
        "Contents" => stream, "Resources" => dictionary! {"Font" => dictionary! {"F1" => ordinary, "F2" => type0}},
    });
    pdf.objects.insert(
        pages,
        Object::Dictionary(dictionary! {
            "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1,
        }),
    );
    if matches!(fixture, Fixture::Mixed) {
        let second = pdf.add_object(pdf.get_object(page).unwrap().clone());
        let dictionary = pdf.get_object_mut(pages).unwrap().as_dict_mut().unwrap();
        dictionary.set("Kids", vec![Object::Reference(page), Object::Reference(second)]);
        dictionary.set("Count", 2);
    }
    let root = pdf.add_object(dictionary! {"Type" => "Catalog", "Pages" => pages});
    pdf.trailer.set("Root", root);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    bytes
}

fn source_with_marker_oracle(fixture: Fixture, expected: usize) -> Vec<u8> {
    let bytes = source_pdf(fixture);
    let native = xberg_native_pdf::PdfDocument::from_bytes(bytes.clone()).unwrap();
    let pages = if matches!(fixture, Fixture::Mixed) { 2 } else { 1 };
    assert_eq!(native.page_count().unwrap(), pages);
    let text: String = (0..pages)
        .flat_map(|page| native.extract_spans(page).unwrap())
        .map(|span| span.text)
        .collect();
    assert_eq!(
        text.matches('\u{FFFD}').count(),
        expected,
        "source glyph oracle: {text:?}"
    );
    bytes
}

struct RecordingBackend {
    fail: bool,
    calls: AtomicUsize,
}

impl Plugin for RecordingBackend {
    fn name(&self) -> &str {
        BACKEND
    }
    fn version(&self) -> String {
        "1.0.0".into()
    }
    fn initialize(&self) -> xberg::Result<()> {
        Ok(())
    }
    fn shutdown(&self) -> xberg::Result<()> {
        Ok(())
    }
}

#[async_trait]
impl OcrBackend for RecordingBackend {
    async fn process_image(&self, image: &[u8], _config: &OcrConfig) -> xberg::Result<ExtractedDocument> {
        assert!(
            image.starts_with(b"\x89PNG\r\n\x1a\n"),
            "the real PDF render must reach OCR"
        );
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.fail {
            return Err(xberg::XbergError::Ocr {
                message: "recorded missing-glyph OCR failure".into(),
                source: None,
            });
        }
        let mut result = ExtractedDocument::default();
        result.content = OCR.into();
        result.document = Some(DocumentStructure {
            nodes: vec![DocumentNode {
                id: String::new(),
                content: NodeContent::Paragraph { text: OCR.into() },
                parent: None,
                children: Vec::new(),
                content_layer: Default::default(),
                page: Some(1),
                page_end: None,
                bbox: None,
                annotations: Vec::new(),
                attributes: None,
            }],
            ..Default::default()
        });
        Ok(result)
    }
    fn supports_language(&self, _language: &str) -> bool {
        true
    }
    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }
}

fn config(format: OutputFormat) -> ExtractionConfig {
    let thresholds = OcrQualityThresholds {
        min_total_non_whitespace: 10_000,
        min_non_whitespace_per_page: 10_000.0,
        pipeline_min_quality: 0.0,
        ..Default::default()
    };
    ExtractionConfig {
        output_format: format,
        use_cache: false,
        include_document_structure: true,
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ocr: Some(OcrConfig {
            backend: BACKEND.into(),
            quality_thresholds: Some(thresholds.clone()),
            pipeline: Some(OcrPipelineConfig {
                stages: vec![serde_json::from_value(serde_json::json!({"backend": BACKEND})).unwrap()],
                quality_thresholds: thresholds,
            }),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn extract(bytes: &[u8], config: &ExtractionConfig, fail: bool) -> (xberg::Result<ExtractedDocument>, usize) {
    let backend = Arc::new(RecordingBackend {
        fail,
        calls: AtomicUsize::new(0),
    });
    register_ocr_backend(backend.clone()).unwrap();
    let result = helpers::extract_bytes_document_blocking(bytes, "application/pdf", config);
    unregister_ocr_backend(BACKEND).unwrap();
    (result, backend.calls.load(Ordering::SeqCst))
}

fn assert_retained_native(format: OutputFormat) {
    let source = source_with_marker_oracle(Fixture::Retained, 1);
    let (result, calls) = extract(&source, &config(format), false);
    let result = result.unwrap();
    assert!(calls > 0, "actual custom OCR must execute");
    assert!(result.content.contains(NATIVE), "{result:?}");
    assert!(
        result.content.contains("A B"),
        "native unknown boundary must remain: {result:?}"
    );
    assert_eq!(
        result.content.matches('\u{FFFD}').count(),
        1,
        "only the genuine OCR marker remains: {result:?}"
    );
    assert!(result.content.contains("OCR\u{FFFD}TOKEN"), "{result:?}");
    assert!(
        result
            .processing_warnings
            .iter()
            .any(|warning| warning.source == "pdf" && warning.message.contains("no Unicode")),
        "{result:?}"
    );
    let document = result.document.unwrap();
    let text: Vec<_> = document
        .nodes
        .iter()
        .filter_map(|node| match &node.content {
            NodeContent::Paragraph { text } | NodeContent::Heading { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert!(text.iter().any(|text| text.contains("A B")), "{text:?}");
    assert_eq!(
        text.iter().map(|text| text.matches('\u{FFFD}').count()).sum::<usize>(),
        1,
        "{text:?}"
    );
}

#[test]
#[serial_test::serial]
fn retained_native_markers_are_cleaned_before_plain_ocr_mixing() {
    assert_retained_native(OutputFormat::Plain);
}

#[test]
#[serial_test::serial]
fn retained_native_markers_are_cleaned_before_structured_ocr_mixing() {
    assert_retained_native(OutputFormat::Markdown);
}

#[test]
#[serial_test::serial]
fn genuine_ocr_replacement_characters_survive_native_cleanup() {
    let source = source_with_marker_oracle(Fixture::Retained, 1);
    let mut config = config(OutputFormat::Plain);
    config.force_ocr = true;
    let (result, calls) = extract(&source, &config, false);
    let result = result.unwrap();
    assert!(calls > 0);
    assert!(result.content.contains("OCR\u{FFFD}TOKEN"), "{result:?}");
    assert_eq!(result.content.matches('\u{FFFD}').count(), 1);
    assert!(
        !result
            .processing_warnings
            .iter()
            .any(|warning| warning.message.contains("no Unicode"))
    );
}

#[test]
#[serial_test::serial]
fn an_all_unmapped_page_with_failed_ocr_is_not_a_blank_success() {
    let source = source_with_marker_oracle(Fixture::AllLost, 7);
    let (result, calls) = extract(&source, &config(OutputFormat::Plain), true);
    assert!(calls > 0, "the failing backend must actually run");
    assert!(
        result.is_err(),
        "all native glyphs were lost and OCR failed: {result:?}"
    );
}

#[test]
#[serial_test::serial]
fn native_unknown_boundaries_do_not_fuse_words_or_digits_with_ocr_disabled() {
    let source = source_with_marker_oracle(Fixture::Boundaries, 2);
    let mut config = config(OutputFormat::Plain);
    config.disable_ocr = true;
    let (result, calls) = extract(&source, &config, false);
    let result = result.unwrap();
    assert_eq!(calls, 0);
    assert!(result.content.contains("A B"), "{result:?}");
    assert!(result.content.contains("1 2"), "{result:?}");
    assert!(!result.content.contains('\u{FFFD}'));
    assert!(
        result
            .processing_warnings
            .iter()
            .any(|warning| warning.message.contains("no Unicode"))
    );
}

#[test]
#[serial_test::serial]
fn mixed_pages_clean_native_page_but_keep_the_ocr_marker() {
    let source = source_with_marker_oracle(Fixture::Mixed, 2);
    let mut config = config(OutputFormat::Markdown);
    config.force_ocr_pages = Some(vec![1]);
    let (result, calls) = extract(&source, &config, false);
    let result = result.unwrap();
    assert_eq!(calls, 1, "only the explicitly selected page reaches OCR");
    assert_eq!(result.content.matches('\u{FFFD}').count(), 1, "{result:?}");
    let pages = result.pages.as_ref().unwrap();
    assert_eq!(pages.len(), 2);
    assert!(
        !pages[0].content.contains(NATIVE),
        "explicitly replaced page: {result:?}"
    );
    assert!(pages[0].content.contains("OCR\u{FFFD}TOKEN"), "{result:?}");
    assert!(pages[1].content.contains("A B"), "{result:?}");
    assert!(!pages[1].content.contains('\u{FFFD}'), "{result:?}");
    assert!(
        result
            .processing_warnings
            .iter()
            .any(|warning| warning.source == "pdf" && warning.message.contains("no Unicode")),
        "{result:?}"
    );
}
