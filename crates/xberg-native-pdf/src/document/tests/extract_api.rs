use super::super::*;
use super::common::*;
use super::pdf_fixtures::*;

/// A page that draws text directly AND paints an overlay Form XObject
/// invoked without a `cm` (dangling operands ahead of the XObject name)
/// must extract both the direct text and the XObject's text, matching
/// poppler's and pymupdf's behaviour on the same malformed content (both
/// tools resolve `Do`'s name from whatever immediately precedes it,
/// discarding the dangling numeric operands rather than misreading them
/// as the XObject name).
#[test]
fn test_direct_and_overlay_xobject_text_both_extracted_with_orphaned_do_operands() {
    let pdf_bytes = build_xobject_do_with_orphaned_operands_pdf(Some("base body text"));
    let doc = PdfDocument::from_bytes(pdf_bytes).expect("parse repro pdf");

    let chars: String = doc.extract_chars(0).unwrap().iter().map(|c| c.char).collect();
    assert!(chars.contains("base body text"), "missing direct text: {chars:?}");
    assert!(chars.contains("overlay text"), "missing XObject text: {chars:?}");

    let text = doc.extract_text(0).unwrap();
    assert!(text.contains("base body text"));
    assert!(text.contains("overlay text"));

    let plain = doc.extract_text(0).unwrap();
    assert!(plain.contains("base body text"));
    assert!(plain.contains("overlay text"));
}

/// A page with no direct content of its own, whose only text comes from
/// a Form XObject invoked without a `cm`, must not extract as empty.
#[test]
fn test_xobject_only_page_text_extracted_with_orphaned_do_operands() {
    let pdf_bytes = build_xobject_do_with_orphaned_operands_pdf(None);
    let doc = PdfDocument::from_bytes(pdf_bytes).expect("parse repro pdf");

    let chars: String = doc.extract_chars(0).unwrap().iter().map(|c| c.char).collect();
    assert_eq!(chars, "overlay text");

    let text = doc.extract_text(0).unwrap();
    assert!(text.contains("overlay text"), "extract_text came back empty: {text:?}");
}

#[test]
fn test_get_page_content_data_empty_content() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let data = doc.get_page_content_data(0).unwrap();
    // Empty content stream still returns data (may be empty or have a newline) ~keep
    assert!(data.len() <= 2);
}

#[test]
fn test_get_page_content_data_with_content() {
    let content = b"BT /F1 12 Tf (Hello) Tj ET";
    let pdf = build_minimal_pdf(content);
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let data = doc.get_page_content_data(0).unwrap();
    assert!(!data.is_empty());
    let text = String::from_utf8_lossy(&data);
    assert!(text.contains("Hello"));
}

#[test]
fn test_get_page_content_data_blank_page() {
    let pdf = build_multi_page_pdf(1);
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let data = doc.get_page_content_data(0).unwrap();
    assert!(data.is_empty());
}

#[test]
fn test_extract_chars_blank_page() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let chars = doc.extract_chars(0).unwrap();
    assert!(chars.is_empty());
}

#[test]
fn test_apply_intelligent_text_processing_empty() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let spans: Vec<TextSpan> = vec![];
    let result = doc.apply_intelligent_text_processing(spans);
    assert!(result.is_empty());
}

#[test]
fn test_apply_intelligent_text_processing_ligature_preserved() {
    // This stage leaves a ligature code point in a span alone (U+FB01 = ﬁ). The glyph stage
    // already returns letters for a font glyph (GH#2322), so such a span comes from another source. ~keep
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let spans = vec![make_test_span("\u{FB01}nd", 0.0, 0.0, 50.0, 12.0)];
    let result = doc.apply_intelligent_text_processing(spans);
    assert_eq!(result.len(), 1);
    assert!(
        result[0].text.contains('\u{FB01}'),
        "ﬁ must be preserved, got: {:?}",
        result[0].text
    );
}

