use super::super::*;
use super::pdf_fixtures::*;

#[test]
fn test_decode_pdf_text_string_utf16be() {
    let bytes = vec![0xFE, 0xFF, 0x00, 0x41, 0x00, 0x42];
    let result = PdfDocument::decode_pdf_text_string(&bytes);
    assert_eq!(result, "AB");
}

#[test]
fn test_decode_pdf_text_string_utf16le() {
    let bytes = vec![0xFF, 0xFE, 0x41, 0x00, 0x42, 0x00];
    let result = PdfDocument::decode_pdf_text_string(&bytes);
    assert_eq!(result, "AB");
}

#[test]
fn test_decode_pdf_text_string_pdfdoc_encoding() {
    let bytes = vec![0x48, 0x65, 0x6C, 0x6C, 0x6F];
    let result = PdfDocument::decode_pdf_text_string(&bytes);
    assert_eq!(result, "Hello");
}

#[test]
fn test_decode_pdf_text_string_empty() {
    let bytes: Vec<u8> = vec![];
    let result = PdfDocument::decode_pdf_text_string(&bytes);
    assert_eq!(result, "");
}

#[test]
fn test_strip_xhtml_tags_basic() {
    let xhtml = "<p>Hello <b>World</b></p>";
    let result = PdfDocument::strip_xhtml_tags(xhtml);
    assert_eq!(result, "Hello World");
}

#[test]
fn test_strip_xhtml_tags_no_tags() {
    let text = "Plain text without any tags";
    let result = PdfDocument::strip_xhtml_tags(text);
    assert_eq!(result, text);
}

#[test]
fn test_strip_xhtml_tags_empty() {
    assert_eq!(PdfDocument::strip_xhtml_tags(""), "");
}

#[test]
fn test_strip_xhtml_tags_nested() {
    let xhtml = "<div><p><span style='color: red'>Red text</span></p></div>";
    let result = PdfDocument::strip_xhtml_tags(xhtml);
    assert_eq!(result, "Red text");
}

#[test]
fn test_parse_string_value_static_string() {
    let obj = Object::String(b"Hello".to_vec());
    let result = PdfDocument::parse_string_value_static(Some(&obj));
    assert!(result.is_some());
    assert_eq!(result.unwrap(), "Hello");
}

#[test]
fn test_parse_string_value_static_name() {
    let obj = Object::Name("MyName".to_string());
    let result = PdfDocument::parse_string_value_static(Some(&obj));
    assert_eq!(result, Some("MyName".to_string()));
}

#[test]
fn test_parse_string_value_static_integer() {
    let obj = Object::Integer(42);
    let result = PdfDocument::parse_string_value_static(Some(&obj));
    assert_eq!(result, Some("42".to_string()));
}

#[test]
fn test_parse_string_value_static_real() {
    let obj = Object::Real(std::f64::consts::PI);
    let result = PdfDocument::parse_string_value_static(Some(&obj));
    assert!(result.is_some());
    let s = result.unwrap();
    assert!(s.starts_with("3.14"));
}

#[test]
fn test_parse_string_value_static_null() {
    let obj = Object::Null;
    let result = PdfDocument::parse_string_value_static(Some(&obj));
    assert!(result.is_none());
}

#[test]
fn test_parse_string_value_static_none() {
    let result = PdfDocument::parse_string_value_static(None);
    assert!(result.is_none());
}

#[test]
fn test_decode_pdf_text_string_utf8_bom() {
    // PDF 2.0 allows UTF-8 text strings introduced by an EF BB BF BOM.
    let bytes = vec![0xEF, 0xBB, 0xBF, b'H', b'e', b'l', b'l', b'o'];
    let result = PdfDocument::decode_pdf_text_string(&bytes);
    assert_eq!(result, "Hello");
}

#[test]
fn test_decode_pdf_text_string_plain_ascii() {
    let result = PdfDocument::decode_pdf_text_string(b"Hello World");
    assert_eq!(result, "Hello World");
}

#[test]
fn test_decode_pdf_text_string_with_special_chars() {
    let bytes = vec![128u8];
    let result = PdfDocument::decode_pdf_text_string(&bytes);
    assert!(result.contains('\u{2022}'));
}

#[test]
fn test_strip_xhtml_tags_self_closing() {
    assert_eq!(PdfDocument::strip_xhtml_tags("Hello<br/>World"), "HelloWorld");
}

#[test]
fn test_strip_xhtml_tags_with_attributes() {
    assert_eq!(
        PdfDocument::strip_xhtml_tags("<p class=\"body\">Content</p>"),
        "Content"
    );
}

#[test]
fn test_strip_xhtml_tags_multiple() {
    assert_eq!(
        PdfDocument::strip_xhtml_tags("<b>Bold</b> and <i>Italic</i>"),
        "Bold and Italic"
    );
}

#[test]
fn test_parse_string_value_static_boolean() {
    assert!(PdfDocument::parse_string_value_static(Some(&Object::Boolean(true))).is_none());
}

#[test]
fn test_parse_string_value_static_array() {
    assert!(PdfDocument::parse_string_value_static(Some(&Object::Array(vec![]))).is_none());
}

#[test]
fn test_oc_name_ocg_utf16le_bom() {
    // Regression for the reuse of decode_pdf_text_string: the previous
    // inline reader only handled UTF-16BE and fell back to latin-1,
    // mangling UTF-16LE-encoded layer names. The shared helper decodes
    // the LE BOM correctly. ~keep
    let doc = oc_test_doc();
    let dict = ocg_dict(utf16_string("ÁREA-Ø", false));
    assert_eq!(doc.read_oc_name(dict.as_dict().unwrap(), 8).as_deref(), Some("ÁREA-Ø"));
}
