use super::super::PdfDocument;
use super::pdf_fixtures::build_catalog_test_pdf;
use crate::content::{Operator, parse_and_execute_text_only, parse_content_stream};
use crate::lexer::{Token, token};
use crate::object::Object;

const BODY: &str = "Actual body retains all original letters";
const ENDINGS: [&str; 3] = ["\n", "\r", "\r\n"];
const NAMES: [&str; 2] = ["PlacedPDF", "Placed#50DF"];

fn paint(text: &str) -> String {
    format!("BT /F1 12 Tf 20 700 Td ({text}) Tj ET\n")
}

fn expected_body(text: &str) -> Vec<Operator> {
    vec![
        Operator::BeginText,
        Operator::Tf {
            font: "F1".to_string(),
            size: 12.0,
        },
        Operator::Td { tx: 20.0, ty: 700.0 },
        Operator::Tj {
            text: text.as_bytes().to_vec(),
        },
        Operator::EndText,
    ]
}

fn qualify_comment(comment: &str, ending: &str) {
    let source = format!("% {comment}{ending}42");
    assert_eq!(token(source.as_bytes()), Ok((&b""[..], Token::Integer(42))));
}

#[allow(deprecated)]
fn all_parser_views(stream: &str) -> [Vec<Operator>; 3] {
    let mut streamed = Vec::new();
    parse_and_execute_text_only(stream.as_bytes(), |operator| {
        streamed.push(operator);
        Ok(())
    })
    .expect("execute actual streaming parser");
    [
        parse_content_stream(stream.as_bytes()).expect("parse actual full stream"),
        streamed,
        crate::content::parser::parse_content_stream_text_only(stream.as_bytes())
            .expect("parse actual buffered text stream"),
    ]
}

#[test]
fn full_parser_treats_operator_shaped_comments_as_trivia() {
    let expected = expected_body(BODY);
    assert_eq!(expected.len(), 5);
    assert_eq!(parse_content_stream(paint(BODY).as_bytes()).unwrap(), expected);
    let mut checked = 0;
    for name in NAMES {
        for ending in ENDINGS {
            let comment = format!("/{name} /MC0 BDC");
            qualify_comment(&comment, ending);
            let source = format!("% {comment}{ending}{}", paint(BODY));
            assert_eq!(
                parse_content_stream(source.as_bytes()).unwrap(),
                expected,
                "comment bytes must never become operators: {name}, {ending:?}"
            );
            checked += 1;
        }
    }
    assert_eq!(checked, 6);
}

#[test]
fn comment_only_and_trailing_eof_streams_emit_no_comment_operators() {
    let expected = expected_body(BODY);
    assert_eq!(parse_content_stream(paint(BODY).as_bytes()).unwrap(), expected);
    let mut checked = 0;
    for comment in ["/PlacedPDF /MC0 BDC", "/Placed#50DF /MC0 BDC", "BT (Fake) Tj ET"] {
        for ending in ["", "\n", "\r", "\r\n"] {
            let source = format!("% {comment}{ending}");
            assert!(source.starts_with("% "));
            assert_eq!(
                parse_content_stream(source.as_bytes()).unwrap(),
                Vec::<Operator>::new(),
                "comment-only bytes must emit no operators: {source:?}"
            );
            let trailing = format!("{}% {comment}{ending}", paint(BODY));
            assert_eq!(
                parse_content_stream(trailing.as_bytes()).unwrap(),
                expected,
                "EOF comment must preserve the actual preceding operators"
            );
            checked += 2;
        }
    }
    assert_eq!(checked, 24);
}

#[test]
fn comments_between_operands_and_operators_preserve_real_arguments() {
    let expected = expected_body(BODY);
    assert_eq!(parse_content_stream(paint(BODY).as_bytes()).unwrap(), expected);
    let mut checked = 0;
    for name in NAMES {
        for ending in ENDINGS {
            let comment = format!("/{name} /MC0 BDC (Fake) Tj");
            qualify_comment(&comment, ending);
            let sources = [
                format!("BT /F1 12 % {comment}{ending}Tf 20 700 Td ({BODY}) Tj ET\n"),
                format!("BT /F1 12 Tf 20 700 Td ({BODY}) % {comment}{ending}Tj ET\n"),
            ];
            for source in sources {
                assert_eq!(
                    parse_content_stream(source.as_bytes()).unwrap(),
                    expected,
                    "a real operand preceding a comment must not be discarded"
                );
                checked += 1;
            }
        }
    }
    assert_eq!(checked, 12);
}

