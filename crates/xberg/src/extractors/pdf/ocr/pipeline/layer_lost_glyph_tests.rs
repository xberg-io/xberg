use super::*;
use crate::core::config::OcrConfig;
use crate::plugins::{OcrBackend, OcrBackendType, Plugin, register_ocr_backend, unregister_ocr_backend};
use std::sync::Arc;

const NOISE: &str = "layer-lost-glyph-noise-backend";
const PAGE: &[u8] = include_bytes!("../../../../../tests/fixtures/ocr/detached_heading_frame_2497/heading-table-0.pdf");
const READ: &str = "qq ww ee rr tt yy uu ii oo pp ss dd ff gg hh jj kk ll zz xx";

struct NoiseBackend;

impl Plugin for NoiseBackend {
    fn name(&self) -> &str {
        NOISE
    }
    fn version(&self) -> String {
        "1.0.0".to_owned()
    }
    fn initialize(&self) -> crate::Result<()> {
        Ok(())
    }
    fn shutdown(&self) -> crate::Result<()> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl OcrBackend for NoiseBackend {
    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }
    fn supports_language(&self, _: &str) -> bool {
        true
    }
    async fn process_image(&self, _: &[u8], _: &OcrConfig) -> crate::Result<crate::ExtractedDocument> {
        Ok(crate::ExtractedDocument {
            content: READ.to_owned(),
            ..Default::default()
        })
    }
}

struct BackendGuard;
impl Drop for BackendGuard {
    fn drop(&mut self) {
        let _ = unregister_ocr_backend(NOISE);
    }
}

fn config() -> ExtractionConfig {
    ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: NOISE.to_owned(),
            ..Default::default()
        }),
        use_cache: false,
        ..Default::default()
    }
}

fn warns_of_lost_glyphs(run: &MixedOcrRun) -> bool {
    run.output
        .8
        .iter()
        .any(|warning| warning.source == "pdf" && warning.message.contains("no Unicode"))
}

/// The selected-page route builds the layer from its own native text: the row put first lost a
/// glyph, so the warning names the loss. ~keep
#[tokio::test]
#[serial_test::serial]
async fn a_selected_page_whose_marked_layer_is_kept_warns_of_its_lost_glyphs() {
    register_ocr_backend(Arc::new(NoiseBackend)).unwrap();
    let _guard = BackendGuard;
    let native = "Reliable native source words remain available beside the missing character boundary.\nA\u{FFFD}B";
    let boundaries = [crate::types::PageBoundary {
        byte_start: 0,
        byte_end: native.len(),
        page_number: 1,
    }];
    let run = extract_mixed_ocr_native_with_layout_inputs(
        native,
        &boundaries,
        MixedOcrPageSelection {
            #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
            probe_reads: None,
            ocr: &[1],
            single_block: &[],
            preserve_native: &[],
            native_script: &[],
            native_layers: None,
            hidden_layer_backend: None,
            #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
            single_native_type0_page: None,
            raster_dpis: None,
        },
        PAGE,
        &config(),
        AllPagesFailedPolicy::PreserveNative,
        MixedLayoutInputs::Resolve(None),
    )
    .await
    .unwrap_or_else(|failed| panic!("the page reads: {:?}", failed.error));
    assert!(
        run.output.0.contains("A B"),
        "the marked layer is kept: {:?}",
        run.output.0
    );
    assert!(warns_of_lost_glyphs(&run), "{:?}", run.output.8);
}

/// A whole-document caller hands the layer beside a text that holds no native page, as the
/// per-page pipeline route does: the layer keeps its markers, and the row put first counts them. ~keep
#[tokio::test]
#[serial_test::serial]
async fn a_caller_supplied_layer_warns_of_the_glyphs_its_page_lost() {
    register_ocr_backend(Arc::new(NoiseBackend)).unwrap();
    let _guard = BackendGuard;
    let boundaries = [crate::types::PageBoundary {
        byte_start: 0,
        byte_end: 0,
        page_number: 1,
    }];
    let layer = "Reliable native source words remain available beside the missing character boundary.\nA\u{FFFD}B";
    let run = extract_mixed_ocr_native_with_layout_inputs(
        "",
        &boundaries,
        MixedOcrPageSelection {
            #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
            probe_reads: None,
            ocr: &[1],
            single_block: &[],
            preserve_native: &[],
            native_script: &[],
            native_layers: Some(Arc::new(ahash::AHashMap::from([(1, layer.to_owned())]))),
            hidden_layer_backend: None,
            #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
            single_native_type0_page: None,
            raster_dpis: None,
        },
        PAGE,
        &config(),
        AllPagesFailedPolicy::ReturnError,
        MixedLayoutInputs::Resolve(None),
    )
    .await
    .unwrap_or_else(|failed| panic!("the page reads: {:?}", failed.error));
    assert!(run.output.0.contains("A B"), "the layer is kept: {:?}", run.output.0);
    assert!(warns_of_lost_glyphs(&run), "{:?}", run.output.8);
}

