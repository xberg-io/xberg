use super::*;
use crate::pdf::structure::types::{PdfLine, PdfParagraph};
use lopdf::{Document, Object, Stream, dictionary};
use std::cell::{Cell, RefCell};

thread_local! {
    static OPEN_ROUTINES: Cell<usize> = const { Cell::new(0) };
    static CANCEL_ON_OPEN: RefCell<Option<crate::cancellation::CancellationToken>> = const { RefCell::new(None) };
}
pub(super) fn record_open() {
    OPEN_ROUTINES.with(|count| count.set(count.get() + 1));
    CANCEL_ON_OPEN.with(|token| {
        if let Some(token) = token.borrow_mut().take() {
            token.cancel();
        }
    });
}
fn opens() -> usize {
    OPEN_ROUTINES.with(Cell::get)
}

const SOURCE: [&str; 2] = [
    "The orchard committee recorded exactly 100 shipments for the northern market this morning.",
    "The harbour committee recorded exactly 100 arrivals for the southern market this evening.",
];

fn source_pdf() -> Vec<u8> {
    let mut pdf = Document::with_version("1.7");
    let pages = pdf.new_object_id();
    let font = pdf.add_object(dictionary! {"Type"=>"Font", "Subtype"=>"Type1", "BaseFont"=>"Helvetica"});
    let mut children = Vec::<Object>::new();
    for text in SOURCE {
        let stream = pdf.add_object(Stream::new(
            dictionary! {},
            format!("BT /F1 10 Tf 30 100 Td ({text}) Tj ET").into_bytes(),
        ));
        let page = pdf.add_object(dictionary! {"Type"=>"Page", "Parent"=>pages,
        "MediaBox"=>vec![0.into(),0.into(),600.into(),200.into()], "Contents"=>stream,
        "Resources"=>dictionary! {"Font"=>dictionary! {"F1"=>font}}});
        children.push(page.into());
    }
    pdf.objects.insert(
        pages,
        Object::Dictionary(dictionary! {"Type"=>"Pages", "Kids"=>children, "Count"=>2}),
    );
    let root = pdf.add_object(dictionary! {"Type"=>"Catalog", "Pages"=>pages});
    pdf.trailer.set("Root", root);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    bytes
}

fn source_pages(bytes: &[u8]) -> Vec<RunPageParagraphs> {
    use crate::pdf::native::hierarchy::{ArtifactRetention, extract_segments_from_page};
    let mut native = crate::pdf::native::NativeDocument::open_bytes_with_passwords(bytes, &[]).unwrap();
    assert_eq!(native.doc.page_count().unwrap(), 2);
    (0..2)
        .map(|index| {
            let mut segments =
                extract_segments_from_page(&mut native, index, ArtifactRetention::from_content_filter(None)).unwrap();
            assert_eq!(
                segments
                    .iter()
                    .map(|segment| segment.text.as_str())
                    .collect::<Vec<_>>()
                    .join(" "),
                SOURCE[index]
            );
            for segment in &mut segments {
                segment.text = segment.text.replace("100", "1OO");
            }
            let line = PdfLine {
                segments,
                native_scripts: vec![],
                baseline_y: 100.,
                dominant_font_size: 10.,
                is_bold: false,
                is_monospace: false,
            };
            RunPageParagraphs {
                paragraphs: vec![PdfParagraph {
                    text: String::new(),
                    lines: vec![line],
                    dominant_font_size: 10.,
                    heading_level: None,
                    is_bold: false,
                    is_list_item: false,
                    is_code_block: false,
                    is_formula: false,
                    is_page_furniture: false,
                    layout_class: None,
                    layout_region_path: None,
                    caption_for: None,
                    block_bbox: None,
                    word_count: SOURCE[index].split_whitespace().count(),
                }],
                ocr_scale: Some((1., 1.)),
            }
        })
        .collect()
}

fn run_merge(config: &ExtractionConfig, verdicts: &[OcrPageNoiseVerdict], empty_paragraphs: bool) -> usize {
    let bytes = source_pdf();
    let mut pages = source_pages(&bytes);
    if empty_paragraphs {
        for page in &mut pages {
            page.paragraphs.clear();
        }
    }
    let layers = SOURCE
        .iter()
        .enumerate()
        .map(|(index, text)| ((index + 1) as u32, text.to_string()))
        .collect();
    let read: Vec<_> = SOURCE.iter().map(|text| text.replace("100", "1OO")).collect();
    OPEN_ROUTINES.with(|count| count.set(0));
    let _result = with_run_layers_first(
        Some(std::sync::Arc::new(layers)),
        RunLayerSources {
            verdicts,
            page_index_offset: 0,
            content: Some(&bytes),
            raw_page_paragraphs: &pages,
        },
        read.join("\n\n"),
        read,
        None,
        config,
    );
    opens()
}

#[test]
fn should_open_and_authenticate_once_for_two_page_geometry_merge() {
    assert_eq!(run_merge(&ExtractionConfig::default(), &[], false), 1);
}