#[test]
fn genuine_tags_and_percent_inside_strings_keep_exact_operator_semantics() {
    let text = "100% genuine /PlacedPDF and /Placed#50DF";
    let expected = expected_body(text);
    assert_eq!(parse_content_stream(paint(text).as_bytes()).unwrap(), expected);
    let mut checked = 0;
    for (name, decoded) in [
        ("PlacedPDF", "PlacedPDF"),
        ("Placed#50DF", "PlacedPDF"),
        ("Span", "Span"),
    ] {
        for operator in ["BMC", "BDC"] {
            let begin = if operator == "BMC" {
                Operator::BeginMarkedContent {
                    tag: decoded.to_string(),
                }
            } else {
                Operator::BeginMarkedContentDict {
                    tag: decoded.to_string(),
                    properties: Box::new(Object::Name("MC0".to_string())),
                }
            };
            let properties = if operator == "BDC" { " /MC0" } else { "" };
            let source = format!("/{name}{properties} {operator}\n{}EMC\n", paint(text));
            let mut marked = vec![begin];
            marked.extend(expected.clone());
            marked.push(Operator::EndMarkedContent);
            assert_eq!(parse_content_stream(source.as_bytes()).unwrap(), marked);
            checked += 1;
        }
    }
    assert_eq!(checked, 6);
}

#[test]
fn full_streaming_and_buffered_parsers_agree_at_comment_boundaries() {
    let expected = expected_body(BODY);
    let expected_views = [expected.clone(), expected.clone(), expected];
    assert_eq!(all_parser_views(&paint(BODY)), expected_views);
    let mut checked = 0;
    for name in NAMES {
        for ending in ENDINGS {
            let comment = format!("/{name} /MC0 BDC");
            qualify_comment(&comment, ending);
            let sources = [
                format!("% {comment}{ending}{}", paint(BODY)),
                format!("BT /F1 12 Tf 20 700 Td % {comment}{ending}({BODY}) Tj ET\n"),
            ];
            for source in sources {
                assert!(source.len() < 256 * 1024);
                let actual = all_parser_views(&source);
                assert_eq!(
                    actual, expected_views,
                    "every parser must preserve the actual five operators"
                );
                checked += 1;
            }
        }
    }
    assert_eq!(checked, 12);
}

fn page_views(stream: &str) -> (String, String) {
    let content = format!("<< /Length {} >>\nstream\n{stream}endstream", stream.len());
    let pdf = build_catalog_test_pdf(
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        &[
            (3, b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>"),
            (4, b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"),
            (5, content.as_bytes()),
        ],
    );
    let spans_document = PdfDocument::from_bytes(pdf.clone()).expect("open actual PDF for spans");
    assert_eq!(spans_document.page_count().unwrap(), 1);
    assert_eq!(spans_document.get_page_media_box(0).unwrap(), (0.0, 0.0, 612.0, 792.0));
    assert_eq!(spans_document.get_page_content_data(0).unwrap(), stream.as_bytes());
    let spans = spans_document.extract_spans(0).expect("extract actual spans");
    let chars_document = PdfDocument::from_bytes(pdf).expect("independently open actual PDF for chars");
    assert_eq!(chars_document.page_count().unwrap(), 1);
    let chars = chars_document.extract_chars(0).expect("extract actual chars");
    assert!(!spans.is_empty() && !chars.is_empty());
    assert!(spans.iter().all(|span| span.font_size == 12.0));
    assert!(chars.iter().all(|character| character.font_size == 12.0));
    (
        spans.iter().map(|span| span.text.as_str()).collect(),
        chars.iter().map(|character| character.char).collect(),
    )
}

#[test]
fn public_page_preserves_body_with_operator_shaped_leading_comments() {
    let text = "Ordinary actual paragraph with 100% genuine text";
    let body = paint(text);
    let baseline = page_views(&body);
    assert_eq!(baseline, (text.to_string(), text.to_string()));
    let mut checked = 0;
    for name in NAMES {
        for ending in ENDINGS {
            let comment = format!("/{name} /MC0 BDC");
            qualify_comment(&comment, ending);
            let source = format!("% {comment}{ending}{body}");
            assert_eq!(
                page_views(&source),
                baseline,
                "actual default public extraction must preserve the source paragraph"
            );
            checked += 1;
        }
    }
    assert_eq!(checked, 6);
}
