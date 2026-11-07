use super::super::*;
use super::common::*;
use super::pdf_fixtures::*;

#[test]
fn test_document_open_nonexistent_file() {
    let result = PdfDocument::open("/nonexistent/path/to/file.pdf");
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), Error::Io(_)));
}

/// Recursion into a Form XObject invoked by another Form XObject, with
/// no `cm` at either level, must still resolve each `Do`'s operand
/// correctly and extract the innermost text.
#[test]
fn test_nested_xobject_text_extracted_with_orphaned_do_operands() {
    let pdf_bytes = build_nested_xobject_do_with_orphaned_operands_pdf();
    let doc = PdfDocument::from_bytes(pdf_bytes).expect("parse repro pdf");

    let text = doc.extract_text(0).unwrap();
    assert!(text.contains("nested text"), "nested XObject text missing: {text:?}");
}

/// Regression test: a content stream with a degenerate CTM (`a == 0`)
/// and an oversized `Tm` translation literal. The lexer now clamps an
/// overflowing real literal to a finite value (see
/// `lexer::tests::test_parse_oversized_real_clamps_to_finite`), so this
/// specific literal no longer reaches `TjBuffer::new` as `Infinity` and
/// can no longer turn into NaN via `ctm.a * Infinity`. This test is
/// kept as a regression guard for that class of bug — before the lexer
/// clamp, `f64::from_str` *saturated* the all-digit literal to
/// `f64::INFINITY` rather than erroring, and combined with the zero
/// CTM component this became a NaN `bbox.y` in `TjBuffer::new`,
/// panicking `snap_superscript_baselines`'s index sort — the first
/// span sort that runs on every page, before `postprocess_spans`'s
/// off-page filter (which otherwise silently drops any NaN-bbox span
/// and requires a missing `/MediaBox` to bypass, see
/// `build_minimal_pdf_with_font`) — with the exact signature
/// `smallsort.rs: user-provided comparison function does not
/// correctly implement a total order`.
///
/// 320 glyphs with distinct (sub-point-jittered) Y coordinates —
/// matching real OCR/scanned-text bbox noise, so no two glyphs tie in
/// the sort key — emitted in a riffle-shuffled, far-from-sorted
/// order: a near-sorted input lets Rust's pattern-defeating sort skip
/// the internal total-order consistency check entirely, which is why
/// a naive grid/row-major layout would not have reproduced the
/// original panic even with the same NaN present.
#[test]
fn test_nan_bbox_from_oversized_tm_literal_does_not_panic() {
    let n = 320usize;
    let ys: Vec<f32> = (0..n).map(|i| 750.0 - (i as f32) * 1.37).collect();
    let mid = ys.len() / 2;
    let (a, b) = ys.split_at(mid);
    let mut shuffled: Vec<f32> = Vec::with_capacity(ys.len());
    let (mut ai, mut bi) = (a.iter(), b.iter());
    loop {
        match (ai.next(), bi.next()) {
            (Some(x), Some(y)) => {
                shuffled.push(*x);
                shuffled.push(*y);
            }
            (Some(x), None) => shuffled.push(*x),
            (None, Some(y)) => shuffled.push(*y),
            (None, None) => break,
        }
    }

    let mut content = Vec::new();
    content.extend_from_slice(b"BT\n/F1 10 Tf\n");
    for (i, y) in shuffled.iter().enumerate() {
        if i == 1 {
            // The malicious glyph, isolated under its own degenerate
            // CTM (a = 0) so only this glyph's position collapses
            // via `0.0 * Infinity`; ordinary glyphs are unaffected. ~keep
            let huge = "9".repeat(400) + ".0";
            content.extend_from_slice(b"ET\nq\n0 0 0 1 0 0 cm\nBT\n/F1 10 Tf\n");
            content.extend_from_slice(format!("1 0 0 1 {huge} 400 Tm\n(BOOM) Tj\n").as_bytes());
            content.extend_from_slice(b"ET\nQ\nBT\n/F1 10 Tf\n");
        }
        content.extend_from_slice(format!("1 0 0 1 20 {y} Tm\n(X) Tj\n").as_bytes());
    }
    content.extend_from_slice(b"ET\n");

    let pdf_bytes = build_minimal_pdf_with_font(&content);
    let doc = PdfDocument::from_bytes(pdf_bytes).expect("parse repro pdf");
    let result1 = doc.extract_text(0);
    assert!(
        result1.is_ok(),
        "extract_text panicked or errored on NaN bbox coordinate"
    );
    let result2 = doc.extract_spans(0);
    assert!(
        result2.is_ok(),
        "extract_spans panicked or errored on NaN bbox coordinate"
    );
}

