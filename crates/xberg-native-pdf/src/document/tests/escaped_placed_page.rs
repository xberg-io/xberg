use super::super::PdfDocument;
use super::pdf_fixtures::build_catalog_test_pdf;
use crate::content::{Operator, parse_content_stream};

const PLACED_NAMES: [&str; 6] = [
    "PlacedPDF",
    "Placed#50DF",
    "#50lacedPDF",
    "PlacedP#44#46",
    "P#6cacedPDF",
    "#50#6c#61#63#65#64#50#44#46",
];

fn marked_stream(name: &str, operator: &str, body: &str) -> String {
    let properties = if operator == "BDC" { " /MC0" } else { "" };
    format!("/{name}{properties} {operator}\n{body}EMC\n")
}

fn tags(stream: &str) -> Vec<String> {
    parse_content_stream(stream.as_bytes())
        .expect("parse actual marked-content operators")
        .into_iter()
        .filter_map(|operator| match operator {
            Operator::BeginMarkedContent { tag } | Operator::BeginMarkedContentDict { tag, .. } => Some(tag),
            _ => None,
        })
        .collect()
}

fn paint(lines: &[String], top: usize) -> String {
    lines
        .iter()
        .enumerate()
        .map(|(index, line)| format!("BT /F1 12 Tf 20 {} Td ({line}) Tj ET\n", top - index * 18))
        .collect()
}

fn letters(text: &str) -> String {
    text.chars().filter(char::is_ascii_alphabetic).collect()
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
    assert!(
        pdf.windows(b"/BaseFont /Helvetica".len())
            .any(|bytes| bytes == b"/BaseFont /Helvetica")
    );
    let spans_document = PdfDocument::from_bytes(pdf.clone()).expect("open actual one-page PDF");
    assert_eq!(spans_document.page_count().unwrap(), 1);
    assert_eq!(spans_document.get_page_media_box(0).unwrap(), (0.0, 0.0, 612.0, 792.0));
    assert_eq!(spans_document.get_page_content_data(0).unwrap(), stream.as_bytes());
    let spans = spans_document.extract_spans(0).expect("extract actual page spans");
    let chars_document = PdfDocument::from_bytes(pdf).expect("independently open actual char PDF");
    assert_eq!(chars_document.page_count().unwrap(), 1);
    let chars = chars_document.extract_chars(0).expect("extract actual page chars");
    for (font_size, bbox) in spans
        .iter()
        .map(|span| (span.font_size, &span.bbox))
        .chain(chars.iter().map(|character| (character.font_size, &character.bbox)))
    {
        assert_eq!(font_size, 12.0);
        assert!(bbox.x.is_finite() && bbox.y.is_finite() && bbox.width > 0.0 && bbox.height > 0.0);
    }
    (
        spans.iter().map(|span| span.text.as_str()).collect(),
        chars.iter().map(|character| character.char).collect(),
    )
}

#[test]
fn parser_decodes_only_exact_placed_marked_content_names() {
    let distinct = [
        ("Placed#5", "Placed#5"),
        ("Placed#GGDF", "Placed#GGDF"),
        ("Placed#2350DF", "Placed#50DF"),
        ("PlacedPDFExtra", "PlacedPDFExtra"),
        ("xPlacedPDF", "xPlacedPDF"),
        ("placedPDF", "placedPDF"),
        ("Placed#50DF#20", "PlacedPDF "),
    ];
    let mut checked = 0;
    for operator in ["BMC", "BDC"] {
        for name in PLACED_NAMES {
            assert_eq!(
                tags(&marked_stream(name, operator, "")),
                ["PlacedPDF"],
                "{name} {operator}"
            );
            checked += 1;
        }
        for (name, expected) in distinct {
            assert_eq!(
                tags(&marked_stream(name, operator, "")),
                [expected],
                "{name} {operator}"
            );
            assert_ne!(expected, "PlacedPDF");
            checked += 1;
        }
    }
    assert_eq!(checked, 26);
}