#[test]
fn should_not_open_geometry_after_cancellation() {
    let token = crate::cancellation::CancellationToken::new();
    token.cancel();
    let config = ExtractionConfig {
        cancel_token: Some(token),
        ..Default::default()
    };
    assert_eq!(run_merge(&config, &[], false), 0);
}

#[test]
fn should_not_open_geometry_without_ocr_lines() {
    assert_eq!(run_merge(&ExtractionConfig::default(), &[], true), 0);
}

#[test]
fn should_not_open_geometry_for_noise_only_pages() {
    let verdicts: Vec<_> = (0..2)
        .map(|page_index| OcrPageNoiseVerdict {
            page_index,
            low_confidence: true,
            fragmented_noise: false,
            dictionary_noise: false,
            fragmented_word_ratio: 0.,
            word_count: 14,
            mean_confidence: Some(10.),
            dict_invalid_word_ratio: None,
            discarded: false,
        })
        .collect();
    assert_eq!(run_merge(&ExtractionConfig::default(), &verdicts, false), 0);
}

#[test]
fn should_reuse_geometry_without_mixing_two_pages() {
    let bytes = source_pdf();
    let pages = source_pages(&bytes);
    let config = ExtractionConfig::default();
    let mut session = MergeGeometrySession::new(&bytes, &config);
    OPEN_ROUTINES.with(|count| count.set(0));
    let first = session
        .page_boxes(1, Some(&pages[0].paragraphs), pages[0].ocr_scale)
        .unwrap();
    let second = session
        .page_boxes(2, Some(&pages[1].paragraphs), pages[1].ocr_scale)
        .unwrap();
    assert_eq!(first.layer.len(), 1);
    assert_eq!(second.layer.len(), 1);
    assert_eq!(first.layer[0].key, SOURCE[0].to_lowercase());
    assert_eq!(second.layer[0].key, SOURCE[1].to_lowercase());
    assert!(first.ocr[0].key.contains("1oo"));
    assert!(second.ocr[0].key.contains("1oo"));
    assert_eq!(opens(), 1);
}

#[test]
fn should_stop_geometry_when_cancelled_between_pages() {
    let bytes = source_pdf();
    let pages = source_pages(&bytes);
    let token = crate::cancellation::CancellationToken::new();
    let config = ExtractionConfig {
        cancel_token: Some(token.clone()),
        ..Default::default()
    };
    let mut session = MergeGeometrySession::new(&bytes, &config);
    OPEN_ROUTINES.with(|count| count.set(0));
    assert!(
        session
            .page_boxes(1, Some(&pages[0].paragraphs), pages[0].ocr_scale)
            .is_some()
    );
    token.cancel();
    assert!(
        session
            .page_boxes(2, Some(&pages[1].paragraphs), pages[1].ocr_scale)
            .is_none()
    );
    assert_eq!(opens(), 1);
}

#[test]
fn should_cache_invalid_pdf_failure_without_reopening_for_other_pages() {
    let bytes = source_pdf();
    let pages = source_pages(&bytes);
    let config = ExtractionConfig::default();
    let mut session = MergeGeometrySession::new(b"not a PDF", &config);
    OPEN_ROUTINES.with(|count| count.set(0));
    for (index, page) in pages.iter().enumerate() {
        assert!(
            session
                .page_boxes((index + 1) as u32, Some(&page.paragraphs), page.ocr_scale)
                .is_none()
        );
    }
    assert_eq!(opens(), 1);
}

#[test]
fn should_not_open_geometry_without_a_known_raster_frame() {
    let bytes = source_pdf();
    let pages = source_pages(&bytes);
    let config = ExtractionConfig::default();
    let mut session = MergeGeometrySession::new(&bytes, &config);
    OPEN_ROUTINES.with(|count| count.set(0));
    assert!(session.page_boxes(1, Some(&pages[0].paragraphs), None).is_none());
    assert_eq!(opens(), 0);
}

fn password_fixture() -> Vec<u8> {
    use lopdf::{EncryptionState, EncryptionVersion, Permissions};
    let mut pdf = Document::load_mem(&source_pdf()).unwrap();
    pdf.trailer.set(
        "ID",
        vec![
            Object::string_literal("geometry fixture"),
            Object::string_literal("geometry fixture"),
        ],
    );
    let state = EncryptionState::try_from(EncryptionVersion::V2 {
        document: &pdf,
        owner_password: "synthetic-owner",
        user_password: "synthetic-user",
        key_length: 128,
        permissions: Permissions::default(),
    })
    .unwrap();
    pdf.encrypt(&state).unwrap();
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    bytes
}

#[test]
fn should_authenticate_encrypted_geometry_once_with_configured_passwords() {
    use crate::core::config::PdfConfig;
    let source = source_pdf();
    let pages = source_pages(&source);
    let bytes = password_fixture();
    let config = ExtractionConfig {
        pdf_options: Some(PdfConfig {
            passwords: Some(vec!["wrong-synthetic-password".into(), "synthetic-user".into()]),
            ..Default::default()
        }),
        ..Default::default()
    };
    let mut session = MergeGeometrySession::new(&bytes, &config);
    OPEN_ROUTINES.with(|count| count.set(0));
    for (index, page) in pages.iter().enumerate() {
        let boxes = session
            .page_boxes((index + 1) as u32, Some(&page.paragraphs), page.ocr_scale)
            .unwrap();
        assert_eq!(boxes.layer[0].key, SOURCE[index].to_lowercase());
    }
    assert_eq!(
        opens(),
        1,
        "one opening/authentication routine, including configured password fallback"
    );
}