#[test]
fn test_from_bytes_empty() {
    let result = PdfDocument::from_bytes(vec![]);
    assert!(result.is_err());
}

#[test]
fn test_page_count_single_page() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    assert_eq!(doc.page_count().unwrap(), 1);
}

#[test]
fn test_page_count_multiple_pages() {
    let pdf = build_multi_page_pdf(5);
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    assert_eq!(doc.page_count().unwrap(), 5);
}

#[test]
fn test_authenticate_unencrypted_pdf() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let result = doc.authenticate(b"anypassword").unwrap();
    assert!(result);
}

#[test]
fn test_extract_text_blank_page() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let text = doc.extract_text(0).unwrap();
    assert!(text.is_empty());
}

#[test]
fn test_extract_text_no_font_resources() {
    let content = b"BT /F1 12 Tf (Hello) Tj ET";
    let pdf = build_minimal_pdf(content);
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    // Should not crash, may return empty or partial text ~keep
    let _text = doc.extract_text(0).unwrap();
}

#[test]
fn test_extract_all_text_multiple_pages() {
    let pdf = build_multi_page_pdf(3);
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let text = doc.extract_all_text().unwrap();
    let page_count = text.matches('\x0c').count();
    assert_eq!(page_count, 2);
}

#[test]
fn test_extract_all_text_single_page() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let text = doc.extract_all_text().unwrap();
    assert!(!text.contains('\x0c'));
}

#[test]
fn test_extract_spans_blank_page() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let spans = doc.extract_spans(0).unwrap();
    assert!(spans.is_empty());
}

#[test]
fn test_extract_spans_no_text_operators() {
    let content = b"100 200 300 400 re S";
    let pdf = build_minimal_pdf(content);
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let spans = doc.extract_spans(0).unwrap();
    assert!(spans.is_empty());
}

#[test]
fn test_decode_stream_with_encryption_null_object() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let result = doc
        .decode_stream_with_encryption(&Object::Null, ObjectRef::new(1, 0))
        .unwrap();
    assert!(result.is_empty());
}

#[test]
fn test_page_cannot_have_text_no_resources() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();

    let page_dict = std::collections::HashMap::new();
    assert!(doc.page_cannot_have_text(&page_dict));
}

#[test]
fn test_page_cannot_have_text_with_font_resources() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();

    let mut font_dict = std::collections::HashMap::new();
    font_dict.insert("F1".to_string(), Object::Reference(ObjectRef::new(10, 0)));

    let mut resources_dict = std::collections::HashMap::new();
    resources_dict.insert("Font".to_string(), Object::Dictionary(font_dict));

    let mut page_dict = std::collections::HashMap::new();
    page_dict.insert("Resources".to_string(), Object::Dictionary(resources_dict));

    assert!(!doc.page_cannot_have_text(&page_dict));
}

#[test]
fn test_page_cannot_have_text_empty_font_dict() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();

    let font_dict = std::collections::HashMap::new();

    let mut resources_dict = std::collections::HashMap::new();
    resources_dict.insert("Font".to_string(), Object::Dictionary(font_dict));

    let mut page_dict = std::collections::HashMap::new();
    page_dict.insert("Resources".to_string(), Object::Dictionary(resources_dict));

    assert!(doc.page_cannot_have_text(&page_dict));
}

#[test]
fn test_extract_paths_blank_page() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let paths = doc.extract_paths(0).unwrap();
    assert!(paths.is_empty());
}

#[test]
fn test_extract_paths_rectangle() {
    let content = b"100 200 300 400 re S";
    let pdf = build_minimal_pdf(content);
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let paths = doc.extract_paths(0).unwrap();
    assert!(!paths.is_empty());
}

