#[cfg(not(feature = "ocr"))]
use super::tests::build_paged_text_pdf;
use super::*;

// OCR tests already install a global allocation observer. ~keep
#[cfg(not(feature = "ocr"))]
#[path = "span_carrier_tests/allocations.rs"]
mod allocations;

#[path = "span_carrier_tests/behavior.rs"]
mod behavior;

#[cfg(not(feature = "ocr"))]
#[test]
fn should_avoid_materializing_unused_character_records_in_the_core_span_carrier() {
    let bytes = build_paged_text_pdf(1, 24);
    let document = xberg_native_pdf::PdfDocument::from_bytes(bytes).unwrap();
    let excluded = HashSet::new();
    for _ in 0..2 {
        page_text_with_options_excluding_layers(&document, 0, &excluded).unwrap();
        document
            .extract_spans_with_reading_order(0, ReadingOrder::ColumnAware)
            .unwrap();
    }
    let legacy = document
        .extract_page_text_with_options(0, ReadingOrder::ColumnAware)
        .unwrap();
    let glyphs = legacy.chars.len();
    assert!(
        glyphs > 1000,
        "real source glyphs must exercise per-character allocation"
    );
    let (_, span_bytes) = allocations::measured(|| {
        document
            .extract_spans_with_reading_order(0, ReadingOrder::ColumnAware)
            .unwrap()
    });
    let (page, carrier_bytes) =
        allocations::measured(|| page_text_with_options_excluding_layers(&document, 0, &excluded).unwrap());
    assert_eq!(
        serde_json::to_value(&page.spans).unwrap(),
        serde_json::to_value(&legacy.spans).unwrap()
    );
    println!("CUMULATIVE_ALLOCATION glyphs={glyphs} spans={span_bytes} carrier={carrier_bytes}");
    let character_storage = glyphs * std::mem::size_of::<xberg_native_pdf::layout::TextChar>();
    assert!(
        carrier_bytes.saturating_sub(span_bytes) < character_storage / 2,
        "unused derived characters cost {character_storage} bytes: spans={span_bytes}, carrier={carrier_bytes}, glyphs={glyphs}"
    );
}