const MISREAD: &str = "layer-lost-glyph-misread-backend";
const MISREAD_RICH: &str = "layer-lost-glyph-misread-rich-backend";
const MISREAD_ROW: &str = "Total 12 cm\u{2019} of water";

/// Reads the noise words under one row that misreads the layer's measure row; the rich variant
/// hands the row over as a heading. ~keep
struct MisreadBackend {
    rich: bool,
}

impl Plugin for MisreadBackend {
    fn name(&self) -> &str {
        if self.rich { MISREAD_RICH } else { MISREAD }
    }
    fn version(&self) -> String {
        "1.0.0".to_owned()
    }
    fn initialize(&self) -> crate::Result<()> {
        Ok(())
    }
    fn shutdown(&self) -> crate::Result<()> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl OcrBackend for MisreadBackend {
    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }
    fn supports_language(&self, _: &str) -> bool {
        true
    }
    async fn process_image(&self, _: &[u8], _: &OcrConfig) -> crate::Result<crate::ExtractedDocument> {
        use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
        let ocr_internal_document = self.rich.then(|| {
            let mut doc = InternalDocument::new("pdf");
            doc.push_element(InternalElement::text(ElementKind::Heading { level: 1 }, MISREAD_ROW, 0));
            doc.push_element(InternalElement::text(ElementKind::Paragraph, READ, 0));
            doc
        });
        Ok(crate::ExtractedDocument {
            content: format!("{MISREAD_ROW}\n{READ}"),
            ocr_internal_document,
            ..Default::default()
        })
    }
}

