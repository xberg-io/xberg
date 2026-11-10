//! The symbol-loss marker (U+FFFE) is made by the extractor for a glyph of a symbol font that has
//! no Unicode value. A simple font, a UTF-8 literal and `/ActualText` never author it.
//!
//! Each input is a one-page PDF with the same line twice, so the second line reads the font
//! through its cached tables.

mod common;

use std::sync::Mutex;
use xberg_native_pdf::LOST_SYMBOL_GLYPH_MARKER as MARKER;
use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::extractors::text::set_preserve_unmapped_glyphs;

/// `set_preserve_unmapped_glyphs` is a process-wide flag.
static FLAG: Mutex<()> = Mutex::new(());

fn to_unicode(a_value: &str) -> Vec<u8> {
    format!(
        "/CIDInit /ProcSet findresource begin 12 dict begin begincmap\n/CMapName /A def /CMapType 2 def\n\
         1 begincodespacerange <00> <FF> endcodespacerange\n\
         3 beginbfchar\n<41> <{a_value}>\n<42> <0042>\n<43> <0043>\nendbfchar\nendcmap end end\n"
    )
    .into_bytes()
}

fn object(out: &mut Vec<u8>, offsets: &mut Vec<usize>, id: usize, body: &[u8]) {
    offsets.push(out.len());
    out.extend_from_slice(format!("{id} 0 obj\n").as_bytes());
    out.extend_from_slice(body);
    out.extend_from_slice(b"\nendobj\n");
}

fn stream(data: &[u8]) -> Vec<u8> {
    let mut out = format!("<< /Length {} >>\nstream\n", data.len()).into_bytes();
    out.extend_from_slice(data);
    out.extend_from_slice(b"\nendstream");
    out
}

/// One page, font `/F1` (Type1 `base_font`, WinAnsi, ToUnicode when given), `content` as the page.
fn page(base_font: &str, to_unicode: Option<&[u8]>, content: &[u8]) -> Vec<u8> {
    let tu = if to_unicode.is_some() { " /ToUnicode 6 0 R" } else { "" };
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = vec![0usize];
    object(&mut out, &mut offsets, 1, b"<< /Type /Catalog /Pages 2 0 R >>");
    object(&mut out, &mut offsets, 2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>");
    object(
        &mut out,
        &mut offsets,
        3,
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R \
          /Resources << /Font << /F1 5 0 R >> >> >>",
    );
    object(&mut out, &mut offsets, 4, &stream(content));
    let font = format!("<< /Type /Font /Subtype /Type1 /BaseFont /{base_font} /Encoding /WinAnsiEncoding{tu} >>");
    object(&mut out, &mut offsets, 5, font.as_bytes());
    object(
        &mut out,
        &mut offsets,
        6,
        &to_unicode.map_or_else(|| b"null".to_vec(), stream),
    );
    common::finalize_pdf(out, &offsets)
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

fn extract(bytes: Vec<u8>, preserve: bool) -> String {
    let _guard = FLAG.lock().unwrap_or_else(|p| p.into_inner());
    let doc = PdfDocument::from_bytes(bytes).expect("the synthetic PDF parses");
    let previous = set_preserve_unmapped_glyphs(preserve);
    let text = doc.extract_text(0).expect("the page extracts");
    set_preserve_unmapped_glyphs(previous);
    text
}

fn simple(base_font: &str, a_value: &str, preserve: bool) -> String {
    extract(
        page(base_font, Some(&to_unicode(a_value)), &two_lines(b"(ABC) Tj")),
        preserve,
    )
}

#[test]
fn a_to_unicode_entry_for_fffe_reads_as_a_glyph_without_a_unicode_value() {
    for preserve in [false, true] {
        let authored = simple("Helvetica", "FFFE", preserve);
        let control = simple("Helvetica", "FFFD", preserve);
        assert!(!authored.contains(MARKER), "preserve={preserve}: {authored:?}");
        assert_eq!(authored, control, "preserve={preserve}");
        assert!(authored.contains("BC"), "preserve={preserve}: {authored:?}");
    }
}

#[test]
fn a_to_unicode_entry_for_fffe_of_a_symbol_font_is_marked_by_the_extractor() {
    let authored = simple("Wingdings", "FFFE", true);
    let control = simple("Wingdings", "FFFD", true);
    assert_eq!(authored.matches(MARKER).count(), 2, "{authored:?}");
    assert_eq!(authored, control);
    let dropped = simple("Wingdings", "FFFE", false);
    assert!(!dropped.contains(MARKER), "{dropped:?}");
}

#[test]
fn a_tj_array_entry_for_fffe_is_not_the_marker() {
    let bytes = page("Helvetica", Some(&to_unicode("FFFE")), &two_lines(b"[(A) -10 (BC)] TJ"));
    let text = extract(bytes, true);
    assert!(!text.contains(MARKER), "{text:?}");
}

#[test]
fn a_utf8_literal_that_holds_fffe_is_not_the_marker() {
    for preserve in [false, true] {
        let text = extract(
            page("Helvetica", None, &two_lines(b"(\xef\xbf\xbe\xd0\x96BC) Tj")),
            preserve,
        );
        assert!(!text.contains(MARKER), "preserve={preserve}: {text:?}");
        assert!(text.contains("\u{416}BC"), "preserve={preserve}: {text:?}");
    }
}

#[test]
fn a_utf8_literal_in_a_tj_array_that_holds_fffe_is_not_the_marker() {
    for preserve in [false, true] {
        let bytes = page(
            "Helvetica",
            None,
            &two_lines(b"[(\xef\xbf\xbe\xd0\x96) -10 (BC)] TJ"),
        );
        let text = extract(bytes, preserve);
        assert!(!text.contains(MARKER), "preserve={preserve}: {text:?}");
        assert!(text.contains('\u{416}'), "preserve={preserve}: {text:?}");
    }
}

#[test]
fn actual_text_that_holds_fffe_is_not_the_marker() {
    let span =
        |y: i32| format!("/Span << /ActualText <FEFFFFFE00420043> >> BDC\nBT /F1 24 Tf 72 {y} Td (ABC) Tj ET\nEMC\n");
    let content = format!("{}{}", span(700), span(650));
    let text = extract(page("Helvetica", None, content.as_bytes()), true);
    assert!(!text.contains(MARKER), "{text:?}");
    assert!(text.contains("BC"), "{text:?}");
}

#[test]
fn a_to_unicode_entry_for_ffff_keeps_the_authored_character() {
    let text = simple("Helvetica", "FFFF", true);
    assert_eq!(text.matches('\u{FFFF}').count(), 2, "{text:?}");
}