#[test]
fn escaped_placed_page_body_matches_literal_in_spans_and_chars() {
    let lines: Vec<String> = (0..30)
        .map(|index| format!("Kestrel paragraph {index} preserves the unique body text"))
        .collect();
    let body = paint(&lines, 720);
    let expected_letters = letters(&lines.concat());
    assert_eq!(lines.len(), 30);
    assert_eq!(expected_letters.len(), 1260);
    assert!(expected_letters.len() > 800);
    let mut checked = 0;
    for operator in ["BMC", "BDC"] {
        let literal = marked_stream("PlacedPDF", operator, &body);
        assert_eq!(tags(&literal), ["PlacedPDF"]);
        let baseline = page_views(&literal);
        for actual in [&baseline.0, &baseline.1] {
            assert_eq!(letters(actual), expected_letters);
            for line in &lines {
                assert!(actual.contains(line), "literal body missing {line}: {actual:?}");
            }
        }
        for name in &PLACED_NAMES[1..] {
            let escaped = marked_stream(name, operator, &body);
            assert_eq!(tags(&escaped), ["PlacedPDF"]);
            let actual = page_views(&escaped);
            assert_eq!(
                actual, baseline,
                "decoded {name} {operator} must preserve the literal page body"
            );
            checked += 1;
        }
    }
    assert_eq!(checked, 10);
}

#[test]
fn escaped_decorative_scope_preserves_only_the_real_outside_body() {
    let outside = "Ordinary outside paragraph remains visible".to_string();
    let label = "Decorative figure label".to_string();
    assert!(label.len() < 800);
    let mut checked = 0;
    for operator in ["BMC", "BDC"] {
        for name in PLACED_NAMES {
            let stream = format!(
                "{}{}",
                paint(std::slice::from_ref(&outside), 720),
                marked_stream(name, operator, &paint(std::slice::from_ref(&label), 660))
            );
            assert_eq!(tags(&stream), ["PlacedPDF"]);
            let actual = page_views(&stream);
            for text in [actual.0, actual.1] {
                assert!(text.contains(&outside), "outside source lost: {text:?}");
                assert_eq!(letters(&text), letters(&outside));
                assert!(!text.contains(&label), "decorative source was retained: {text:?}");
            }
            checked += 1;
        }
    }
    assert_eq!(checked, 12);
}

#[test]
fn ordinary_names_strings_comments_and_property_names_preserve_body() {
    let ordinary = "Ordinary paragraph remains complete".to_string();
    let body = paint(std::slice::from_ref(&ordinary), 720);
    let mut checked = 0;
    for name in [
        "Placed#5",
        "Placed#GGDF",
        "Placed#2350DF",
        "PlacedPDFExtra",
        "xPlacedPDF",
        "placedPDF",
        "Placed#50DF#20",
    ] {
        let stream = marked_stream(name, "BDC", &body);
        assert_ne!(tags(&stream), ["PlacedPDF"]);
        let actual = page_views(&stream);
        for actual in [actual.0, actual.1] {
            assert!(actual.contains(&ordinary));
            assert_eq!(letters(&actual), letters(&ordinary));
        }
        checked += 1;
    }
    let visible_name = "Ordinary literal /Placed#50DF remains complete".to_string();
    let cases = [
        (
            format!("% /Placed#50DF /MC0 BDC\n{body}"),
            ordinary.as_str(),
            Vec::<String>::new(),
        ),
        (
            paint(std::slice::from_ref(&visible_name), 720),
            visible_name.as_str(),
            Vec::new(),
        ),
        (
            format!("/Span /Placed#50DF BDC\n{body}EMC\n"),
            ordinary.as_str(),
            vec!["Span".to_string()],
        ),
    ];
    for (stream, expected, expected_tags) in cases {
        assert_eq!(tags(&stream), expected_tags);
        let actual = page_views(&stream);
        for text in [actual.0, actual.1] {
            assert!(text.contains(expected));
            assert_eq!(letters(&text), letters(expected));
        }
        checked += 1;
    }
    assert_eq!(checked, 10);
}