struct NamedGuard(&'static str);
impl Drop for NamedGuard {
    fn drop(&mut self) {
        let _ = unregister_ocr_backend(self.0);
    }
}

/// The mixed route drops the OCR row that misreads a restored native row on a page of plain OCR
/// text, and keeps it on a page whose OCR payload is rich. ~keep
#[tokio::test]
#[serial_test::serial]
async fn the_mixed_route_drops_a_misread_ocr_row_on_a_plain_page_only() {
    for rich in [false, true] {
        let backend = MisreadBackend { rich };
        let name = if rich { MISREAD_RICH } else { MISREAD };
        register_ocr_backend(Arc::new(backend)).unwrap();
        let _guard = NamedGuard(name);
        let native = concat!(
            "Reliable native source words remain available beside the missing character boundary.\n",
            "Total 12 cm3 of water"
        );
        let boundaries = [crate::types::PageBoundary {
            byte_start: 0,
            byte_end: native.len(),
            page_number: 1,
        }];
        let mut config = config();
        config.ocr.as_mut().unwrap().backend = name.to_owned();
        let run = extract_mixed_ocr_native_with_layout_inputs(
            native,
            &boundaries,
            MixedOcrPageSelection {
                #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
                probe_reads: None,
                ocr: &[1],
                single_block: &[],
                preserve_native: &[],
                native_script: &[],
                native_layers: None,
                hidden_layer_backend: None,
                #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
                single_native_type0_page: None,
                raster_dpis: None,
            },
            PAGE,
            &config,
            AllPagesFailedPolicy::PreserveNative,
            MixedLayoutInputs::Resolve(None),
        )
        .await
        .unwrap_or_else(|failed| panic!("the page reads: {:?}", failed.error));
        let text = &run.output.0;
        assert!(
            text.contains("Total 12 cm3 of water"),
            "rich {rich}: the native row is kept: {text:?}"
        );
        assert_eq!(
            text.contains(MISREAD_ROW),
            rich,
            "rich {rich}: the misread OCR row is printed on a rich page only: {text:?}"
        );
    }
}

#[test]
fn a_native_layer_with_either_lost_glyph_marker_is_not_a_clean_ink_layer() {
    let mut layers = ahash::AHashMap::new();
    layers.insert(1, "clean words".to_string());
    layers.insert(
        2,
        format!("lost{}words", super::super::super::unmapped_glyphs::LOST_GLYPH_MARKER),
    );
    layers.insert(
        3,
        format!(
            "symbol{}words",
            super::super::super::unmapped_glyphs::LOST_SYMBOL_GLYPH_MARKER
        ),
    );
    assert_eq!(clean_ink_layer_of(Some(&layers), &1), Some("clean words"));
    assert_eq!(clean_ink_layer_of(Some(&layers), &2), None);
    assert_eq!(clean_ink_layer_of(Some(&layers), &3), None);
    assert_eq!(clean_ink_layer_of(Some(&layers), &4), None);
    assert_eq!(clean_ink_layer_of(None, &1), None);
}

#[cfg(all(paddle_ocr, feature = "bundle-tessdata-eng"))]
#[tokio::test]
#[ignore = "Requires the pinned Khmer model and the implicit per-page OCR pipeline"]
async fn recovered_number_cannot_confirm_the_native_layer() {
    const UNPAINTED: &str = "UNPAINTEDNATIVELAYERCONTROLMARKER";
    const MATCHING: &[u8] = include_bytes!("../../../../../tests/fixtures/pdf_khmer_number_2408/matching.pdf");
    let mut doc = lopdf::Document::load_mem(MATCHING).unwrap();
    let page_id = doc.get_pages()[&1];
    let content = doc.get_page_content(page_id);
    let content = String::from_utf8(content).unwrap();
    assert_eq!(content.matches(r"(\001\002\001) Tj").count(), 1);
    // Keep the original number origin while making it a whole word in the native layer.
    let content = content.replace(r"(\001\002\001) Tj", "[ -635 <02> ] TJ");
    let content = format!("{content}\nBT /F1 12 Tf 3 Tr 1 0 0 1 100 320 Tm (office {UNPAINTED}) Tj ET\n");
    doc.change_page_content(page_id, content.into_bytes()).unwrap();
    let mut input = Vec::new();
    doc.save_to(&mut input).unwrap();
    let pdf = xberg_native_pdf::PdfDocument::from_bytes(input.clone()).unwrap();
    let layer = pdf.extract_text(0).unwrap();
    let thresholds = crate::core::config::OcrQualityThresholds::default();
    assert!(
        super::super::document::ocr_confirms_layer(&layer, "office ០", &thresholds),
        "{layer}"
    );
    assert!(
        !super::super::document::ocr_confirms_layer(&layer, "office", &thresholds),
        "{layer}"
    );

    let models = tempfile::tempdir().unwrap();
    let english = xberg_tesseract::bundled_eng_traineddata().expect("bundled English");
    std::fs::write(models.path().join("eng.traineddata"), english).unwrap();
    let khmer = std::env::var_os("XBERG_2408_KHM_MODEL").expect("set XBERG_2408_KHM_MODEL");
    std::fs::copy(khmer, models.path().join("khm.traineddata")).unwrap();
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            tessdata_path: Some(models.path().to_owned()),
            ..Default::default()
        }),
        use_cache: false,
        ..Default::default()
    };
    let boundaries = [crate::types::PageBoundary {
        byte_start: 0,
        byte_end: 0,
        page_number: 1,
    }];
    let run = extract_mixed_ocr_native_with_layout_inputs(
        "",
        &boundaries,
        MixedOcrPageSelection {
            native_layers: Some(Arc::new(ahash::AHashMap::from([(1, layer)]))),
            ocr: &[1],
            single_block: &[],
            preserve_native: &[],
            native_script: &[],
            probe_reads: None,
            raster_dpis: None,
            hidden_layer_backend: None,
            single_native_type0_page: None,
        },
        &input,
        &config,
        AllPagesFailedPolicy::ReturnError,
        MixedLayoutInputs::Resolve(None),
    )
    .await
    .unwrap_or_else(|failed| panic!("the page reads: {:?}", failed.error));
    for text in [&run.output.0, &run.output.1[&1]] {
        assert!(
            text.contains("The office keeps the current monthly service records"),
            "{text}"
        );
        assert_eq!(text.matches('០').count(), 1, "recover the painted number: {text}");
        assert!(
            !text.contains(UNPAINTED),
            "the recovered number must not confirm the layer: {text}"
        );
    }
}

#[test]
fn noise_verdict_retains_the_same_native_row_with_a_recovered_number() {
    use super::super::document::{PlainOcrRows, text_with_layer_first};
    let thresholds = crate::core::config::OcrQualityThresholds::default();
    let layer = "Reliable native source words remain available beside the OCR read.";
    let primary = "VOU";
    let recovered = "VOU ០";
    assert!(!super::super::document::ocr_confirms_layer(
        layer,
        recovered,
        &thresholds
    ));
    for read in [primary, recovered] {
        let first = mixed_layer_lines_to_put_first(
            layer,
            primary,
            read,
            true,
            &thresholds,
            None,
            PlainOcrRows::of_page(None, None),
        )
        .expect("the noise verdict retains the native row");
        assert_eq!(first.lines, layer);
        let output = text_with_layer_first(read, &first);
        assert_eq!(output, format!("{layer}\n\n{read}"));
    }
}