#[test]
fn test_get_page_caching() {
    let pdf = build_multi_page_pdf(3);
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let _page1 = doc.get_page(0).unwrap();
    let _page2 = doc.get_page(0).unwrap();
}

#[test]
fn test_get_page_out_of_bounds() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let result = doc.get_page(99);
    assert!(result.is_err());
}

#[test]
#[allow(deprecated)]
fn test_page_count_u32_returns_correct_value() {
    let pdf = build_multi_page_pdf(3);
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    assert_eq!(doc.page_count_u32(), 3);
}

#[test]
fn test_open_with_config() {
    let pdf = build_minimal_pdf(b"");
    let dir = tempfile::tempdir().expect("create temp dir");
    let tmp_path = dir.path().join("native_pdf_test_open_with_config.pdf");
    std::fs::write(&tmp_path, &pdf).unwrap();
    let config = 42u32;
    let result = PdfDocument::open_with_config(&tmp_path, config);
    let _ = std::fs::remove_file(&tmp_path);
    assert!(result.is_ok());
}

#[test]
fn test_extract_paths_in_rect_empty_page() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let region = crate::geometry::Rect {
        x: 0.0,
        y: 0.0,
        width: 612.0,
        height: 792.0,
    };
    let paths = doc.extract_paths_in_rect(0, region).unwrap();
    assert!(paths.is_empty());
}

#[test]
fn test_get_page_ref_out_of_bounds() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let result = doc.get_page_ref(99);
    assert!(result.is_err());
}

/// A ToUnicode CMap that maps char 0x01 → U+FB01 (ﬁ) must produce the letters
/// "fi" in extract_text output (GH#2322): a ligature glyph comes out as its
/// letters at the glyph stage, so a search for "office" matches.
#[test]
fn test_ligature_fi_expanded_in_extract_text() {
    let pdf = build_ligature_fi_pdf();
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let text = doc.extract_text(0).unwrap();
    assert!(text.contains("fi"), "ﬁ must come out as fi; got: {text:?}");
    assert!(!text.contains('\u{FB01}'), "U+FB01 must not reach the text; got: {text:?}");
}

#[test]
fn test_annotation_freetext() {
    let annot = b"4 0 obj\n<< /Type /Annot /Subtype /FreeText /Contents (Hello from annotation) >>\nendobj\n".to_vec();
    let pdf = build_pdf_with_annotations(vec![(4, annot)]);
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let text = doc.extract_text(0).unwrap();
    assert!(text.contains("Hello from annotation"));
}

#[test]
fn test_annotation_text_type() {
    // Text (sticky-note) /Contents is reviewer popup comment text, not visible page
    // content — it must NOT appear in extract_text output (ISO 32000-1 §12.5.6.2). ~keep
    let annot = b"4 0 obj\n<< /Type /Annot /Subtype /Text /Contents (Sticky note) >>\nendobj\n".to_vec();
    let pdf = build_pdf_with_annotations(vec![(4, annot)]);
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    assert!(!doc.extract_text(0).unwrap().contains("Sticky note"));
}

#[test]
fn test_annotation_stamp() {
    let annot = b"4 0 obj\n<< /Type /Annot /Subtype /Stamp /Contents (APPROVED) >>\nendobj\n".to_vec();
    let pdf = build_pdf_with_annotations(vec![(4, annot)]);
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    assert!(doc.extract_text(0).unwrap().contains("APPROVED"));
}

#[test]
fn test_annotation_link() {
    let annot = b"4 0 obj\n<< /Type /Annot /Subtype /Link /Contents (Click here) >>\nendobj\n".to_vec();
    let pdf = build_pdf_with_annotations(vec![(4, annot)]);
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    assert!(doc.extract_text(0).unwrap().contains("Click here"));
}

#[test]
fn test_annotation_highlight() {
    // Highlight annotation /Contents is a user comment on the highlighted
    // text — it is NOT page content and must NOT appear in extract_text output. ~keep
    let annot = b"4 0 obj\n<< /Type /Annot /Subtype /Highlight /Contents (Highlighted) >>\nendobj\n".to_vec();
    let pdf = build_pdf_with_annotations(vec![(4, annot)]);
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    assert!(!doc.extract_text(0).unwrap().contains("Highlighted"));
}

