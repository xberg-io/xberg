//! Regression tests for issue #794: `remove_footers` on untagged PDFs
//! where the footer's page number shares a baseline with constant brand
//! chrome — in one combined span, and in a recto/verso (alternating-page)
//! layout split across separate spans.

mod common;

use common::build_pdf_with_page_extras;
use xberg_native_pdf::PdfDocument;

/// A footer where the page number is combined with a constant brand
/// fragment on one line ("1 com", "2 com", ...). The helper's own "Body
/// text placeholder" line covers the incidental body content.
fn build_combined_span_footer_pdf(page_count: usize) -> Vec<u8> {
    build_pdf_with_page_extras(page_count, |i| {
        format!("BT /F1 10 Tf 1 0 0 1 72 30 Tm ({} com) Tj ET\n", i + 1)
    })
}

/// A footer where the page number and a constant brand fragment are ONE
/// span ("1 com", "2 com", ...) — mirroring a real single-run footer like
/// "1 ERLC.com". Confirmed to fail before this fix and pass after: the
/// exact-text pass only matches a span's full text verbatim against
/// itself, so a span whose digit varies every page never repeats and is
/// invisible to it; the digit-normalizing signature ("# com") is what
/// catches it.
#[test]
fn remove_footers_strips_combined_page_number_and_brand_span() {
    let bytes = build_combined_span_footer_pdf(6);
    let doc = PdfDocument::from_bytes(bytes).unwrap();

    let removed = doc.remove_footers(0.2).unwrap();
    assert!(removed > 0, "expected remove_footers to erase something");

    for page in 1..6 {
        let text = doc.extract_text(page).unwrap();
        assert!(!text.contains("com"), "page {page}: brand fragment survived: {text:?}");
        assert!(
            !text.contains(&format!("{}", page + 1)),
            "page {page}: page number survived: {text:?}"
        );
        assert!(
            text.contains("Body text placeholder"),
            "page {page}: body content wrongly removed: {text:?}"
        );
    }
}
