//! Page-space geometry must be mapped alongside the spans it is compared to.
//!
//! A landscape table typeset on an upright page carries a dominant text-matrix
//! rotation, and the row-major assembler only reads it correctly once the spans
//! are rotated upright. That map is applied inside the converters, and it used
//! to be an unwritten convention of whichever local variable held the mapped
//! spans — so every other page-space value a converter compared them against
//! stayed in the frame the file wrote it in.
//!
//! ISO 32000-1:2008 gives all of them page space: §12.5.2 puts an annotation's
//! `/Rect` "in default user space", and the table geometry comes from the same
//! page-space words and paths. None of that follows the spans through the map,
//! so each comparison was between two different frames and simply never
//! matched.
//!
//! Each test below pairs a rotated page with an otherwise identical upright
//! one. The upright page is the control: it fixes what the feature is supposed
//! to produce, so a test cannot pass by the feature being broken everywhere.

use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::converters::ConversionOptions;

/// A page whose text is drawn either upright or under a 90° text matrix, with
/// a ruled 2x2 grid around it and a `/Link` over the first run.
///
/// The page itself is `/Rotate 0` in both cases: the rotation that triggers the
/// reading-frame map is the *text*'s, not the page's.
fn page(rotated: bool, link_rect: &str, grid: bool) -> Vec<u8> {
    let tm = if rotated { "0 1 -1 0" } else { "1 0 0 1" };
    let mut content = String::new();
    if grid {
        for x in [100, 150, 200] {
            content.push_str(&format!("0.7 w {x} 300 m {x} 600 l S\n"));
        }
        for y in [300, 450, 600] {
            content.push_str(&format!("0.7 w 100 {y} m 200 {y} l S\n"));
        }
    }
    for (x, y, t) in [
        (120, 320, "Alpha"),
        (120, 470, "Beta"),
        (170, 320, "Gamma"),
        (170, 470, "Delta"),
    ] {
        content.push_str(&format!("BT /F1 10 Tf {tm} {x} {y} Tm ({t}) Tj ET\n"));
    }
    let content = content.into_bytes();

    let mut buf: Vec<u8> = Vec::new();
    let mut off = vec![0usize; 7];
    let obj = |buf: &mut Vec<u8>, off: &mut Vec<usize>, id: usize, body: &str| {
        off[id] = buf.len();
        buf.extend_from_slice(format!("{id} 0 obj\n{body}\nendobj\n").as_bytes());
    };

    buf.extend_from_slice(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n");
    obj(&mut buf, &mut off, 1, "<< /Type /Catalog /Pages 2 0 R >>");
    obj(&mut buf, &mut off, 2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>");
    obj(
        &mut buf,
        &mut off,
        3,
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Rotate 0 \
         /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R /Annots [6 0 R] >>",
    );
    off[4] = buf.len();
    buf.extend_from_slice(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes());
    buf.extend_from_slice(&content);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    obj(
        &mut buf,
        &mut off,
        5,
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
    );
    obj(
        &mut buf,
        &mut off,
        6,
        &format!(
            "<< /Type /Annot /Subtype /Link /Rect {link_rect} /Border [0 0 0] \
             /A << /S /URI /URI (https://example.invalid/alpha) >> >>"
        ),
    );

    let xref = buf.len();
    buf.extend_from_slice(b"xref\n0 7\n0000000000 65535 f \n");
    for offset in &off[1..=6] {
        buf.extend_from_slice(format!("{:010} 00000 n \n", *offset).as_bytes());
    }
    buf.extend_from_slice(b"trailer\n<< /Size 7 /Root 1 0 R >>\nstartxref\n");
    buf.extend_from_slice(format!("{xref}\n%%EOF\n").as_bytes());
    buf
}

const ROTATED_LINK: &str = "[113 315 133 350]";

fn text(pdf: Vec<u8>) -> String {
    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    doc.extract_text(0).expect("text")
}

/// Control: an upright page renders its grid and does not also emit the cell
/// text as prose.
#[test]
fn test_upright_page_emits_each_cell_once() {
    let t = text(page(false, "[115 315 150 335]", true));
    assert_eq!(t.matches("Alpha").count(), 1, "control:\n{t}");
}

/// The defect: the table's boxes stayed in page space, so no cell could claim
/// the spans it renders and every one was emitted a second time beside it.
#[test]
fn test_rotated_frame_page_emits_each_cell_once() {
    let t = text(page(true, ROTATED_LINK, true));
    for word in ["Alpha", "Beta", "Gamma", "Delta"] {
        assert_eq!(
            t.matches(word).count(),
            1,
            "{word} was emitted both by the table and as flow text beside it:\n{t}"
        );
    }
}

/// A page with two labelled text fields, whose widget rectangles sit where the
/// values read next to their own labels once the page is turned upright.
fn form_page(rotated: bool) -> Vec<u8> {
    let tm = if rotated { "0 1 -1 0" } else { "1 0 0 1" };
    let mut content = String::new();
    for (x, y, t) in [(120, 320, "Name"), (170, 320, "Date")] {
        content.push_str(&format!("BT /F1 10 Tf {tm} {x} {y} Tm ({t}) Tj ET\n"));
    }
    let content = content.into_bytes();

    // Page-space rectangles. Under the 90° map (x, y) -> (y, 612 - x), the
    // rotated page's two rects land to the right of their own labels.
    let (r1, r2) = if rotated {
        ("[115 395 135 460]", "[165 395 185 460]")
    } else {
        ("[160 315 260 335]", "[210 315 310 335]")
    };

    let mut buf: Vec<u8> = Vec::new();
    let mut off = vec![0usize; 8];
    let obj = |buf: &mut Vec<u8>, off: &mut Vec<usize>, id: usize, body: &str| {
        off[id] = buf.len();
        buf.extend_from_slice(format!("{id} 0 obj\n{body}\nendobj\n").as_bytes());
    };

    buf.extend_from_slice(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n");
    obj(
        &mut buf,
        &mut off,
        1,
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [6 0 R 7 0 R] >> >>",
    );
    obj(&mut buf, &mut off, 2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>");
    obj(
        &mut buf,
        &mut off,
        3,
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Rotate 0 \
         /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R /Annots [6 0 R 7 0 R] >>",
    );
    off[4] = buf.len();
    buf.extend_from_slice(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes());
    buf.extend_from_slice(&content);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    obj(
        &mut buf,
        &mut off,
        5,
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
    );
    obj(
        &mut buf,
        &mut off,
        6,
        &format!("<< /Type /Annot /Subtype /Widget /FT /Tx /T (nameField) /V (ADA) /Rect {r1} >>"),
    );
    obj(
        &mut buf,
        &mut off,
        7,
        &format!("<< /Type /Annot /Subtype /Widget /FT /Tx /T (dateField) /V (ZULU) /Rect {r2} >>"),
    );

    let xref = buf.len();
    buf.extend_from_slice(b"xref\n0 8\n0000000000 65535 f \n");
    for offset in &off[1..=7] {
        buf.extend_from_slice(format!("{:010} 00000 n \n", *offset).as_bytes());
    }
    buf.extend_from_slice(b"trailer\n<< /Size 8 /Root 1 0 R >>\nstartxref\n");
    buf.extend_from_slice(format!("{xref}\n%%EOF\n").as_bytes());
    buf
}

fn form_text(rotated: bool) -> String {
    let doc = PdfDocument::from_bytes(form_page(rotated)).expect("parse");
    let opts = ConversionOptions {
        include_form_fields: true,
        ..Default::default()
    };
    doc.extract_text_with_options(0, &opts).expect("text")
}

/// Control: on an upright page each value reads after its own label.
#[test]
fn test_upright_page_reads_each_field_value_after_its_label() {
    let t = form_text(false);
    let (name, ada) = (t.find("Name").unwrap(), t.find("ADA").unwrap());
    let (date, zulu) = (t.find("Date").unwrap(), t.find("ZULU").unwrap());
    assert!(name < ada && ada <= date && date < zulu, "control:\n{t}");
}

/// The defect: a widget `/Rect` is page-space, so appending unmapped widget
/// spans to mapped page spans put two frames in one vector. Both values
/// detached from their labels and collected at the end of the page.
#[test]
fn test_rotated_frame_page_reads_each_field_value_after_its_label() {
    let t = form_text(true);
    let (name, ada) = (t.find("Name").unwrap(), t.find("ADA").unwrap());
    let (date, zulu) = (t.find("Date").unwrap(), t.find("ZULU").unwrap());
    assert!(
        name < ada && ada < date && date < zulu,
        "the field values did not follow their own labels, so a value reads \
         against the wrong field:\n{t}"
    );
}