#[test]
fn test_annotation_hidden_flag() {
    let annot = b"4 0 obj\n<< /Type /Annot /Subtype /FreeText /F 2 /Contents (Hidden) >>\nendobj\n".to_vec();
    let pdf = build_pdf_with_annotations(vec![(4, annot)]);
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    assert!(!doc.extract_text(0).unwrap().contains("Hidden"));
}

#[test]
fn test_annotation_invisible_flag() {
    let annot = b"4 0 obj\n<< /Type /Annot /Subtype /FreeText /F 1 /Contents (Invisible) >>\nendobj\n".to_vec();
    let pdf = build_pdf_with_annotations(vec![(4, annot)]);
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    assert!(!doc.extract_text(0).unwrap().contains("Invisible"));
}

#[test]
fn test_annotation_noview_flag() {
    let annot = b"4 0 obj\n<< /Type /Annot /Subtype /Text /F 32 /Contents (NoView) >>\nendobj\n".to_vec();
    let pdf = build_pdf_with_annotations(vec![(4, annot)]);
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    assert!(!doc.extract_text(0).unwrap().contains("NoView"));
}

#[test]
fn test_annotation_unknown_subtype() {
    let annot = b"4 0 obj\n<< /Type /Annot /Subtype /CustomType /Contents (Custom) >>\nendobj\n".to_vec();
    let pdf = build_pdf_with_annotations(vec![(4, annot)]);
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    assert!(doc.extract_text(0).unwrap().contains("Custom"));
}

#[test]
fn test_annotation_multiple() {
    // FreeText /Contents is visible page text; Text (sticky-note) /Contents is popup
    // comment — only FreeText should appear in extract_text output. ~keep
    let a1 = b"4 0 obj\n<< /Type /Annot /Subtype /FreeText /Contents (First) >>\nendobj\n".to_vec();
    let a2 = b"5 0 obj\n<< /Type /Annot /Subtype /Text /Contents (Second) >>\nendobj\n".to_vec();
    let pdf = build_pdf_with_annotations(vec![(4, a1), (5, a2)]);
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let text = doc.extract_text(0).unwrap();
    assert!(text.contains("First"));
    assert!(!text.contains("Second"));
}

#[test]
fn test_annotation_no_subtype() {
    let annot = b"4 0 obj\n<< /Type /Annot /Contents (No subtype) >>\nendobj\n".to_vec();
    let pdf = build_pdf_with_annotations(vec![(4, annot)]);
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    assert!(!doc.extract_text(0).unwrap().contains("No subtype"));
}

#[test]
fn test_annotation_widget_with_value() {
    let annot =
        b"4 0 obj\n<< /Type /Annot /Subtype /Widget /FT /Tx /V (Field value) /Rect [72 700 272 720] >>\nendobj\n"
            .to_vec();
    let pdf = build_pdf_with_annotations(vec![(4, annot)]);
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    assert!(doc.extract_text(0).unwrap().contains("Field value"));
}

#[test]
fn test_extract_text_graphics_only() {
    let pdf = build_minimal_pdf(b"q 1 0 0 1 0 0 cm 100 200 300 400 re S Q");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    assert!(doc.extract_text(0).unwrap().is_empty());
}

#[test]
fn test_extract_text_page_out_of_bounds() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    assert!(doc.extract_text(100).is_err());
}

#[test]
fn test_extract_spans_out_of_bounds() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    assert!(doc.extract_spans(999).is_err());
}

#[test]
fn test_extract_paths_line() {
    let pdf = build_minimal_pdf(b"0 0 m 100 100 l S");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    assert!(!doc.extract_paths(0).unwrap().is_empty());
}

#[test]
fn test_extract_paths_out_of_bounds() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    assert!(doc.extract_paths(999).is_err());
}

#[test]
fn test_extract_paths_curve() {
    let pdf = build_minimal_pdf(b"0 0 m 25 50 75 50 100 0 c S");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    assert!(!doc.extract_paths(0).unwrap().is_empty());
}

#[test]
fn test_extract_paths_filled_rect() {
    let pdf = build_minimal_pdf(b"50 50 200 100 re f");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    assert!(!doc.extract_paths(0).unwrap().is_empty());
}

