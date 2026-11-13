//! The symbol-loss marker (U+FFFE) in extracted text is made by the extractor and never by the PDF.
//!
//! Each input is a hand-built one-page PDF with the same line twice, so the second line reads the
//! font through its cached tables. A font mapping, a UTF-8 literal and /ActualText that author
//! U+FFFE must extract like text that has no Unicode value (U+FFFD), never as the marker.

#![cfg(feature = "pdf")]

mod helpers;

use xberg::core::config::ExtractionConfig;

const MARKER: char = xberg_native_pdf::LOST_SYMBOL_GLYPH_MARKER;

fn pdf(objs: &[Vec<u8>]) -> Vec<u8> {
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (i, body) in objs.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objs.len() + 1).as_bytes());
    for offset in offsets {
        out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objs.len() + 1
        )
        .as_bytes(),
    );
    out
}

fn stream(data: &[u8]) -> Vec<u8> {
    let mut out = format!("<< /Length {} >>\nstream\n", data.len()).into_bytes();
    out.extend_from_slice(data);
    out.extend_from_slice(b"\nendstream");
    out
}

/// A ToUnicode map that sends the code of `A` to `a_value` and leaves `B` and `C` ordinary.
fn to_unicode(a_value: &str) -> Vec<u8> {
    format!(
        "/CIDInit /ProcSet findresource begin 12 dict begin begincmap\n/CMapName /A def /CMapType 2 def\n\
         1 begincodespacerange <00> <FF> endcodespacerange\n\
         3 beginbfchar\n<41> <{a_value}>\n<42> <0042>\n<43> <0043>\nendbfchar\nendcmap end end\n"
    )
    .into_bytes()
}

/// One page, one font `/F1` (Type1 `base_font`, WinAnsi), two lines of `content`.
fn page(base_font: &str, to_unicode: Option<&[u8]>, content: &[u8]) -> Vec<u8> {
    let tu = if to_unicode.is_some() { " /ToUnicode 6 0 R" } else { "" };
    let font = format!("<< /Type /Font /Subtype /Type1 /BaseFont /{base_font} /Encoding /WinAnsiEncoding{tu} >>");
    pdf(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R \
          /Resources << /Font << /F1 5 0 R >> >> >>"
            .to_vec(),
        stream(content),
        font.into_bytes(),
        to_unicode.map_or_else(|| b"null".to_vec(), stream),
    ])
}

fn two_lines(show: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    for y in [700, 650] {
        out.extend_from_slice(format!("BT /F1 24 Tf 72 {y} Td ").as_bytes());
        out.extend_from_slice(show);
        out.extend_from_slice(b" ET\n");
    }
    out
}

/// The settings of a hosted extraction with OCR switched off. ~keep
const CONFIG: &str = r#"{
    "mime_detection_policy": "prefer_content", "use_cache": false, "enable_quality_processing": true,
    "ocr": {"enabled": true, "backend": "tesseract", "language": ["eng"], "auto_rotate": false,
            "vlm_fallback": {"mode": "disabled"}, "numeric_repair": false},
    "force_ocr": false, "ocr_strategy": {"mode": "auto"}, "disable_ocr": true,
    "pages": {"extract_pages": true, "insert_page_markers": false, "keep_native_content": false},
    "result_format": "unified", "output_format": "plain", "include_document_structure": false
}"#;

/// The text of the document and of each stored page, and the warnings the extraction reports. ~keep
fn extract(bytes: &[u8]) -> (String, String) {
    let config: ExtractionConfig = serde_json::from_str(CONFIG).expect("the settings parse");
    let doc = helpers::extract_bytes_document_blocking(bytes, "application/pdf", &config)
        .expect("the synthetic PDF extracts");
    let pages = doc.pages.iter().flatten().map(|page| page.content.as_str());
    let text = std::iter::once(doc.content.as_str())
        .chain(pages)
        .collect::<Vec<_>>()
        .join("\n");
    (text, format!("{:?}", doc.processing_warnings))
}

/// The page with the marker as the PDF writes it must extract like the page with a glyph that has
/// no Unicode value: same text, same warnings. A font that has lost a glyph is a different page. ~keep
fn assert_like_control(authored: &[u8], control: &[u8]) {
    let (text, warnings) = extract(authored);
    let (control_text, control_warnings) = extract(control);
    assert!(!text.contains(MARKER), "the PDF authored the marker: {text:?}");
    assert!(text.contains("BC"), "{text:?}");
    assert_eq!(text, control_text);
    assert_eq!(
        warnings, control_warnings,
        "the PDF made the extractor report a lost glyph"
    );
}

#[test]
fn a_type1_to_unicode_entry_for_fffe_extracts_like_a_glyph_without_a_unicode_value() {
    let show = two_lines(b"(ABC) Tj");
    assert_like_control(
        &page("Helvetica", Some(&to_unicode("FFFE")), &show),
        &page("Helvetica", Some(&to_unicode("FFFD")), &show),
    );
}

#[test]
fn a_type1_to_unicode_entry_for_fffe_in_a_tj_array_extracts_like_a_glyph_without_a_unicode_value() {
    let show = two_lines(b"[(A) -10 (BC)] TJ");
    assert_like_control(
        &page("Helvetica", Some(&to_unicode("FFFE")), &show),
        &page("Helvetica", Some(&to_unicode("FFFD")), &show),
    );
}

#[test]
fn a_utf8_literal_that_holds_fffe_extracts_like_the_literal_without_it() {
    assert_like_control(
        &page("Helvetica", None, &two_lines(b"(\xef\xbf\xbe\xd0\x96BC) Tj")),
        &page("Helvetica", None, &two_lines(b"(\xd0\x96BC) Tj")),
    );
}

#[test]
fn actual_text_that_holds_fffe_extracts_like_the_text_without_it() {
    let marked = |actual: &str| {
        let span =
            |y: i32| format!("/Span << /ActualText <{actual}> >> BDC\nBT /F1 24 Tf 72 {y} Td (ABC) Tj ET\nEMC\n");
        page("Helvetica", None, format!("{}{}", span(700), span(650)).as_bytes())
    };
    assert_like_control(&marked("FEFFFFFE00420043"), &marked("FEFF00420043"));
}