#[test]
fn test_page_with_array_contents() {
    let mut pdf = b"%PDF-1.4\n".to_vec();

    let off1 = pdf.len();
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    let off2 = pdf.len();
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    let off3 = pdf.len();
    pdf.extend_from_slice(
            b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents [4 0 R 5 0 R] /Resources << >> >>\nendobj\n",
        );

    let content1 = b"q";
    let off4 = pdf.len();
    pdf.extend_from_slice(format!("4 0 obj\n<< /Length {} >>\nstream\n", content1.len()).as_bytes());
    pdf.extend_from_slice(content1);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");

    let content2 = b"Q";
    let off5 = pdf.len();
    pdf.extend_from_slice(format!("5 0 obj\n<< /Length {} >>\nstream\n", content2.len()).as_bytes());
    pdf.extend_from_slice(content2);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");

    let xref_off = pdf.len();
    pdf.extend_from_slice(b"xref\n0 6\n");
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off1).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off2).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off3).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off4).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off5).as_bytes());
    pdf.extend_from_slice(format!("trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", xref_off).as_bytes());

    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let data = doc.get_page_content_data(0).unwrap();
    let text = String::from_utf8_lossy(&data);
    assert!(text.contains("q"));
    assert!(text.contains("Q"));
}

#[test]
fn test_extract_hierarchical_content_blank_page() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let result = doc.extract_hierarchical_content(0);
    // Should not crash, may return Ok(Some) or Ok(None) ~keep
    assert!(result.is_ok());
}

#[test]
fn test_extract_spans_with_config_blank_page() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let config = crate::extractors::SpanMergingConfig::default();
    let spans = doc.extract_spans_with_config(0, config).unwrap();
    assert!(spans.is_empty());
}

#[test]
fn test_extract_chars_out_of_bounds() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    assert!(doc.extract_chars(999).is_err());
}

#[test]
fn test_get_page_content_data_out_of_bounds() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    assert!(doc.get_page_content_data(999).is_err());
}

#[test]
fn test_page_content_indirect_array() {
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let off1 = pdf.len();
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    let off2 = pdf.len();
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    let off3 = pdf.len();
    pdf.extend_from_slice(
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << >> >>\nendobj\n",
    );
    let off4 = pdf.len();
    pdf.extend_from_slice(b"4 0 obj\n[5 0 R 6 0 R]\nendobj\n");
    let c1 = b"q";
    let off5 = pdf.len();
    pdf.extend_from_slice(format!("5 0 obj\n<< /Length {} >>\nstream\n", c1.len()).as_bytes());
    pdf.extend_from_slice(c1);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");
    let c2 = b"Q";
    let off6 = pdf.len();
    pdf.extend_from_slice(format!("6 0 obj\n<< /Length {} >>\nstream\n", c2.len()).as_bytes());
    pdf.extend_from_slice(c2);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");
    let xref_off = pdf.len();
    pdf.extend_from_slice(b"xref\n0 7\n");
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off1).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off2).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off3).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off4).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off5).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off6).as_bytes());
    pdf.extend_from_slice(format!("trailer\n<< /Size 7 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", xref_off).as_bytes());
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let data = doc.get_page_content_data(0).unwrap();
    let text = String::from_utf8_lossy(&data);
    assert!(text.contains("q"));
    assert!(text.contains("Q"));
}

#[test]
fn test_get_page_content_data_null_contents() {
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let off1 = pdf.len();
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    let off2 = pdf.len();
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    let off3 = pdf.len();
    pdf.extend_from_slice(
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents null /Resources << >> >>\nendobj\n",
    );
    let xref_off = pdf.len();
    pdf.extend_from_slice(b"xref\n0 4\n");
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off1).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off2).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off3).as_bytes());
    pdf.extend_from_slice(format!("trailer\n<< /Size 4 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", xref_off).as_bytes());
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    assert!(doc.get_page_content_data(0).unwrap().is_empty());
}

#[test]
fn test_apply_intelligent_text_processing_fl_ligature_preserved() {
    // Same as ﬁ: this stage leaves ﬂ (U+FB02) alone. ~keep
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let spans = vec![make_test_span("\u{FB02}oor", 0.0, 0.0, 50.0, 12.0)];
    let result = doc.apply_intelligent_text_processing(spans);
    assert!(
        result[0].text.contains('\u{FB02}'),
        "ﬂ must be preserved, got: {:?}",
        result[0].text
    );
}

#[test]
fn test_apply_intelligent_text_processing_ocr_font() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let mut span = make_test_span("Test  Text", 0.0, 0.0, 100.0, 12.0);
    span.font_name = "OCR".to_string();
    let result = doc.apply_intelligent_text_processing(vec![span]);
    assert!(!result[0].text.contains("  "));
}

#[test]
fn test_extract_spans_with_config_adaptive() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    assert!(
        doc.extract_spans_with_config(0, crate::extractors::SpanMergingConfig::adaptive())
            .unwrap()
            .is_empty()
    );
}