#[test]
fn should_cache_authentication_failure_without_geometry_or_retries() {
    let source = source_pdf();
    let pages = source_pages(&source);
    let bytes = password_fixture();
    let config = ExtractionConfig::default();
    let mut session = MergeGeometrySession::new(&bytes, &config);
    OPEN_ROUTINES.with(|count| count.set(0));
    for (index, page) in pages.iter().enumerate() {
        assert!(
            session
                .page_boxes((index + 1) as u32, Some(&page.paragraphs), page.ocr_scale)
                .is_none()
        );
    }
    assert_eq!(opens(), 1);
}

struct GeometryBackend(std::sync::atomic::AtomicUsize);
impl crate::plugins::Plugin for GeometryBackend {
    fn name(&self) -> &str {
        "geometry-cancellation-control"
    }
    fn version(&self) -> String {
        "1".into()
    }
    fn initialize(&self) -> crate::Result<()> {
        Ok(())
    }
    fn shutdown(&self) -> crate::Result<()> {
        Ok(())
    }
}
#[async_trait::async_trait]
impl crate::plugins::OcrBackend for GeometryBackend {
    fn supports_language(&self, _: &str) -> bool {
        true
    }
    fn backend_type(&self) -> crate::plugins::OcrBackendType {
        crate::plugins::OcrBackendType::Custom
    }
    async fn process_image(
        &self,
        bytes: &[u8],
        _: &crate::core::config::OcrConfig,
    ) -> crate::Result<crate::types::ExtractedDocument> {
        use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
        let image = image::load_from_memory(bytes).unwrap();
        let index = self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        assert!(index < SOURCE.len());
        let text = SOURCE[index].replace("100", "1OO");
        let mut doc = InternalDocument::new("image/png");
        let mut element = InternalElement::text(
            ElementKind::OcrText {
                level: crate::types::OcrElementLevel::Line,
            },
            &text,
            0,
        );
        let (sx, sy) = (f64::from(image.width()) / 600., f64::from(image.height()) / 200.);
        element.bbox = Some(crate::types::BoundingBox {
            x0: 30. * sx,
            y0: 90. * sy,
            x1: 500. * sx,
            y1: 100. * sy,
        });
        doc.push_element(element);
        Ok(crate::types::ExtractedDocument {
            content: text,
            ocr_internal_document: Some(doc),
            ..Default::default()
        })
    }
}

#[tokio::test]
#[serial_test::serial]
async fn should_return_cancelled_when_cancellation_arrives_during_native_geometry() {
    use crate::plugins::{register_ocr_backend, unregister_ocr_backend};
    let bytes = source_pdf();
    let token = crate::cancellation::CancellationToken::new();
    let backend = std::sync::Arc::new(GeometryBackend(std::sync::atomic::AtomicUsize::new(0)));
    register_ocr_backend(backend.clone()).unwrap();
    let config = async_geometry_config(&token);
    let hints = geometry_page_hints();
    OPEN_ROUTINES.with(|count| count.set(0));
    CANCEL_ON_OPEN.with(|slot| *slot.borrow_mut() = Some(token.clone()));
    let result = extract_with_ocr_with_page_hints(
        Some(&bytes),
        None,
        #[cfg(feature = "layout-detection")]
        None,
        &config,
        None,
        Some(hints),
    )
    .await;
    CANCEL_ON_OPEN.with(|slot| {
        slot.borrow_mut().take();
    });
    unregister_ocr_backend("geometry-cancellation-control").unwrap();
    assert_eq!(
        backend.0.load(std::sync::atomic::Ordering::SeqCst),
        2,
        "both real PDF rasters reach OCR"
    );
    assert_eq!(
        opens(),
        1,
        "cancellation starts at the native geometry opening boundary"
    );
    assert!(token.is_cancelled());
    assert!(
        matches!(
            result,
            Err(FailedOcrRun {
                error: crate::XbergError::Cancelled,
                ..
            })
        ),
        "cancelled geometry must not return success: {result:?}"
    );
}

fn async_geometry_config(token: &crate::cancellation::CancellationToken) -> ExtractionConfig {
    ExtractionConfig {
        include_document_structure: true,
        use_cache: false,
        cancel_token: Some(token.clone()),
        ocr: Some(crate::core::config::OcrConfig {
            backend: "geometry-cancellation-control".into(),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn geometry_page_hints() -> PageOcrHints {
    PageOcrHints {
        native_page_layers: Some(std::sync::Arc::new(
            SOURCE
                .iter()
                .enumerate()
                .map(|(index, text)| ((index + 1) as u32, text.to_string()))
                .collect(),
        )),
        ..Default::default()
    }
}
