#![cfg(all(feature = "pdf", feature = "ocr", feature = "bundle-tessdata-eng"))]

mod helpers;

use lopdf::{Document, Stream, dictionary};
use xberg::core::config::{ExtractionConfig, OcrConfig, OutputFormat, PageConfig};
use xberg::types::ExtractionMethod;

/// A one-page Type0 source whose `ToUnicode` omits `a` and whose encoding makes the render drop
/// those glyphs: its native text is the true content, and the render lost Type0 ink.
const TYPE0_PAGE: &[u8] = include_bytes!("../../xberg-native-pdf/tests/fixtures/cid_glyph_raster_2309/custom.pdf");
const SHORT_LINE: &str = "Quarterly revenue improved";
/// What the source's text layer says the short line reads, which its ink contradicts.
const CONTRADICTING_TEXT: &str = "Ledger entries pending";
const RETAINED_WARNING: &str = "retained native text after its OCR render lost Type0 glyph ink";

#[derive(Clone, Copy, PartialEq)]
enum Layer {
    Covered,
    Visible,
}

/// The Type0 page followed by an ordinary second page. In the covered variant a white fill paints
/// over the Type0 layer and a short line of other text is drawn on top of it.
fn source_pdf(layer: Layer) -> Vec<u8> {
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
    if layer == Layer::Covered {
        let cover = format!(
            "\nq 1 g 0 0 612 792 re f Q\n/Span << /ActualText ({CONTRADICTING_TEXT}) >> BDC \
             BT /F9 28 Tf 40 300 Td ({SHORT_LINE}) Tj ET EMC\n"
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
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    bytes
}

fn extract(layer: Layer) -> xberg::ExtractedDocument {
    let tessdata = tempfile::tempdir().unwrap();
    let english = xberg_tesseract::bundled_eng_traineddata().expect("bundled English data");
    std::fs::write(tessdata.path().join("eng.traineddata"), english).unwrap();
    let config = ExtractionConfig {
        use_cache: false,
        output_format: OutputFormat::Plain,
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
    helpers::extract_bytes_document_blocking(&source_pdf(layer), "application/pdf", &config).unwrap()
}

fn retained(result: &xberg::ExtractedDocument) -> bool {
    result
        .processing_warnings
        .iter()
        .any(|warning| warning.message.contains(RETAINED_WARNING))
}

#[test]
fn a_covered_type0_layer_does_not_outvote_the_short_visible_read() {
    let result = extract(Layer::Covered);
    let page = &result.pages.as_ref().unwrap()[0].content;
    assert!(page.contains("Quarterly revenue improved"), "{page:?}");
    assert!(!page.contains("committee"), "{page:?}");
    assert!(!retained(&result), "{:?}", result.processing_warnings);
}

#[test]
fn a_visible_correct_type0_layer_keeps_native_text_after_ink_loss() {
    let result = extract(Layer::Visible);
    let page = &result.pages.as_ref().unwrap()[0].content;
    assert!(page.contains("committee met on Tuesd"), "{page:?}");
    assert!(retained(&result), "{:?}", result.processing_warnings);
    assert_eq!(result.extraction_method, Some(ExtractionMethod::Native));
}