#[test]
fn test_extract_spans_with_config_out_of_bounds() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    assert!(
        doc.extract_spans_with_config(999, crate::extractors::SpanMergingConfig::default())
            .is_err()
    );
}

#[test]
fn test_extract_page_text_blank_page() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let page_text = doc.extract_page_text(0).unwrap();
    assert!(page_text.spans.is_empty());
    assert!(page_text.chars.is_empty());
    // MediaBox is [0 0 612 792] in build_minimal_pdf ~keep
    assert!((page_text.page_width - 612.0).abs() < 0.1);
    assert!((page_text.page_height - 792.0).abs() < 0.1);
}

#[test]
fn test_extract_page_text_has_page_dimensions() {
    let content = b"BT /F1 12 Tf (Hello) Tj ET";
    let pdf = build_minimal_pdf(content);
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let page_text = doc.extract_page_text(0).unwrap();
    assert!((page_text.page_width - 612.0).abs() < 0.1);
    assert!((page_text.page_height - 792.0).abs() < 0.1);
}

#[test]
fn test_extract_page_text_chars_derived_from_spans() {
    let content = b"BT /F1 12 Tf (Hello) Tj ET";
    let pdf = build_minimal_pdf(content);
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let page_text = doc.extract_page_text(0).unwrap();
    let expected_char_count: usize = page_text.spans.iter().map(|s| s.text.chars().count()).sum();
    assert_eq!(page_text.chars.len(), expected_char_count);
}

#[test]
fn test_extract_page_text_with_column_aware() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let page_text = doc
        .extract_page_text_with_options(0, ReadingOrder::ColumnAware)
        .unwrap();
    assert!(page_text.spans.is_empty());
    assert!((page_text.page_width - 612.0).abs() < 0.1);
}

#[test]
fn test_extract_page_text_out_of_bounds() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let result = doc.extract_page_text(99);
    assert!(result.is_err());
}

/// Regression test: Tm-scale containment filter must not
/// drop distinct text lines whose bounding boxes overlap spatially.
///
/// Before the fix, the containment filter in extract_text() would skip any
/// span geometrically contained within the previous span, even if the text
/// was different. This caused the second line to silently disappear.
///
/// The fix adds a `span.text == prev.text` guard so that only true
/// duplicates are filtered.
#[test]
fn test_containment_filter_preserves_distinct_overlapping_lines() {
    // Build a minimal PDF with two Td-placed text strings at very close Y
    // positions (Y=700 and Y=699 — within the 2.0pt "same line" threshold)
    // but with different content. The first string is wider so the second
    // is geometrically contained within it. ~keep
    let content = b"BT /F1 12 Tf 50 700 Td (First line has longer text here) Tj 0 -1 Td (Second) Tj ET";

    let mut pdf = b"%PDF-1.4\n".to_vec();

    let off1 = pdf.len();
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    let off2 = pdf.len();
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    let off3 = pdf.len();
    pdf.extend_from_slice(
            b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>\nendobj\n",
        );

    let off4 = pdf.len();
    let content_len = content.len();
    pdf.extend_from_slice(format!("4 0 obj\n<< /Length {} >>\nstream\n", content_len).as_bytes());
    pdf.extend_from_slice(content);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");

    let off5 = pdf.len();
    pdf.extend_from_slice(b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n");

    let xref_off = pdf.len();
    pdf.extend_from_slice(b"xref\n0 6\n");
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off1).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off2).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off3).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off4).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off5).as_bytes());
    pdf.extend_from_slice(format!("trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", xref_off).as_bytes());

    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let text = doc.extract_text(0).unwrap();

    assert!(
        text.contains("First line has longer text here"),
        "First line should be present in extracted text, got: {:?}",
        text
    );
    assert!(
        text.contains("Second"),
        "Second line must NOT be dropped by containment filter, got: {:?}",
        text
    );
}

