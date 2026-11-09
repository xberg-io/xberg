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
            ocr: &[1],
            single_block: &[],
            preserve_native: &[],
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
            ocr: &[1],
            single_block: &[],
            preserve_native: &[],
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
                ocr: &[1],
                single_block: &[],
                preserve_native: &[],
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

const SHORT: &str = "layer-lost-glyph-short-read-backend";
/// A short, correct read of a page: under half of the readable letters and digits of the long
/// marked layer below. ~keep
const SHORT_READ: &str = "The committee met on Tuesday to review the plans for the library.";
const LONG_MARKED: [&str; 8] = [
    "The warehouse workers inspected the broken engine beside the river.",
    "Visitors requested a refund for the damaged furniture and plates.",
    "The gardener planted roses behind the wooden gate last summer.",
    "A musician played a quiet melody during the evening concert.",
    "The captain ordered every sailor to secure the ropes before sunrise.",
    "The baker sold fresh bread to the neighbours every early morning.",
    "Two cyclists repaired a flat tyre near the old stone bridge.",
    "A painter covered the tall fence with three coats of green paint.",
];
const CLEAN_PAGE: &str = "Plain second page words stay exactly as the native layer printed them.";

/// Reads `SHORT_READ` on every page. ~keep
struct ShortReadBackend;

impl Plugin for ShortReadBackend {
    fn name(&self) -> &str {
        SHORT
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
impl OcrBackend for ShortReadBackend {
    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }
    fn supports_language(&self, _: &str) -> bool {
        true
    }
    async fn process_image(&self, _: &[u8], _: &OcrConfig) -> crate::Result<crate::ExtractedDocument> {
        Ok(crate::ExtractedDocument {
            content: SHORT_READ.to_owned(),
            ..Default::default()
        })
    }
}

/// The native text and page boundaries of a two-page document whose page `marked` (1 or 2) holds
/// the long marked layer and whose other page is clean. ~keep
fn long_marked_layer_document(marked: u32) -> (String, Vec<crate::types::PageBoundary>) {
    let layer = format!("{}\nA\u{FFFD}B", LONG_MARKED.join("\n"));
    let pages = if marked == 1 {
        [layer.as_str(), CLEAN_PAGE]
    } else {
        [CLEAN_PAGE, layer.as_str()]
    };
    let text = pages.join("\n");
    let boundaries = vec![
        crate::types::PageBoundary {
            byte_start: 0,
            byte_end: pages[0].len(),
            page_number: 1,
        },
        crate::types::PageBoundary {
            byte_start: pages[0].len() + 1,
            byte_end: text.len(),
            page_number: 2,
        },
    ];
    (text, boundaries)
}

/// The failures of one run of the short read on page `marked`: the read must replace the marked
/// layer, which no same-page evidence proves, and the clean page stays. ~keep
async fn short_read_failures(marked: u32, forced: bool) -> Vec<String> {
    let (native, boundaries) = long_marked_layer_document(marked);
    let pdf = super::super::tests::tests::build_minimal_two_page_pdf_with_sizes((612.0, 792.0), (612.0, 792.0));
    let mut config = config();
    config.ocr.as_mut().unwrap().backend = SHORT.to_owned();
    if forced {
        config.force_ocr_pages = Some(vec![marked]);
    }
    let run = extract_mixed_ocr_native_with_layout_inputs(
        &native,
        &boundaries,
        MixedOcrPageSelection {
            ocr: &[marked],
            single_block: &[],
            preserve_native: &[],
            native_layers: None,
            hidden_layer_backend: None,
            #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
            single_native_type0_page: None,
            raster_dpis: None,
        },
        &pdf,
        &config,
        if forced {
            AllPagesFailedPolicy::ReturnError
        } else {
            AllPagesFailedPolicy::PreserveNative
        },
        MixedLayoutInputs::Resolve(None),
    )
    .await;
    let run = match run {
        Ok(run) => run,
        Err(failed) => {
            return vec![format!(
                "page {marked} forced {forced}: the page reads: {:?}",
                failed.error
            )];
        }
    };
    let text = &run.output.0;
    let mut failures = Vec::new();
    if text.matches(SHORT_READ).count() != 1 {
        failures.push(format!(
            "page {marked} forced {forced}: the short read prints once: {text:?}"
        ));
    }
    if LONG_MARKED.iter().any(|row| text.contains(row)) {
        failures.push(format!(
            "page {marked} forced {forced}: the unproved marked layer is dropped: {text:?}"
        ));
    }
    if text.matches(CLEAN_PAGE).count() != 1 {
        failures.push(format!("page {marked} forced {forced}: the clean page stays: {text:?}"));
    }
    failures
}

/// Review point 1 of #2637: a marked page keeps its layer against the OCR read only on same-page
/// ink evidence for the layer. A custom backend gives none, so a short correct read replaces a
/// long marked layer, in both page orders. ~keep
#[tokio::test]
#[serial_test::serial]
async fn a_short_read_replaces_a_long_marked_layer_without_same_page_evidence() {
    register_ocr_backend(Arc::new(ShortReadBackend)).unwrap();
    let _guard = NamedGuard(SHORT);
    let mut failures = short_read_failures(1, false).await;
    failures.extend(short_read_failures(2, false).await);
    assert!(failures.is_empty(), "{failures:#?}");
}

/// Review point 2 of #2637: a page the caller forces to OCR keeps its successful short read, in
/// both page orders. ~keep
#[tokio::test]
#[serial_test::serial]
async fn a_forced_marked_page_keeps_its_successful_short_read() {
    register_ocr_backend(Arc::new(ShortReadBackend)).unwrap();
    let _guard = NamedGuard(SHORT);
    let mut failures = short_read_failures(1, true).await;
    failures.extend(short_read_failures(2, true).await);
    assert!(failures.is_empty(), "{failures:#?}");
}
