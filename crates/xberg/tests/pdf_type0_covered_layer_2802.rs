#![cfg(all(feature = "pdf", feature = "ocr", feature = "bundle-tessdata-eng"))]

mod helpers;

use lopdf::{Document, Stream, dictionary};
use xberg::core::config::{ExtractionConfig, OcrConfig, OutputFormat, PageConfig};
use xberg::types::ExtractionMethod;

/// A one-page Type0 source whose `ToUnicode` omits `a` and whose encoding makes the render drop
/// those glyphs: its native text is the true content, and the render lost Type0 ink.
const TYPE0_PAGE: &[u8] = include_bytes!("../../xberg-native-pdf/tests/fixtures/cid_glyph_raster_2309/custom.pdf");
const SHORT_LINE: &str = "Quarterly revenue improved";
/// Visible lines of real words that share three words with the covered layer (committee, budget,
/// winter) among sixteen: the layer spells a few of them by chance, not the page.
const SHARING_LINES: [&str; 3] = [
    "Quarterly revenue improved in every region",
    "while the committee kept the budget steady",
    "and shipping costs fell before winter ended",
];
/// What the source's text layer says the short line reads, which its ink contradicts.
const CONTRADICTING_TEXT: &str = "Ledger entries pending";
/// Each retention site words its notice differently; both name the lost Type0 ink, so a match
/// also proves the render set `type0_ink_loss`.
const MIXED_RETAINED: &str = "Page 1 retained native text after its OCR render lost Type0 glyph ink.";
const SINGLE_RETAINED: &str = "Page 1 OCR read omitted native text after its render lost Type0 glyph ink";

#[derive(Clone, Copy, PartialEq)]
enum Layer {
    Covered,
    /// The covered layer beside visible lines that share a few words with it by chance.
    CoveredSharing,
    Visible,
    /// The visible layer under this many small black squares: the render reads as noise, and the
    /// native text is still what the page shows.
    Speckled(usize),
}

/// Squares of 2.2 points at pseudo-random places over the page, from a fixed linear congruential
/// sequence so every run draws the same page.
fn speckle(squares: usize) -> String {
    let mut state: u32 = 1;
    let mut next = |bound: u32| {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (state >> 8) % bound
    };
    let mut ops = String::from("\nq 0 g\n");
    for _ in 0..squares {
        let (x, y) = (next(600), next(780));
        ops.push_str(&format!("{x} {y} 2.2 2.2 re f\n"));
    }
    ops.push_str("Q\n");
    ops
}