#[test]
fn test_extract_paths_in_rect_with_content() {
    let pdf = build_minimal_pdf(b"100 200 300 400 re S");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let region = crate::geometry::Rect {
        x: 0.0,
        y: 0.0,
        width: 612.0,
        height: 792.0,
    };
    assert!(!doc.extract_paths_in_rect(0, region).unwrap().is_empty());
}

#[test]
fn test_populate_page_cache_sequential() {
    let pdf = build_multi_page_pdf(5);
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    for i in 0..5 {
        assert!(doc.get_page(i).unwrap().as_dict().is_some());
    }
}

#[test]
fn test_get_page_ref_multi_page() {
    let pdf = build_multi_page_pdf(3);
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let r0 = doc.get_page_ref(0).unwrap();
    let r1 = doc.get_page_ref(1).unwrap();
    let r2 = doc.get_page_ref(2).unwrap();
    assert_ne!(r0.id, r1.id);
    assert_ne!(r1.id, r2.id);
}

#[test]
fn test_decode_stream_with_encryption_non_null() {
    let pdf = build_minimal_pdf(b"BT (Hello) Tj ET");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let stream_obj = doc.load_object(ObjectRef::new(4, 0)).unwrap();
    assert!(
        doc.decode_stream_with_encryption(&stream_obj, ObjectRef::new(4, 0))
            .is_ok()
    );
}

#[test]
fn test_extract_text_annotations_only() {
    let annot = b"4 0 obj\n<< /Type /Annot /Subtype /FreeText /Contents (Only annotation) >>\nendobj\n".to_vec();
    let pdf = build_pdf_with_annotations(vec![(4, annot)]);
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    assert!(doc.extract_text(0).unwrap().contains("Only annotation"));
}

/// A password-protected PDF is detected as encrypted, and text extraction
/// degrades to empty output (warn + empty) rather than erroring — matching
/// pdftotext/PyMuPDF. (`page_count` still surfaces `Error::EncryptedPdf`;
/// see `tests/test_extraction_robustness.rs`.)
#[test]
fn test_encrypted_pdf_extracts_empty_without_password() {
    let pdf_path = "tests/fixtures/encrypted_needs_password.pdf";
    let doc = PdfDocument::open(pdf_path).expect("open should succeed even without password");
    assert!(doc.is_encrypted(), "PDF should be detected as encrypted");

    let text = doc
        .extract_text(0)
        .expect("extract_text degrades to empty, not an error");
    assert!(
        text.is_empty(),
        "undecryptable extraction should be empty, got: {:?}",
        text,
    );
}

/// After authenticating with the correct password, extraction should succeed.
#[test]
fn test_encrypted_pdf_works_after_authentication() {
    let pdf_path = "tests/fixtures/encrypted_needs_password.pdf";
    let doc = PdfDocument::open(pdf_path).expect("open should succeed");
    assert!(doc.is_encrypted());

    let result = doc.authenticate(b"secret").expect("authenticate should not error");
    assert!(result, "Authentication with correct password should succeed");

    let page_count = doc.page_count().expect("page_count should work after auth");
    assert!(page_count > 0, "Should have at least 1 page after auth");

    // extract_text should not error (content may be minimal since it's a test PDF) ~keep
    let _text = doc.extract_text(0).expect("extract_text should work after auth");
}

/// PDFs that are encrypted but authenticated with empty password (the common
/// case for permission-only encryption) must continue to work without error.
#[test]
fn test_encrypted_pdf_with_empty_password_still_works() {
    let pdf_path = "tests/fixtures/encrypted_cid_truetype.pdf";
    let doc = PdfDocument::open(pdf_path).expect("open should succeed");
    // This PDF auto-authenticates with empty password during open() ~keep
    assert!(doc.is_encrypted(), "Should be detected as encrypted");

    let page_count = doc.page_count().expect("page_count should work");
    assert!(page_count > 0);

    let text = doc.extract_text(0).expect("extract_text should work");
    assert!(!text.trim().is_empty(), "Should extract non-empty text");
}

