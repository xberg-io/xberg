//! A space glyph after a ligature glyph, in Markdown and plain output. The font maps code 1 to the
//! glyph `fi` through `/Differences`, and in one variant also to U+FB01 through `/ToUnicode`. When the
//! content stream pulls the pen back over the whole space, the space is drawn inside the ligature's
//! box and takes no room, so the word stays whole. When the space keeps its full advance, the page
//! shows a gap and the space stays, as in Poppler: the PDF has no mark that tells a gap inside a word
//! from a gap between two words (`staﬀ only`).
#![cfg(all(feature = "pdf", feature = "tokio-runtime"))]

mod helpers;

use helpers::extract_bytes_document_blocking;
use xberg::core::config::{ExtractionConfig, OutputFormat};

const BODY: &str = "The committee reviewed every proposal and decided that funding should continue for the coming \
                    year while the remaining questions are settled by the board.";

/// The Helvetica advance of each character the fixture draws, in thousandths of the font size.
fn helvetica_width(code: u8) -> u32 {
    match code {
        1 => 500,
        b' ' | b'f' | b't' | b'I' | b'.' | b',' => 278,
        b'i' | b'l' | b'j' => 222,
        b'r' => 333,
        b'm' => 833,
        b'w' => 722,
        b'c' | b'k' | b's' | b'v' | b'y' | b'z' | b'x' => 500,
        b'T' => 611,
        b'W' => 944,
        _ => 556,
    }
}

/// A `/ToUnicode` map that sends code 1 to the ligature code point U+FB01.
const LIGATURE_TO_UNICODE: &str = "/CIDInit /ProcSet findresource begin 12 dict begin begincmap \
    /CMapName /Ligature def 1 begincodespacerange <00> <FF> endcodespacerange \
    1 beginbfchar <01> <FB01> endbfchar endcmap CMapName currentdict /CMap defineresource pop end end";

/// One Letter page whose font maps code 1 to the `fi` ligature glyph and carries its own widths.
/// With `to_unicode`, the font also maps code 1 to U+FB01, as many producers do.
fn build_pdf(content: &str, to_unicode: bool) -> Vec<u8> {
    let widths: Vec<String> = (1u8..=126).map(|code| helvetica_width(code).to_string()).collect();
    let font = format!(
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /FirstChar 1 /LastChar 126 /Widths [{}] \
         /Encoding << /Type /Encoding /BaseEncoding /WinAnsiEncoding /Differences [1 /fi] >>{} >>",
        widths.join(" "),
        if to_unicode { " /ToUnicode 6 0 R" } else { "" }
    );
    let objects: [Vec<u8>; 6] = [
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R \
          /Resources << /Font << /F1 5 0 R >> >> >>"
            .to_vec(),
        [
            format!("<< /Length {} >>\nstream\n", content.len()).into_bytes(),
            content.as_bytes().to_vec(),
            b"endstream".to_vec(),
        ]
        .concat(),
        font.into_bytes(),
        format!(
            "<< /Length {} >>\nstream\n{LIGATURE_TO_UNICODE}\nendstream",
            LIGATURE_TO_UNICODE.len() + 1
        )
        .into_bytes(),
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

/// A page of body text with one line that draws `show` (a `Tj` or `TJ` operator) after `The `.
fn page_with(show: &str) -> String {
    let mut page = String::new();
    let mut y = 720.0;
    for _ in 0..4 {
        page += &format!("BT /F1 10 Tf 1 0 0 1 72 {y} Tm ({BODY}) Tj ET\n");
        y -= 14.0;
    }
    page += &format!("BT /F1 10 Tf 1 0 0 1 72 {y} Tm {show} ET\n");
    page
}

fn extract(show: &str, output_format: OutputFormat, to_unicode: bool) -> String {
    let config = ExtractionConfig {
        output_format,
        disable_ocr: true,
        use_cache: false,
        ..Default::default()
    };
    let bytes = build_pdf(&page_with(show), to_unicode);
    let result = extract_bytes_document_blocking(&bytes, "application/pdf", &config).expect("fixture must extract");
    assert!(
        result.content.contains("committee reviewed"),
        "the fixture page did not extract: {}",
        result.content
    );
    result.content.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The ligature, then a space glyph with its full advance, then the rest of the word.
const SPACE_WITH_ROOM: &str = "(The \\001 eld was open.) Tj";
/// The same text, with a pull-back of the whole space width (278) between the ligature and the space.
const SPACE_PULLED_BACK: &str = "[(The \\001) 278 ( eld was open.)] TJ";

/// The last line of the page, for the font with and without the `/ToUnicode` map.
fn ligature_lines(show: &str, output_format: OutputFormat) -> [String; 2] {
    [false, true].map(|to_unicode| {
        let content = extract(show, output_format.clone(), to_unicode);
        let start = content.rfind("The ").expect("the ligature line");
        content[start..].to_string()
    })
}

#[test]
fn markdown_keeps_a_space_glyph_with_its_own_room_after_a_ligature() {
    assert_eq!(
        ligature_lines(SPACE_WITH_ROOM, OutputFormat::Markdown),
        ["The fi eld was open."; 2].map(String::from)
    );
}

#[test]
fn plain_keeps_a_space_glyph_with_its_own_room_after_a_ligature() {
    assert_eq!(
        ligature_lines(SPACE_WITH_ROOM, OutputFormat::Plain),
        ["The fi eld was open."; 2].map(String::from)
    );
}

#[test]
fn markdown_drops_a_space_glyph_pulled_back_into_the_ligature() {
    assert_eq!(
        ligature_lines(SPACE_PULLED_BACK, OutputFormat::Markdown),
        ["The field was open."; 2].map(String::from)
    );
}

#[test]
fn plain_drops_a_space_glyph_pulled_back_into_the_ligature() {
    assert_eq!(
        ligature_lines(SPACE_PULLED_BACK, OutputFormat::Plain),
        ["The field was open."; 2].map(String::from)
    );
}