/// The Type0 page followed by an ordinary second page. In the covered variant a white fill paints
/// over the Type0 layer and a short line of other text is drawn on top of it.
fn source_pdf(layer: Layer, second_page: bool) -> Vec<u8> {
    let mut pdf = Document::load_mem(TYPE0_PAGE).unwrap();
    let pages = *pdf.get_pages().get(&1).unwrap();
    let parent = pdf
        .get_dictionary(pages)
        .unwrap()
        .get(b"Parent")
        .unwrap()
        .as_reference()
        .unwrap();
    let helvetica = pdf.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica", "Encoding" => "WinAnsiEncoding",
    });
    let contents = pdf
        .get_dictionary(pages)
        .unwrap()
        .get(b"Contents")
        .unwrap()
        .as_reference()
        .unwrap();
    if matches!(layer, Layer::Covered | Layer::CoveredSharing) {
        let shown = if layer == Layer::Covered {
            format!("/F9 28 Tf 40 300 Td ({SHORT_LINE}) Tj")
        } else {
            format!(
                "/F9 20 Tf 40 400 Td 30 TL ({}) Tj T* ({}) Tj T* ({}) Tj",
                SHARING_LINES[0], SHARING_LINES[1], SHARING_LINES[2]
            )
        };
        let cover = format!(
            "\nq 1 g 0 0 612 792 re f Q\n/Span << /ActualText ({CONTRADICTING_TEXT}) >> BDC \
             BT {shown} ET EMC\n"
        );
        let stream = pdf.get_object_mut(contents).unwrap().as_stream_mut().unwrap();
        assert!(
            stream.dict.get(b"Filter").is_err(),
            "the source page stream must be unfiltered"
        );
        stream.content.extend_from_slice(cover.as_bytes());
        stream.dict.set("Length", stream.content.len() as i64);
        let resources = pdf.get_object_mut(pages).unwrap().as_dict_mut().unwrap();
        let fonts = resources
            .get_mut(b"Resources")
            .unwrap()
            .as_dict_mut()
            .unwrap()
            .get_mut(b"Font")
            .unwrap();
        fonts.as_dict_mut().unwrap().set("F9", helvetica);
    }
    if let Layer::Speckled(squares) = layer {
        let stream = pdf.get_object_mut(contents).unwrap().as_stream_mut().unwrap();
        assert!(
            stream.dict.get(b"Filter").is_err(),
            "the source page stream must be unfiltered"
        );
        stream.content.extend_from_slice(speckle(squares).as_bytes());
        stream.dict.set("Length", stream.content.len() as i64);
    }
    if second_page {
        let second_stream = pdf.add_object(Stream::new(
            dictionary! {},
            b"BT /F9 14 Tf 40 700 Td 18 TL (Second page text) Tj T* (stays an ordinary native page) Tj ET".to_vec(),
        ));
        let second = pdf.add_object(dictionary! {
            "Type" => "Page", "Parent" => parent, "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
            "Contents" => second_stream,
            "Resources" => dictionary! {"Font" => dictionary! {"F9" => helvetica}},
        });
        let kids = pdf.get_object_mut(parent).unwrap().as_dict_mut().unwrap();
        kids.get_mut(b"Kids")
            .unwrap()
            .as_array_mut()
            .unwrap()
            .push(second.into());
        kids.set("Count", 2);
    }
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    bytes
}

fn extract(layer: Layer, second_page: bool) -> xberg::ExtractedDocument {
    extract_as(layer, second_page, OutputFormat::Plain)
}

