//! A signature seen at two positions keeps the position that recurs on the
//! most pages. Ranking by occurrence count let one page repeating the text five
//! times outvote a footer present on four pages, so the recurrence filter then
//! saw a single page and rejected the signature.

mod common;

use common::build_pdf_with_page_extras;
use xberg_native_pdf::PdfDocument;

fn header_is_artifact(doc: &PdfDocument, page: usize) -> bool {
    doc.extract_spans(page)
        .expect("spans")
        .iter()
        .find(|s| s.text.contains("Annual Report"))
        .unwrap_or_else(|| panic!("no header span on page {page}"))
        .artifact_type
        .is_some()
}

#[test]
fn test_recurring_header_outranks_a_crowded_single_page() {
    let bytes = build_pdf_with_page_extras(5, |i| {
        if i == 0 {
            (0..5)
                .map(|n| {
                    format!(
                        "BT /F1 10 Tf 1 0 0 1 72 {} Tm (Annual Report {}) Tj ET\n",
                        750 - n * 4,
                        n + 1
                    )
                })
                .collect()
        } else {
            format!("BT /F1 10 Tf 1 0 0 1 400 750 Tm (Annual Report {}) Tj ET\n", i + 10)
        }
    });
    let doc = PdfDocument::from_bytes(bytes).expect("parse");

    assert!(
        header_is_artifact(&doc, 3),
        "the header on four pages was not recognised"
    );
}
