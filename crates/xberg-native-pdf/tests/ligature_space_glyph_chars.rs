//! A space glyph after a ligature glyph, read as single characters. The span path and the character
//! path decide separately whether a space takes no room, and this file covers the character path:
//! `PdfDocument::extract_chars`. The font maps code 1 to the glyph `fi` through `/Differences`. When the
//! content stream pulls the pen back over the whole space, the space lies inside the ligature's box
//! and the character list holds no space. When the space keeps its full advance, the list keeps it.

use xberg_native_pdf::document::PdfDocument;

/// The Helvetica advance of each character the fixture draws, in thousandths of the font size.
fn helvetica_width(code: u8) -> u32 {
    match code {
        1 => 500,
        b' ' | b'f' | b't' | b'.' => 278,
        b'i' | b'l' => 222,
        b'w' => 722,
        b'T' => 611,
        _ => 556,
    }
}

/// One Letter page that draws `show` after four lines of body text, in a font whose code 1 is the
/// ligature glyph `fi`.
fn build_pdf(show: &str) -> Vec<u8> {
    let body = "The committee reviewed every proposal and decided that funding should continue.";
    let mut content = String::new();
    let mut y = 720.0;
    for _ in 0..4 {
        content += &format!("BT /F1 10 Tf 1 0 0 1 72 {y} Tm ({body}) Tj ET\n");
        y -= 14.0;
    }
    content += &format!("BT /F1 10 Tf 1 0 0 1 72 {y} Tm {show} ET\n");

    let widths: Vec<String> = (1u8..=126).map(|code| helvetica_width(code).to_string()).collect();
    let font = format!(
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /FirstChar 1 /LastChar 126 /Widths [{}] \
         /Encoding << /Type /Encoding /BaseEncoding /WinAnsiEncoding /Differences [1 /fi] >> >>",
        widths.join(" ")
    );
    let objects: [Vec<u8>; 5] = [
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R \
          /Resources << /Font << /F1 5 0 R >> >> >>"
            .to_vec(),
        [
            format!("<< /Length {} >>\nstream\n", content.len()).into_bytes(),
            content.into_bytes(),
            b"\nendstream".to_vec(),
        ]
        .concat(),
        font.into_bytes(),
    ];
    let mut pdf = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        pdf.extend_from_slice(object);
        pdf.extend_from_slice(b"\nendobj\n");
    }
    let xref = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes());
    for offset in &offsets {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    pdf
}

/// The characters of the line that holds the word `open`, in the order the document lists them. Only the
/// fixture's last line holds it.
fn last_line_chars(show: &str) -> String {
    let document = PdfDocument::from_bytes(build_pdf(show)).expect("open the fixture");
    let chars = document.extract_chars(0).expect("extract the characters");
    let mut lines: Vec<(f32, String)> = Vec::new();
    for c in &chars {
        match lines.iter_mut().find(|(y, _)| (c.origin_y - *y).abs() < 1.0) {
            Some((_, line)) => line.push(c.char),
            None => lines.push((c.origin_y, c.char.to_string())),
        }
    }
    let mut holding_open = lines.into_iter().filter(|(_, line)| line.contains("open"));
    let (_, line) = holding_open.next().expect("the fixture's last line did not extract");
    assert!(holding_open.next().is_none(), "the word `open` stands on two lines");
    line
}

#[test]
fn chars_keep_a_space_glyph_with_its_own_room_after_a_ligature() {
    assert_eq!(last_line_chars("(The \\001 eld was open.) Tj"), "The fi eld was open.");
}

#[test]
fn chars_drop_a_space_glyph_pulled_back_into_the_ligature() {
    assert_eq!(
        last_line_chars("[(The \\001) 278 ( eld was open.)] TJ"),
        "The field was open."
    );
}

#[test]
fn chars_keep_a_space_that_a_partial_pull_back_leaves_a_gap_for() {
    assert_eq!(
        last_line_chars("[(The \\001) 167 ( eld was open.)] TJ"),
        "The fi eld was open."
    );
}

#[test]
fn chars_drop_a_space_that_a_near_total_pull_back_puts_inside_the_ligature() {
    assert_eq!(
        last_line_chars("[(The \\001) 267 ( eld was open.)] TJ"),
        "The field was open."
    );
}

/// The characters on or just above the fixture's last baseline (664), in the order the document lists
/// them, so a space on a raised baseline is still read with its line.
fn tail_chars(show: &str) -> String {
    let document = PdfDocument::from_bytes(build_pdf(show)).expect("open the fixture");
    let chars = document.extract_chars(0).expect("extract the characters");
    let line: String = chars
        .iter()
        .filter(|c| (660.0..670.0).contains(&c.origin_y))
        .map(|c| c.char)
        .collect();
    line
}

#[test]
fn chars_keep_a_pulled_back_space_that_stands_on_another_baseline() {
    let line = tail_chars("(The \\001) Tj 1 0 0 1 94.23 667 Tm ( eld was open.) Tj");
    assert!(line.contains("The fi") && line.contains(" eld was open."), "{line:?}");
}
