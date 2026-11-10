//! Synthetic PDF encoding controls for #2858: the base letter differs from the raw byte.
#![cfg(all(feature = "pdf", feature = "tokio-runtime"))]

mod helpers;

use helpers::extract_bytes_document_blocking;
use lopdf::{Document, Object, Stream, dictionary};
use xberg::core::config::{ExtractionConfig, OutputFormat};

const BODY: &str = "The committee reviewed every proposal and decided that funding should continue for the coming \
                    year while the remaining questions are settled by the board.";
const ENCODINGS: [(&str, u8, &str); 2] = [("WinAnsiEncoding", 0x8a, "Š"), ("MacRomanEncoding", 0x80, "Ä")];

fn build_pdf(base_encoding: &str, code: u8, glyph: Option<&str>) -> Vec<u8> {
    let mut document = Document::with_version("1.7");
    let pages = document.new_object_id();
    let mut encoding = dictionary! { "Type" => "Encoding", "BaseEncoding" => base_encoding };
    if let Some(glyph) = glyph {
        encoding.set(
            "Differences",
            vec![
                Object::Integer(i64::from(code)),
                Object::Name(glyph.as_bytes().to_vec()),
            ],
        );
    }
    let font = document.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica", "Encoding" => encoding,
    });
    let content = content_bytes(code);
    let stream = document.add_object(Stream::new(dictionary! {}, content.clone()));
    let page = document.add_object(dictionary! {
        "Type" => "Page", "Parent" => pages, "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
        "Resources" => dictionary! { "Font" => dictionary! { "F1" => font } }, "Contents" => stream,
    });
    document.objects.insert(
        pages,
        dictionary! { "Type" => "Pages", "Kids" => vec![Object::Reference(page)], "Count" => 1 }.into(),
    );
    let catalog = document.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
    document.trailer.set("Root", catalog);
    let mut bytes = Vec::new();
    document.save_to(&mut bytes).expect("serialize synthetic PDF");
    let parsed = Document::load_mem(&bytes).expect("synthetic PDF must parse");
    assert_eq!(parsed.get_pages().len(), 1);
    let mut roundtripped = parsed.get_page_content(page);
    // lopdf separates concatenated page content streams with a trailing newline.
    assert_eq!(roundtripped.pop(), Some(b'\n'));
    assert_eq!(roundtripped, content);
    bytes
}

fn content_bytes(code: u8) -> Vec<u8> {
    let mut content = String::new();
    for line in 0..4 {
        let y = 720 - line * 14;
        content += &format!("BT /F1 10 Tf 1 0 0 1 72 {y} Tm ({BODY}) Tj ET\n");
    }
    content += &format!("BT /F1 10 Tf 1 0 0 1 72 664 Tm (The \\{code:03o} marker.) Tj ET\n");
    content.into_bytes()
}

fn assert_marker(glyph: Option<&str>, replacement: Option<&str>) {
    let mut actual = Vec::new();
    let mut expected = Vec::new();
    for (encoding, code, base_letter) in ENCODINGS {
        assert_ne!(char::from(code).to_string(), base_letter);
        for output_format in [OutputFormat::Plain, OutputFormat::Markdown] {
            let config = ExtractionConfig {
                output_format: output_format.clone(),
                disable_ocr: true,
                use_cache: false,
                ..Default::default()
            };
            let bytes = build_pdf(encoding, code, glyph);
            let result = extract_bytes_document_blocking(&bytes, "application/pdf", &config)
                .expect("synthetic PDF must extract through the public API");
            let content = result.content.split_whitespace().collect::<Vec<_>>().join(" ");
            let context = format!("{encoding}, {code:#x}, {glyph:?}, {output_format:?}");
            actual.push((context.clone(), content));
            let marker = replacement.unwrap_or(base_letter);
            expected.push((context, format!("{} The {marker} marker.", [BODY; 4].join(" "))));
        }
    }
    assert_eq!(actual.len(), 4);
    assert_eq!(actual, expected);
}

#[test]
fn should_preserve_nonidentity_base_letters_without_differences() {
    assert_marker(None, None);
}

#[test]
fn should_preserve_nonidentity_base_letters_when_glyph_name_is_unknown() {
    assert_marker(Some("NoSuchGlyph2858"), None);
}

#[test]
fn should_preserve_nonidentity_base_letters_when_compound_name_is_unknown() {
    assert_marker(Some("zzz_yyy"), None);
}

#[test]
fn should_replace_nonidentity_base_letters_when_single_name_is_known() {
    assert_marker(Some("X"), Some("X"));
}

#[test]
fn should_replace_nonidentity_base_letters_when_compound_name_is_known() {
    assert_marker(Some("f_f_i"), Some("ffi"));
}