#[test]
fn test_encrypted_pdf_with_compressed_object_streams() {
    // Encrypted PDFs with /Type /ObjStm streams must NOT have those streams
    // decrypted, per ISO 32000-1 Section 7.6.2. Object streams and XRef
    // streams are never individually encrypted; only the overall stream
    // data is compressed. Attempting to decrypt them causes AES errors
    // because the data length is not a multiple of the block size. ~keep
    let pdf_path = "tests/fixtures/encrypted_objstm.pdf";
    let doc = PdfDocument::open(pdf_path).expect("open should succeed for encrypted+objstm PDF");
    assert!(doc.is_encrypted(), "Should be detected as encrypted");

    let page_count = doc.page_count().expect("page_count should work with encrypted objstm");
    assert!(page_count > 0, "Should have at least one page");
}

#[test]
fn test_extract_paths_layer_none_for_plain_stroke() {
    // End-to-end through the real page pipeline: a stroked line on a page
    // with no optional content yields a path whose `layer` is None. Guards
    // the page-level marked-content refactor against perturbing plain
    // extraction (and mirrors the Python shape test's synthetic PDF). ~keep
    let doc = PdfDocument::from_bytes(build_minimal_pdf(b"100 100 m 200 200 l S")).unwrap();
    let paths = doc.extract_paths(0).unwrap();
    assert_eq!(paths.len(), 1);
    assert_eq!(paths[0].layer, None);
}

#[test]
fn test_get_page_rotation_status_absent_is_absent() {
    let doc = PdfDocument::from_bytes(build_pdf_with_rotate_token(None, false)).unwrap();
    assert_eq!(doc.get_page_rotation_status(0).unwrap(), PageRotation::Absent);
}

#[test]
fn test_get_page_rotation_status_zero_is_valid_zero() {
    let doc = PdfDocument::from_bytes(build_pdf_with_rotate_token(Some("0"), false)).unwrap();
    assert_eq!(doc.get_page_rotation_status(0).unwrap(), PageRotation::Valid(0));
}

#[test]
fn test_get_page_rotation_status_90_is_valid_90() {
    let doc = PdfDocument::from_bytes(build_pdf_with_rotate_token(Some("90"), false)).unwrap();
    assert_eq!(doc.get_page_rotation_status(0).unwrap(), PageRotation::Valid(90));
}

#[test]
fn test_get_page_rotation_status_180_is_valid_180() {
    let doc = PdfDocument::from_bytes(build_pdf_with_rotate_token(Some("180"), false)).unwrap();
    assert_eq!(doc.get_page_rotation_status(0).unwrap(), PageRotation::Valid(180));
}

#[test]
fn test_get_page_rotation_status_270_is_valid_270() {
    let doc = PdfDocument::from_bytes(build_pdf_with_rotate_token(Some("270"), false)).unwrap();
    assert_eq!(doc.get_page_rotation_status(0).unwrap(), PageRotation::Valid(270));
}

#[test]
fn test_get_page_rotation_status_negative_90_normalizes_to_270() {
    let doc = PdfDocument::from_bytes(build_pdf_with_rotate_token(Some("-90"), false)).unwrap();
    assert_eq!(doc.get_page_rotation_status(0).unwrap(), PageRotation::Valid(270));
}

#[test]
fn test_get_page_rotation_status_real_value_is_read() {
    let doc = PdfDocument::from_bytes(build_pdf_with_rotate_token(Some("90.0"), false)).unwrap();
    assert_eq!(doc.get_page_rotation_status(0).unwrap(), PageRotation::Valid(90));
}

#[test]
fn test_get_page_rotation_status_non_integral_real_is_malformed() {
    let doc = PdfDocument::from_bytes(build_pdf_with_rotate_token(Some("90.5"), false)).unwrap();
    assert_eq!(doc.get_page_rotation_status(0).unwrap(), PageRotation::Malformed);
}

#[test]
fn test_get_page_rotation_status_non_numeric_is_malformed() {
    let doc = PdfDocument::from_bytes(build_pdf_with_rotate_token(Some("(bogus)"), false)).unwrap();
    assert_eq!(doc.get_page_rotation_status(0).unwrap(), PageRotation::Malformed);
}

#[test]
fn test_get_page_rotation_status_inherited_from_pages_node() {
    let doc = PdfDocument::from_bytes(build_pdf_with_rotate_token(Some("90"), true)).unwrap();
    assert_eq!(doc.get_page_rotation_status(0).unwrap(), PageRotation::Valid(90));
}
