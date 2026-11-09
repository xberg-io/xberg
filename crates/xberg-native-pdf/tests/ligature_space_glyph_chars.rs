//! A space glyph after a ligature glyph, read as single characters. The span path and the character
//! path decide separately whether a space takes no room, and this file covers the character path:
//! `PdfDocument::extract_chars`. The font maps code 1 to the glyph `fi` through `/Differences`. When the
//! content stream pulls the pen back over the whole space, the space lies inside the ligature's box
//! and the character list holds no space. When the space keeps its full advance, the list keeps it.
//! The rule has four conditions besides the ligature: the overlap share, the baseline, and the rotation of
//! the space and of the glyph before it. Each condition has a test on each side of its limit.

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

/// The baseline of the fixture's last line, in points: four body lines of 14 points under y = 720.
const LAST_LINE_Y: f32 = 664.0;
/// The left edge of the last line.
const LINE_X: f32 = 72.0;
/// Where the pen stands after `The \001` in the 10 point font: T 611, h 556, e 556, space 278 and the
/// ligature 500 thousandths. A space that pulls back over its whole width starts 2.78 points before it.
const PEN_AFTER_LIGATURE: f32 = LINE_X + 25.01;
const SPACE_WIDTH: f32 = 2.78;
/// The spaces of `The fi eld was open.` with and without the space after the ligature.
const KEPT: usize = 4;
const DROPPED: usize = 3;

/// The number of space characters the last line of the fixture keeps. The character list is in reading
/// order, not in the order of the content stream, so the test counts the spaces and does not read the
/// line. The four body lines hold ten spaces each.
fn spaces_in_the_last_line(show: &str) -> usize {
    let document = PdfDocument::from_bytes(build_pdf(show)).expect("open the fixture");
    let chars = document.extract_chars(0).expect("extract the characters");
    let body = "The committee reviewed every proposal and decided that funding should continue.";
    let total = chars.iter().filter(|c| c.char == ' ').count();
    let body_spaces = 4 * body.matches(' ').count();
    assert!(
        chars.len() > 4 * body.chars().count(),
        "the fixture's last line did not extract"
    );
    total - body_spaces
}

/// `The fi` and a space glyph with `rest`, set at `y` and pulled back over the whole space.
fn space_pulled_back_over(y: f32, rest: &str) -> String {
    let x = PEN_AFTER_LIGATURE - SPACE_WIDTH;
    format!("(The \\001) Tj 1 0 0 1 {x} {y} Tm {rest}")
}

#[test]
fn chars_keep_a_space_glyph_that_a_partial_pull_back_leaves_room_for() {
    // The pull-back covers 60% of the space: under the share of 90% at which the space counts as inside.
    assert_eq!(spaces_in_the_last_line("[(The \\001) 167 ( eld was open.)] TJ"), KEPT);
}

#[test]
fn chars_drop_a_space_glyph_that_a_near_total_pull_back_puts_inside_the_ligature() {
    // The pull-back covers 96% of the space: over the share of 90%.
    assert_eq!(
        spaces_in_the_last_line("[(The \\001) 267 ( eld was open.)] TJ"),
        DROPPED
    );
}

#[test]
fn chars_drop_a_pulled_back_space_glyph_on_a_baseline_a_half_point_off() {
    // 0.5 points is under a tenth of the 10 point font size: the space still sits on the ligature's line.
    assert_eq!(
        spaces_in_the_last_line(&space_pulled_back_over(LAST_LINE_Y + 0.5, "( eld was open.) Tj")),
        DROPPED
    );
}

#[test]
fn chars_keep_a_pulled_back_space_glyph_on_a_baseline_two_points_off() {
    // 2 points is over a tenth of the font size: the space stands on another line and the gap test decides.
    assert_eq!(
        spaces_in_the_last_line(&space_pulled_back_over(LAST_LINE_Y + 2.0, "( eld was open.) Tj")),
        KEPT
    );
}

#[test]
fn chars_keep_a_pulled_back_space_glyph_in_a_run_turned_90_degrees() {
    // The space after `The` counts too, so the line holds two spaces with the pulled-back one and one without.
    assert_eq!(
        spaces_in_the_last_line(&format!(
            "(The \\001) Tj 0 1 -1 0 {} {LAST_LINE_Y} Tm ( eld) Tj",
            PEN_AFTER_LIGATURE - SPACE_WIDTH
        )),
        2
    );
}

#[test]
fn chars_keep_a_pulled_back_space_glyph_after_a_ligature_in_a_run_turned_90_degrees() {
    // The letters of a ligature in a run turned 90 degrees report an advance of 0 at x = 100, so the
    // upright space must start 2.78 points before x = 100 to lie over the glyph.
    assert_eq!(
        spaces_in_the_last_line(&format!(
            "0 1 -1 0 100 {LAST_LINE_Y} Tm (\\001) Tj 1 0 0 1 {} {LAST_LINE_Y} Tm ( eld) Tj",
            100.0 - SPACE_WIDTH
        )),
        1
    );
}