/// `extract_spans`/`extract_words`/`extract_text_lines` must report the
/// resolved `/BaseFont` name ("Helvetica"), not the page's
/// `/Resources/Font` dictionary alias ("F1") — matching what
/// `extract_chars` already did.
#[test]
fn test_span_word_line_font_name_is_resolved_not_alias() {
    let content = b"BT /F1 12 Tf 50 700 Td (Hello) Tj ET";

    let mut pdf = b"%PDF-1.4\n".to_vec();
    let off1 = pdf.len();
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    let off2 = pdf.len();
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    let off3 = pdf.len();
    pdf.extend_from_slice(
            b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>\nendobj\n",
        );
    let off4 = pdf.len();
    pdf.extend_from_slice(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes());
    pdf.extend_from_slice(content);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");
    let off5 = pdf.len();
    pdf.extend_from_slice(b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n");
    let xref_off = pdf.len();
    pdf.extend_from_slice(b"xref\n0 6\n");
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off1).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off2).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off3).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off4).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off5).as_bytes());
    pdf.extend_from_slice(format!("trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", xref_off).as_bytes());

    let doc = PdfDocument::from_bytes(pdf).unwrap();

    let chars = doc.extract_chars(0).unwrap();
    assert_eq!(chars[0].font_name, "Helvetica");

    let spans = doc.extract_spans(0).unwrap();
    assert_eq!(spans[0].font_name, "Helvetica");

    let words = doc.extract_words(0).unwrap();
    assert_eq!(words[0].dominant_font, "Helvetica");

    let lines = doc.extract_text_lines(0).unwrap();
    assert_eq!(lines[0].words[0].dominant_font, "Helvetica");
}

/// `Word.sequence` must reflect content-stream emission order (the
/// originating span's `sequence`), not just reading order, so
/// consumers can tell genuinely-consecutive draw calls apart from
/// spatially-close-but-stream-distant ones (e.g. table cells vs.
/// overlays).
#[test]
fn test_extract_words_sequence_reflects_stream_order() {
    let content = b"BT /F1 12 Tf 50 700 Td (First) Tj 0 -20 Td (Second) Tj ET";

    let mut pdf = b"%PDF-1.4\n".to_vec();
    let off1 = pdf.len();
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    let off2 = pdf.len();
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    let off3 = pdf.len();
    pdf.extend_from_slice(
            b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>\nendobj\n",
        );
    let off4 = pdf.len();
    pdf.extend_from_slice(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes());
    pdf.extend_from_slice(content);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");
    let off5 = pdf.len();
    pdf.extend_from_slice(b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n");
    let xref_off = pdf.len();
    pdf.extend_from_slice(b"xref\n0 6\n");
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off1).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off2).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off3).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off4).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off5).as_bytes());
    pdf.extend_from_slice(format!("trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", xref_off).as_bytes());

    let doc = PdfDocument::from_bytes(pdf).unwrap();

    let words = doc.extract_words(0).unwrap();
    let first = words.iter().find(|w| w.text == "First").unwrap();
    let second = words.iter().find(|w| w.text == "Second").unwrap();
    assert!(
        first.sequence < second.sequence,
        "word drawn first in the content stream must have the smaller sequence: \
             First={}, Second={}",
        first.sequence,
        second.sequence
    );

    let lines = doc.extract_text_lines(0).unwrap();
    let line_words: Vec<&crate::layout::Word> = lines.iter().flat_map(|l| l.words.iter()).collect();
    let first_line_word = line_words.iter().find(|w| w.text == "First").unwrap();
    let second_line_word = line_words.iter().find(|w| w.text == "Second").unwrap();
    assert!(
        first_line_word.sequence < second_line_word.sequence,
        "extract_text_lines words must also carry stream-order sequence"
    );
}

#[test]
fn test_extract_page_text_dimensions_use_extent_not_corner_for_nonzero_origin() {
    // GH#1653: `/MediaBox [10 -100 622 692]` describes a 612x792 page whose
    // lower-left corner sits at (10, -100), not the origin. `page_width` /
    // `page_height` must report the EXTENT (urx - llx, ury - lly) = 612x792,
    // not the raw upper-right corner (622, 692). ~keep
    let pdf = build_minimal_pdf_with_media_box("10 -100 622 692", b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let page_text = doc.extract_page_text(0).unwrap();
    assert!(
        (page_text.page_width - 612.0).abs() < 0.1,
        "expected extent width 612, got {}",
        page_text.page_width
    );
    assert!(
        (page_text.page_height - 792.0).abs() < 0.1,
        "expected extent height 792, got {}",
        page_text.page_height
    );
}

#[test]
fn test_extract_page_text_dimensions_unchanged_for_zero_origin() {
    // Companion to the non-zero-origin test above: when llx == lly == 0 (the
    // overwhelming majority of real PDFs), extent and corner coincide, so this
    // must report the exact same values as before GH#1653's fix. ~keep
    let pdf = build_minimal_pdf_with_media_box("0 0 612 792", b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let page_text = doc.extract_page_text(0).unwrap();
    assert!((page_text.page_width - 612.0).abs() < 0.1);
    assert!((page_text.page_height - 792.0).abs() < 0.1);
}
