use super::*;
use sha2::{Digest, Sha256};
use xberg_native_pdf::geometry::Rect;
use xberg_native_pdf::layout::TextSpan;

fn fragment(text: &str, x: f32, y: f32, width: f32, font_size: f32) -> TextSpan {
    TextSpan {
        text: text.to_owned(),
        bbox: Rect {
            x,
            y,
            width,
            height: font_size,
        },
        font_size,
        ..TextSpan::default()
    }
}

fn calculus_native_plain() -> String {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest = std::fs::read_to_string(root.join("tests/fixtures/pdf/regression_pages.sha256")).unwrap();
    let name = "gutenberg-33283-p161.pdf";
    let expected = manifest
        .lines()
        .find_map(|line| {
            let (hash, path) = line.split_once("  ")?;
            (path == name).then_some(hash)
        })
        .expect("source page is in the committed manifest");
    let bytes = std::fs::read(root.join("../../test_documents/pdf/regression_pages").join(name))
        .expect("mandatory source page is present");
    assert_eq!(hex::encode(Sha256::digest(&bytes)), expected, "source PDF identity");
    let source = xberg_native_pdf::PdfDocument::from_bytes(bytes.clone()).unwrap();
    let page = page_text_with_options_excluding_layers(&source, 0, &std::collections::HashSet::new()).unwrap();
    assert!(
        is_fragmented_span_list(&page.spans),
        "source exercises the fragmented text path"
    );
    let mut doc = NativeDocument::open_bytes(&bytes).unwrap();
    extract_text_from_native_document(&mut doc, None, None, PageMarginFractions::default())
        .expect("native plain source extraction succeeds")
        .0
}

#[test]
fn native_plain_calculus_preserves_the_word_space_before_the_variable() {
    let text = calculus_native_plain();
    assert!(
        text.contains("Differentiate y ="),
        "source prose and its variable stay separate: {text}"
    );
}

#[test]
fn native_plain_calculus_preserves_the_logarithm_argument_space() {
    let text = calculus_native_plain();
    assert!(
        text.contains("logϵ x"),
        "source logarithm base and argument stay separate: {text}"
    );
}

#[test]
fn fragmented_prose_preserves_the_measured_source_word_gap() {
    let spans = [
        fragment("(1) Differentiate", 50.307_007, 187.711_01, 84.377_41, 11.9552),
        fragment("y", 138.588_01, 187.711_01, 5.707_413, 11.9552),
    ];
    let gap = spans[1].bbox.x - spans[0].bbox.x - spans[0].bbox.width;
    assert!((gap - 3.9036).abs() < 0.0001, "source gap is preserved");
    assert_eq!(rebuild_text_from_fragmented_spans(&spans), "(1) Differentiate y");
}

#[test]
fn fragmented_logarithm_keeps_its_index_attached_and_argument_separate() {
    let spans = [
        fragment("log", 101.35, 497.745, 14.958_347, 11.9552),
        fragment("ϵ", 116.47, 494.922, 3.423_158, 7.9701),
        fragment("x", 122.384, 497.745, 6.651_874, 11.9552),
    ];
    assert_eq!(rebuild_text_from_fragmented_spans(&spans), "logϵ x");
}

#[test]
fn fragmented_word_preserves_its_tightly_spaced_final_glyph() {
    let spans = [
        fragment("wor", 100.0, 700.0, 18.0, 12.0),
        fragment("d", 118.4, 700.0, 6.0, 12.0),
    ];
    assert_eq!(rebuild_text_from_fragmented_spans(&spans), "word");
}

#[test]
fn fragmented_rows_preserve_their_existing_vertical_and_horizontal_order() {
    let spans = [
        fragment("D", 106.0, 670.0, 6.0, 10.0),
        fragment("B", 106.0, 700.0, 6.0, 10.0),
        fragment("A", 100.0, 700.0, 6.0, 10.0),
        fragment("C", 100.0, 670.0, 6.0, 10.0),
    ];
    assert_eq!(rebuild_text_from_fragmented_spans(&spans), "AB\nCD");
}

#[test]
fn fragmented_word_boundaries_do_not_duplicate_source_whitespace() {
    let spans = [
        fragment("word ", 100.0, 700.0, 30.0, 12.0),
        fragment("next", 134.0, 700.0, 24.0, 12.0),
    ];
    assert_eq!(rebuild_text_from_fragmented_spans(&spans), "word next");
}

#[test]
fn fragmented_numeric_note_keeps_its_existing_prose_separator() {
    let spans = [
        fragment("comma 3", 100.0, 700.0, 60.0, 12.0),
        fragment("5", 160.1, 700.0, 4.0, 8.0),
        fragment("5", 100.0, 600.0, 4.0, 6.0),
        fragment("A supporting note", 105.0, 600.0, 60.0, 8.0),
    ];
    assert_eq!(
        rebuild_text_from_fragmented_spans(&spans),
        "comma 3 5\n5A supporting note"
    );
}

#[test]
fn fragmented_true_exponent_keeps_its_existing_plain_text_join() {
    let spans = [
        fragment("x", 100.0, 700.0, 7.0, 12.0),
        fragment("2", 107.1, 704.0, 4.0, 8.0),
    ];
    assert_eq!(rebuild_text_from_fragmented_spans(&spans), "x2");
}

#[test]
fn fragmented_cjk_glyphs_keep_their_existing_contiguous_join() {
    let spans = [
        fragment("你", 100.0, 700.0, 12.0, 12.0),
        fragment("好", 112.0, 700.0, 12.0, 12.0),
        fragment("世", 124.0, 700.0, 12.0, 12.0),
        fragment("界", 136.0, 700.0, 12.0, 12.0),
    ];
    assert_eq!(rebuild_text_from_fragmented_spans(&spans), "你好世界");
}

#[test]
fn fragmented_rtl_runs_preserve_their_source_space_without_new_reordering() {
    let mut first = fragment("שלום ", 100.0, 700.0, 40.0, 12.0);
    first.rtl_draw_logical = true;
    let mut second = fragment("עולם", 144.0, 700.0, 30.0, 12.0);
    second.rtl_draw_logical = true;
    assert_eq!(rebuild_text_from_fragmented_spans(&[second, first]), "שלום עולם");
}