fn extract_as(layer: Layer, second_page: bool, output_format: OutputFormat) -> xberg::ExtractedDocument {
    let tessdata = tempfile::tempdir().unwrap();
    let english = xberg_tesseract::bundled_eng_traineddata().expect("bundled English data");
    std::fs::write(tessdata.path().join("eng.traineddata"), english).unwrap();
    let config = ExtractionConfig {
        use_cache: false,
        output_format,
        ocr: Some(OcrConfig {
            tessdata_path: Some(tessdata.path().to_path_buf()),
            ..Default::default()
        }),
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    helpers::extract_bytes_document_blocking(&source_pdf(layer, second_page), "application/pdf", &config).unwrap()
}

fn notice(result: &xberg::ExtractedDocument, text: &str) -> bool {
    result
        .processing_warnings
        .iter()
        .any(|warning| warning.message.contains(text))
}

/// Automatic single-page fallback may retain native text through either OCR route.
fn single_page_type0_retained(result: &xberg::ExtractedDocument) -> bool {
    notice(result, SINGLE_RETAINED) || notice(result, MIXED_RETAINED)
}

#[test]
fn a_covered_type0_layer_does_not_outvote_the_short_visible_read() {
    let result = extract(Layer::Covered, true);
    let page = &result.pages.as_ref().unwrap()[0].content;
    assert!(page.contains("Quarterly revenue improved"), "{page:?}");
    assert!(!page.contains("committee"), "{page:?}");
    assert!(!notice(&result, MIXED_RETAINED), "{:?}", result.processing_warnings);
}

/// The negative control of the share rule: three words the covered layer spells by chance do not
/// make it the page's content, so the visible lines go to OCR on both routes.
#[test]
fn a_covered_type0_layer_sharing_a_few_words_with_the_visible_read_goes_to_ocr() {
    for second_page in [false, true] {
        let result = extract(Layer::CoveredSharing, second_page);
        let page = &result.pages.as_ref().unwrap()[0].content;
        assert!(
            page.contains("shipping costs fell"),
            "second page {second_page}: {page:?}"
        );
        assert!(!page.contains("Tuesd"), "second page {second_page}: {page:?}");
        assert!(
            !single_page_type0_retained(&result),
            "second page {second_page}: {:?}",
            result.processing_warnings
        );
    }
}

#[test]
fn a_visible_correct_type0_layer_keeps_native_text_after_ink_loss() {
    let result = extract(Layer::Visible, true);
    let page = &result.pages.as_ref().unwrap()[0].content;
    assert!(page.contains("committee met on Tuesd"), "{page:?}");
    assert!(notice(&result, MIXED_RETAINED), "{:?}", result.processing_warnings);
    assert_eq!(result.extraction_method, Some(ExtractionMethod::Native));
}

#[test]
fn a_covered_single_page_type0_layer_does_not_outvote_the_short_visible_read() {
    let result = extract(Layer::Covered, false);
    let page = &result.pages.as_ref().unwrap()[0].content;
    assert_eq!(result.pages.as_ref().unwrap().len(), 1);
    assert!(page.contains("Quarterly revenue improved"), "{page:?}");
    assert!(!page.contains("committee"), "{page:?}");
    assert!(!single_page_type0_retained(&result), "{:?}", result.processing_warnings);
}

#[test]
fn a_visible_single_page_type0_layer_keeps_native_text_after_ink_loss() {
    let result = extract(Layer::Visible, false);
    let page = &result.pages.as_ref().unwrap()[0].content;
    assert!(page.contains("committee met on Tuesd"), "{page:?}");
    assert!(single_page_type0_retained(&result), "{:?}", result.processing_warnings);
    assert_eq!(result.extraction_method, Some(ExtractionMethod::Native));
}

#[test]
fn a_speckled_type0_layer_keeps_native_text_after_ink_loss() {
    let result = extract(Layer::Speckled(9000), true);
    let pages = result.pages.as_ref().unwrap();
    assert!(
        pages[0].content.contains("committee met on Tuesd"),
        "{:?}",
        pages[0].content
    );
    assert!(pages[1].content.contains("Second page text"), "{:?}", pages[1].content);
    assert!(notice(&result, MIXED_RETAINED), "{:?}", result.processing_warnings);
}

#[test]
fn a_speckled_single_page_type0_layer_keeps_native_text_after_ink_loss() {
    let result = extract(Layer::Speckled(9000), false);
    let page = &result.pages.as_ref().unwrap()[0].content;
    assert!(page.contains("committee met on Tuesd"), "{page:?}");
    assert!(single_page_type0_retained(&result), "{:?}", result.processing_warnings);
    assert_eq!(result.extraction_method, Some(ExtractionMethod::Native));
}

/// The page keeps its native text in both forms, whatever blocks, lists or tables the noisy read
/// was built into.
fn assert_speckled_page_keeps_native_text(squares: usize, output_format: OutputFormat) {
    for second_page in [false, true] {
        let result = extract_as(Layer::Speckled(squares), second_page, output_format.clone());
        let pages = result.pages.as_ref().unwrap();
        assert!(
            pages[0].content.contains("committee met on Tuesd"),
            "{squares} squares, second page {second_page}: {:?}",
            pages[0].content
        );
        assert!(
            single_page_type0_retained(&result),
            "{squares} squares, second page {second_page}: {:?}",
            result.processing_warnings
        );
        if second_page {
            assert!(pages[1].content.contains("Second page text"), "{:?}", pages[1].content);
        }
    }
}

#[test]
fn a_lightly_speckled_type0_layer_keeps_native_text_after_ink_loss() {
    assert_speckled_page_keeps_native_text(3000, OutputFormat::Plain);
}

#[test]
fn a_moderately_speckled_type0_layer_keeps_native_text_after_ink_loss() {
    assert_speckled_page_keeps_native_text(6000, OutputFormat::Plain);
}

#[test]
fn a_speckled_type0_layer_keeps_native_text_in_markdown() {
    assert_speckled_page_keeps_native_text(9000, OutputFormat::Markdown);
}

#[test]
fn a_lightly_speckled_type0_layer_keeps_native_text_in_markdown() {
    assert_speckled_page_keeps_native_text(3000, OutputFormat::Markdown);
}
