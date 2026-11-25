use super::super::*;
use super::pdf_fixtures::build_catalog_test_pdf;

const CONTENT: &str = "/P << /MCID 0 /ActualText (InlineWins) >> BDC BT /F1 12 Tf 72 700 Td (glyphs) Tj ET EMC\n\
     /P << /MCID 1 >> BDC BT /F1 12 Tf 72 680 Td (Plain words) Tj ET EMC\n\
     /OC /L1 BDC BT /F1 12 Tf 72 660 Td (Layered words) Tj ET EMC\n";

/// A tagged page whose MCID 0 carries an inline /ActualText while its structure
/// element carries another, beside a default-off optional-content layer.
pub(super) fn tagged_layered_pdf() -> Vec<u8> {
    let content = format!("<< /Length {} >>\nstream\n{CONTENT}\nendstream", CONTENT.len());
    build_catalog_test_pdf(
        b"<< /Type /Catalog /Pages 2 0 R /MarkInfo << /Marked true >> /StructTreeRoot 7 0 R \
          /OCProperties << /OCGs [6 0 R] /D << /OFF [6 0 R] >> >> >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        &[
            (
                3,
                &b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /StructParents 0 \
                  /Resources << /Font << /F1 5 0 R >> /Properties << /L1 6 0 R >> >> >>"[..],
            ),
            (4, content.as_bytes()),
            (
                5,
                &b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"[..],
            ),
            (6, &b"<< /Type /OCG /Name (Hidden) >>"[..]),
            (7, &b"<< /Type /StructTreeRoot /K [8 0 R] >>"[..]),
            (8, &b"<< /Type /StructElem /S /Document /K [9 0 R 10 0 R] >>"[..]),
            (
                9,
                &b"<< /Type /StructElem /S /P /P 8 0 R /Pg 3 0 R /ActualText (StructWins) /K 0 >>"[..],
            ),
            (10, &b"<< /Type /StructElem /S /P /P 8 0 R /Pg 3 0 R /K 1 >>"[..]),
        ],
    )
}

fn fresh_text() -> String {
    let fresh = PdfDocument::from_bytes(tagged_layered_pdf()).expect("open fresh document");
    fresh.extract_text(0).expect("fresh text")
}

/// The MC-scope set changes what this page assembles to, so a stale set is visible.
#[test]
fn the_fixture_text_depends_on_the_mc_actual_text_set() {
    let doc = PdfDocument::from_bytes(tagged_layered_pdf()).expect("open document");
    let spans = doc.extract_spans(0).expect("spans");
    assert_eq!(
        doc.mc_actualtext_mcids.lock().unwrap().get(&0),
        Some(&HashSet::from([0]))
    );
    let options = crate::converters::ConversionOptions::default();
    let with_set = doc.assemble_text_from_spans(0, spans.clone(), &options).unwrap();
    doc.mc_actualtext_mcids.lock().unwrap().remove(&0);
    let without_set = doc.assemble_text_from_spans(0, spans, &options).unwrap();
    assert_ne!(with_set, without_set);
}

/// Another read replaced the page's entry; extract_text served from the
/// postprocessed span cache must still assemble with the set of its own read.
#[test]
fn a_postprocessed_span_hit_replays_the_pages_actual_text_set() {
    let doc = PdfDocument::from_bytes(tagged_layered_pdf()).expect("open document");
    drop(doc.extract_spans(0).expect("spans"));
    doc.mc_actualtext_mcids.lock().unwrap().remove(&0);
    assert_eq!(doc.extract_text(0).expect("text"), fresh_text());
}

#[test]
fn spans_then_a_filtered_read_then_text_equals_a_fresh_document() {
    let doc = PdfDocument::from_bytes(tagged_layered_pdf()).expect("open document");
    drop(doc.extract_spans(0).expect("spans"));
    let hidden = HashSet::from(["Hidden".to_string()]);
    let filtered: String = doc
        .extract_spans_filtered(0, hidden, HashSet::new())
        .expect("filtered spans")
        .iter()
        .map(|span| span.text.as_str())
        .collect();
    assert!(!filtered.contains("Layered"), "{filtered:?}");
    assert_eq!(doc.extract_text(0).expect("text"), fresh_text());
}
