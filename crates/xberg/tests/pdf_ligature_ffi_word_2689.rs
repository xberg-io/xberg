//! A word with an `ffi` ligature glyph, in Markdown and plain output with OCR off (#2689, point 4 of the
//! #2630 review). The font draws code 1 as the `ffi` glyph, placed with 0.2 em, inside the
//! 0.15-0.225 em ligature window, on each side of it. The word must stay whole whether the font maps
//! the glyph to U+FB03, to the letters `ffi`, or to nothing the extractor can read.
#![cfg(all(feature = "pdf", feature = "tokio-runtime"))]

mod helpers;

use helpers::extract_bytes_document_blocking;
use xberg::core::config::{ExtractionConfig, OutputFormat};

const BODY: &str = "The committee reviewed every proposal and decided that funding should continue for the coming \
                    year while the remaining questions are settled by the board.";

/// The Helvetica advance of each character the fixture draws, in thousandths of the font size.
fn helvetica_width(code: u8) -> u32 {
    match code {
        1 => 840,
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

fn to_unicode_map(target_hex: &str) -> String {
    format!(
        "/CIDInit /ProcSet findresource begin 12 dict begin begincmap \
         /CMapName /Ligature def 1 begincodespacerange <00> <FF> endcodespacerange \
         1 beginbfchar <01> <{target_hex}> endbfchar endcmap CMapName currentdict /CMap defineresource pop end end"
    )
}

/// One Letter page whose font draws code 1 as the `ffi` glyph, with `to_unicode` mapping it when given.
fn build_pdf(content: &str, to_unicode: Option<&str>, glyph: &str) -> Vec<u8> {
    let widths: Vec<String> = (1u8..=126).map(|code| helvetica_width(code).to_string()).collect();
    let font = format!(
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /FirstChar 1 /LastChar 126 /Widths [{}] \
         /Encoding << /Type /Encoding /BaseEncoding /WinAnsiEncoding /Differences [1 /{glyph}] >>{} >>",
        widths.join(" "),
        if to_unicode.is_some() { " /ToUnicode 6 0 R" } else { "" }
    );
    let map = to_unicode.map(to_unicode_map).unwrap_or_default();
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
        format!("<< /Length {} >>\nstream\n{map}\nendstream", map.len() + 1).into_bytes(),
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

fn last_line(show: &str, output_format: OutputFormat, to_unicode: Option<&str>, glyph: &str) -> String {
    let config = ExtractionConfig {
        output_format,
        disable_ocr: true,
        use_cache: false,
        ..Default::default()
    };
    let bytes = build_pdf(&page_with(show), to_unicode, glyph);
    let result = extract_bytes_document_blocking(&bytes, "application/pdf", &config).expect("fixture must extract");
    assert!(
        result.content.contains("committee reviewed"),
        "the fixture page did not extract: {}",
        result.content
    );
    let content = result.content.split_whitespace().collect::<Vec<_>>().join(" ");
    let start = content.rfind("The ").expect("the ligature line");
    content[start..].to_string()
}

/// "The o", the `ffi` glyph and "ce is open." as three text-matrix placements with 2pt (0.2 em at 10pt)
/// of room on each side of the glyph, as a producer that sets each glyph run by its own matrix leaves it.
/// The glyph is 8.4pt wide, so the placements sit at x = 99.57 and 109.97 on the baseline y = 664.
const WORD_WITH_LIGATURE_ROOM: &str = "(The o) Tj ET\nBT /F1 10 Tf 1 0 0 1 99.57 664 Tm (\\001) Tj ET\nBT /F1 10 Tf 1 0 0 1 109.97 664 Tm (ce is open.) Tj";

/// The ToUnicode target (when any) and the glyph name the /Differences array gives code 1.
const MAPPINGS: [(&str, Option<&str>, &str); 4] = [
    ("U+FB03", Some("FB03"), "ffi"),
    ("the letters ffi", Some("006600660069"), "ffi"),
    ("glyph name ffi, no ToUnicode", None, "ffi"),
    ("glyph name f_f_i, no ToUnicode", None, "f_f_i"),
];

#[test]
fn markdown_keeps_a_word_whole_around_an_ffi_glyph() {
    let lines: Vec<String> = MAPPINGS
        .iter()
        .map(|(name, map, glyph)| {
            format!(
                "{name}: {}",
                last_line(WORD_WITH_LIGATURE_ROOM, OutputFormat::Markdown, *map, glyph)
            )
        })
        .collect();
    assert!(lines.iter().all(|l| l.contains(": The office is open.")), "{lines:#?}");
}

#[test]
fn plain_text_keeps_a_word_whole_around_an_ffi_glyph() {
    let lines: Vec<String> = MAPPINGS
        .iter()
        .map(|(name, map, glyph)| {
            format!(
                "{name}: {}",
                last_line(WORD_WITH_LIGATURE_ROOM, OutputFormat::Plain, *map, glyph)
            )
        })
        .collect();
    assert!(lines.iter().all(|l| l.contains(": The office is open.")), "{lines:#?}");
}

/// Control: with 6pt (0.6 em) on each side the glyph sits between three words, so the 0.2 em above is
/// what the ligature rule joins and not a gap the extractor never sees.
#[test]
fn an_ffi_glyph_with_a_word_gap_on_each_side_stays_between_words() {
    let show = "(The o) Tj ET\nBT /F1 10 Tf 1 0 0 1 103.57 664 Tm (\\001) Tj ET\nBT /F1 10 Tf 1 0 0 1 118.97 664 Tm (ce is open.) Tj";
    let line = last_line(show, OutputFormat::Markdown, Some("FB03"), "ffi");
    assert!(line.starts_with("The o ffi ce is open."), "{line:?}");
}

fn printable_last_line(glyph: &str, to_unicode: Option<&str>) -> String {
    let bytes = build_pdf(&page_with("(The ) Tj (A) Tj ( marker.) Tj"), to_unicode, glyph);
    let mut document = lopdf::Document::load_mem(&bytes).expect("fixture PDF");
    let font = document.get_object_mut((5, 0)).unwrap().as_dict_mut().unwrap();
    font.get_mut(b"Encoding").unwrap().as_dict_mut().unwrap().set(
        "Differences",
        vec![
            lopdf::Object::Integer(i64::from(b'A')),
            lopdf::Object::Name(glyph.as_bytes().to_vec()),
        ],
    );
    font.get_mut(b"Widths").unwrap().as_array_mut().unwrap()[usize::from(b'A') - 1] = 840.into();
    if let Some(target) = to_unicode {
        document
            .get_object_mut((6, 0))
            .unwrap()
            .as_stream_mut()
            .unwrap()
            .set_plain_content(to_unicode_map(target).replace("<01>", "<41>").into_bytes());
    }
    let mut bytes = Vec::new();
    document.save_to(&mut bytes).unwrap();
    let config = ExtractionConfig {
        output_format: OutputFormat::Plain,
        disable_ocr: true,
        use_cache: false,
        ..Default::default()
    };
    let result = extract_bytes_document_blocking(&bytes, "application/pdf", &config).expect("fixture must extract");
    assert!(result.content.contains("committee reviewed"), "{}", result.content);
    let content = result.content.split_whitespace().collect::<Vec<_>>().join(" ");
    content[content.rfind("The ").expect("marker line")..].to_owned()
}

#[test]
fn printable_compound_differences_replace_the_base_encoding_letter() {
    let actual: Vec<_> = ["f_f_i", "a_b", "uni006600660069"]
        .map(|glyph| printable_last_line(glyph, None))
        .into();
    assert_eq!(actual, ["The ffi marker.", "The ab marker.", "The ffi marker."]);
}

#[test]
fn printable_compound_differences_keep_tounicode_authoritative() {
    assert_eq!(printable_last_line("f_f_i", Some("0058")), "The X marker.");
}

#[test]
fn printable_compound_differences_strip_suffixes_and_keep_unknown_base_letters() {
    let actual: Vec<_> = ["A.alt_B", "zzz_yyy", "f_f_i.alt"]
        .map(|glyph| printable_last_line(glyph, None))
        .into();
    assert_eq!(actual, ["The A marker.", "The A marker.", "The ffi marker."]);
}
