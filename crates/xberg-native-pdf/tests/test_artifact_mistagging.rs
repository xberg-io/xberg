//! Regression tests: PDF text extraction must not mis-tag content that
//! only coincidentally resembles a running header/footer/page-number as
//! a pagination artifact.
//!
//! One case: distinct per-page form line-item labels ("1a", "2a", "3a",
//! ...) get treated as a running pagination artifact, because digit
//! values are normalized away when detecting recurring page numbers — so
//! "1a" and "2a" both look like the same recurring varying-digit pattern
//! (e.g. "Page 1"/"Page 2"), even though they are unrelated content on
//! different pages, not chrome. The mis-tag happens purely from reading
//! the document's spans, independent of whether any header/footer/artifact
//! removal is ever requested.

mod common;

use common::build_pdf_with_page_extras;
use xberg_native_pdf::PdfDocument;

/// One label per page ("1a", "2a", "3a", "4a") in the top margin band, each
/// at a DIFFERENT position — mirroring a real-world use case (a form with
/// line-item labels): these labels land at unrelated x/y positions across
/// pages, not the same spot every time, since they're tied to wherever
/// that line item falls in each page's layout, not to a fixed
/// running-header slot. The helper's own "Body text placeholder" line
/// covers "unrelated body text so each page isn't classified as
/// all-chrome".
fn build_form_line_item_label_pdf(labels: &[&str]) -> Vec<u8> {
    let labels: Vec<String> = labels.iter().map(|s| s.to_string()).collect();
    build_pdf_with_page_extras(labels.len(), move |i| {
        // Positions vary widely per page; y stays inside the top margin
        // but also drifts, as in the real corpus data.
        let x = 72.0 + (i as f32) * 90.0;
        let y = 705.0 + (i as f32) * 6.0;
        format!("BT /F1 8 Tf 1 0 0 1 {x} {y} Tm ({}) Tj ET\n", labels[i])
    })
}

/// Distinct form line-item labels ("1a", "2a", "3a", "4a") must never be
/// tagged as a pagination artifact just because they share a
/// digit-normalized signature ("#a") with each other.
#[test]
fn distinct_form_line_item_labels_are_not_tagged_as_pagination() {
    let bytes = build_form_line_item_label_pdf(&["1a", "2a", "3a", "4a"]);
    let doc = PdfDocument::from_bytes(bytes).unwrap();

    for (page_index, label) in ["1a", "2a", "3a", "4a"].iter().enumerate() {
        let spans = doc.extract_spans(page_index).unwrap();
        let span = spans
            .iter()
            .find(|s| s.text.trim() == *label)
            .unwrap_or_else(|| panic!("page {page_index}: expected span {label:?} not found"));
        assert!(
            span.artifact_type.is_none(),
            "page {page_index}: {label:?} was wrongly tagged as {:?} — distinct labels were \
             wrongly treated as the same recurring pattern",
            span.artifact_type
        );
    }
}

/// One label per page, at an explicit (x, y) position — lets a test place
/// two labels close enough together on purpose, unlike
/// `build_form_line_item_label_pdf`'s formula-driven spread.
fn build_form_line_item_label_pdf_at_positions(labels_and_positions: &[(&str, f32, f32)]) -> Vec<u8> {
    let entries: Vec<(String, f32, f32)> = labels_and_positions
        .iter()
        .map(|&(label, x, y)| (label.to_string(), x, y))
        .collect();
    build_pdf_with_page_extras(entries.len(), move |i| {
        let (label, x, y) = &entries[i];
        format!("BT /F1 8 Tf 1 0 0 1 {x} {y} Tm ({label}) Tj ET\n")
    })
}

/// A real-world finding that broke the FIRST attempt at this fix: "1a"
/// (page 0) and "2a" (page 5) coincidentally landed close enough together
/// — 9.76pt/5.4pt apart — even though they're unrelated labels on
/// different pages. Checking only "is there some same-shaped occurrence
/// nearby?" wrongly treats that coincidence as enough to tag both;
/// detection must instead require that a position-consistent group of
/// occurrences independently clears the recurrence threshold — 2
/// occurrences out of 6 pages (a 50% threshold needs 3) must not be
/// enough, even though those 2 happen to be close together.
#[test]
fn coincidentally_colocated_form_line_item_labels_are_not_tagged_as_pagination() {
    let bytes = build_form_line_item_label_pdf_at_positions(&[
        ("1a", 72.0, 710.0),
        ("2a", 75.0, 712.0), // close to "1a", by coincidence
        ("3a", 200.0, 730.0),
        ("4a", 300.0, 715.0),
        ("5a", 400.0, 705.0),
        ("6a", 150.0, 725.0),
    ]);
    let doc = PdfDocument::from_bytes(bytes).unwrap();

    for (page_index, label) in ["1a", "2a", "3a", "4a", "5a", "6a"].iter().enumerate() {
        let spans = doc.extract_spans(page_index).unwrap();
        let span = spans
            .iter()
            .find(|s| s.text.trim() == *label)
            .unwrap_or_else(|| panic!("page {page_index}: expected span {label:?} not found"));
        assert!(
            span.artifact_type.is_none(),
            "page {page_index}: {label:?} was wrongly tagged as {:?} — a coincidental \
             nearby occurrence of a same-shaped label was wrongly treated as enough \
             recurrence to admit it as chrome",
            span.artifact_type
        );
    }
}
