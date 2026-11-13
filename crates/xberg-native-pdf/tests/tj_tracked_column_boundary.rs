mod common;

use std::fmt::Write;
use xberg_native_pdf::PdfDocument;

fn document(content: &str) -> PdfDocument {
    PdfDocument::from_bytes(common::build_pdf_with_standard_fonts(
        content.as_bytes(),
        b"/Type /Page /Parent 2 0 R /MediaBox [0 0 612 792]",
    ))
    .unwrap()
}

#[test]
fn tracked_tj_strings_keep_eight_digit_and_description_cells_separate() {
    let mut content = String::from("BT /Helvetica 1 Tf 24.9084 Tc -24.9088 Tw\n");
    for digit in 0..8 {
        let y = 505 - digit * 16;
        writeln!(
            content,
            "9 0 0 9 123.42 {y} Tm [({digit}F) 24908.4 (r) 24908.4 (o) 24908.4 (m) 24908.4] TJ"
        )
        .unwrap();
    }
    content.push_str("ET\n");
    let pdf = document(&content);
    let lines = pdf.extract_text_lines(0).unwrap();
    let words: Vec<_> = lines.iter().flat_map(|line| &line.words).collect();
    assert_eq!(words.len(), 16);
    for digit in 0..8 {
        let y = (505 - digit * 16) as f32;
        let mut row: Vec<_> = words.iter().filter(|word| (word.bbox.y - y).abs() < 0.5).collect();
        row.sort_by(|a, b| a.bbox.left().total_cmp(&b.bbox.left()));
        let [index, description] = row.as_slice() else {
            panic!("expected two cells at y={y}: {row:?}");
        };
        assert_eq!(index.text, digit.to_string());
        assert_eq!(description.text, "From");
        assert!(index.bbox.right() < description.bbox.left());
    }

    let plain = pdf.extract_text(0).unwrap();
    let words: Vec<_> = plain.split_whitespace().collect();
    assert_eq!(words.len(), 16);
    assert_eq!(words.iter().filter(|&&word| word == "From").count(), 8);
    let digits: Vec<_> = words.into_iter().filter(|&word| word != "From").collect();
    assert_eq!(digits, ["0", "1", "2", "3", "4", "5", "6", "7"]);
}

#[test]
fn compensated_tracking_keeps_a_tightly_spaced_word_intact() {
    let pdf = document(
        "BT /Helvetica 1 Tf 24.9084 Tc -24.9088 Tw 9 0 0 9 123.42 505 Tm \
         [(F) 24908.4 (r) 24908.4 (o) 24908.4 (m) 24908.4] TJ ET",
    );
    let lines = pdf.extract_text_lines(0).unwrap();
    let words: Vec<_> = lines
        .iter()
        .flat_map(|line| line.words.iter().map(|word| word.text.as_str()))
        .collect();
    assert_eq!(words, ["From"]);
    assert_eq!(pdf.extract_text(0).unwrap().trim(), "From");
}

#[test]
fn ordinary_tracking_keeps_an_alphanumeric_word_in_one_tj_string() {
    let pdf = document("BT /Helvetica 9 Tf 0.05 Tc 1 0 0 1 123.42 505 Tm [(0From)] TJ ET");
    assert_eq!(pdf.extract_text(0).unwrap().trim(), "0From");
}

fn custom_font_document(content: &str, font: &str, cmap: &str) -> PdfDocument {
    let page = "/Type /Page /Parent 2 0 R /MediaBox [0 0 612 792]";
    let minimal = common::build_minimal_pdf_raw(content.as_bytes(), page.as_bytes());
    let font_start = minimal.windows(8).position(|bytes| bytes == b"5 0 obj\n").unwrap();
    let mut bytes = minimal[..font_start].to_vec();
    let mut offsets = vec![0];
    for id in 1..=4 {
        let marker = format!("{id} 0 obj\n");
        offsets.push(
            bytes
                .windows(marker.len())
                .position(|w| w == marker.as_bytes())
                .unwrap(),
        );
    }
    offsets.push(font_start);
    write!(
        PdfBytes(&mut bytes),
        "5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding {font} >>\nendobj\n"
    )
    .unwrap();
    if !cmap.is_empty() {
        offsets.push(bytes.len());
        write!(
            PdfBytes(&mut bytes),
            "6 0 obj\n<< /Length {} >>\nstream\n{cmap}\nendstream\nendobj\n",
            cmap.len()
        )
        .unwrap();
    }
    PdfDocument::from_bytes(common::finalize_pdf(bytes, &offsets)).unwrap()
}

struct PdfBytes<'a>(&'a mut Vec<u8>);

impl std::fmt::Write for PdfBytes<'_> {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.0.extend_from_slice(s.as_bytes());
        Ok(())
    }
}

fn raw_span_text(pdf: &PdfDocument) -> Vec<String> {
    use xberg_native_pdf::document::ReadingOrder;
    pdf.extract_spans_with_reading_order(0, ReadingOrder::ColumnAware)
        .unwrap()
        .into_iter()
        .map(|span| span.text)
        .collect()
}

#[test]
fn tracked_tj_preserves_repaired_zero_width_in_the_next_glyph_origin() {
    let font = "/FirstChar 65 /LastChar 66 /Widths [0 556]";
    let content = |tracking| format!("BT /F1 9 Tf {tracking} Tc 1 0 0 1 100 505 Tm [(AB)] TJ ET");
    let first_origin_gap = |tracking| {
        let pdf = custom_font_document(&content(tracking), font, "");
        let lines = pdf.extract_text_lines(0).unwrap();
        let words: Vec<_> = lines.iter().flat_map(|line| &line.words).collect();
        let chars: Vec<_> = words.iter().flat_map(|word| &word.chars).collect();
        assert_eq!(chars.iter().map(|glyph| glyph.char).collect::<Vec<_>>(), ['A', 'B']);
        chars[1].bbox.x - chars[0].bbox.x
    };
    let untracked = first_origin_gap(0);
    assert!(untracked > 0.0);
    assert!((first_origin_gap(25) - untracked - 25.0).abs() < 0.01);
}

#[test]
fn tracked_tj_bypasses_skipped_and_multi_scalar_mappings() {
    let cmap = "begincmap\n1 begincodespacerange\n<00> <FF>\nendcodespacerange\n\
                3 beginbfchar\n<41> <00660069>\n<42> <FFFD>\n<43> <0043>\nendbfchar\nendcmap\n";
    let pdf = custom_font_document(
        "BT /F1 9 Tf 25 Tc 1 0 0 1 100 505 Tm [(ABC)] TJ ET",
        "/ToUnicode 6 0 R",
        cmap,
    );
    assert_eq!(raw_span_text(&pdf), ["fiC"]);
}

#[test]
fn tracked_tj_preserves_horizontal_rtl_correction() {
    let cmap = "begincmap\n1 begincodespacerange\n<00> <FF>\nendcodespacerange\n\
                4 beginbfchar\n<41> <05D3>\n<42> <05D2>\n<43> <05D1>\n<44> <05D0>\nendbfchar\nendcmap\n";
    let pdf = custom_font_document(
        "BT /F1 9 Tf 25 Tc 1 0 0 1 100 505 Tm [(ABCD)] TJ ET",
        "/ToUnicode 6 0 R",
        cmap,
    );
    assert_eq!(raw_span_text(&pdf), ["אבגד"]);
}
