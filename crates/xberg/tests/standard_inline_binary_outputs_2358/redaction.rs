use super::*;
use xberg::core::config::redaction::{RedactionConfig, RedactionTerm};
use xberg::types::redaction::RedactionStrategy;

#[derive(Clone, Copy)]
struct Case {
    term: &'static str,
    plain: &'static str,
    raised: &'static str,
    underlined: &'static str,
}

const CASES: [Case; 4] = [
    Case {
        term: "sup",
        plain: "x2 and under end",
        raised: "2",
        underlined: "under",
    },
    Case {
        term: "u",
        plain: "x2 and [REDACTED]nder end",
        raised: "2",
        underlined: "[REDACTED]nder",
    },
    Case {
        term: "2",
        plain: "x[REDACTED] and under end",
        raised: "[REDACTED]",
        underlined: "under",
    },
    Case {
        term: "under",
        plain: "x2 and [REDACTED] end",
        raised: "2",
        underlined: "[REDACTED]",
    },
];

async fn redacted(source: &[u8], mime: &str, format: OutputFormat, term: &str) -> String {
    let config = ExtractionConfig {
        output_format: format,
        use_cache: false,
        redaction: Some(RedactionConfig {
            strategy: RedactionStrategy::Mask,
            custom_terms: vec![RedactionTerm::labeled("private", term)],
            ..Default::default()
        }),
        ..Default::default()
    };
    let response = extract(ExtractInput::from_bytes(source, mime, None), &config)
        .await
        .expect("actual redacted public extraction");
    assert_eq!(response.results.len(), 1);
    let content = response.results.into_iter().next().unwrap().content;
    assert!(!content.is_empty());
    content
}

fn package_xml(encoded: &str) -> String {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .expect("actual binary package");
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).expect("actual DOCX archive");
    assert!(archive.len() >= 6);
    let mut xml = String::new();
    archive
        .by_name("word/document.xml")
        .unwrap()
        .read_to_string(&mut xml)
        .unwrap();
    xml
}

fn pdf_content(encoded: &str) -> (String, Content) {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .expect("actual binary output");
    assert!(bytes.starts_with(b"%PDF-"));
    let document = lopdf::Document::load_mem(&bytes).expect("actual PDF");
    let pages = document.get_pages();
    assert_eq!(pages.len(), 1);
    let text = document.extract_text(&[1]).expect("selectable actual text");
    let content =
        Content::decode(&document.get_page_content(pages[&1])).expect("actual operations");
    assert!(!content.operations.is_empty());
    (text, content)
}

async fn docx_case(case: Case) {
    let output = redacted(
        SEMANTIC,
        "text/html",
        OutputFormat::Custom("docx".into()),
        case.term,
    )
    .await;
    let xml = package_xml(&output);
    let doc = roxmltree::Document::parse(&xml).expect("actual Word XML");
    let runs: Vec<_> = doc
        .descendants()
        .filter(|node| node.has_tag_name((WORD_NS, "r")))
        .collect();
    assert!(!runs.is_empty());
    assert_eq!(text_of(doc.root()), case.plain, "term {}", case.term);
    for (name, value, expected) in [
        ("vertAlign", "superscript", case.raised),
        ("u", "single", case.underlined),
    ] {
        let styled: String = runs
            .iter()
            .filter(|node| property(**node, name, value))
            .map(|node| text_of(*node))
            .collect();
        assert_eq!(styled, expected, "term {} / {name}", case.term);
    }
}

async fn pdf_case(case: Case) {
    let output = redacted(
        SEMANTIC,
        "text/html",
        OutputFormat::Custom("pdf".into()),
        case.term,
    )
    .await;
    let (text, content) = pdf_content(&output);
    assert_eq!(compact(&text), compact(case.plain), "term {}", case.term);
    assert!(content.operations.iter().any(|op| op.operator == "Ts"
        && op
            .operands
            .first()
            .and_then(|v| v.as_float().ok())
            .is_some_and(|rise| rise > 0.0)));
    assert_eq!(
        content
            .operations
            .iter()
            .filter(|op| op.operator == "l")
            .count(),
        1
    );
}

async fn markdown_case(case: Case) {
    let source = converted(
        "<p>éx<sup>2</sup> and <u>under</u> end</p>".as_bytes(),
        "docx",
    )
    .await;
    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        use_cache: false,
        ..Default::default()
    };
    let baseline = extract(
        ExtractInput::from_bytes(
            source.as_slice(),
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            None,
        ),
        &config,
    )
    .await
    .expect("real styled DOCX source baseline");
    assert_eq!(baseline.results.len(), 1);
    assert_eq!(
        baseline.results[0].content.trim(),
        "éx<sup>2</sup> and <u>under</u> end"
    );
    let output = redacted(
        &source,
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        OutputFormat::Markdown,
        case.term,
    )
    .await;
    assert_eq!(
        output.trim(),
        format!(
            "éx<sup>{}</sup> and <u>{}</u> end",
            case.raised, case.underlined
        )
    );
}

macro_rules! redaction_cases {
    ($check:ident; $($name:ident: $index:expr),+ $(,)?) => {$(
        #[tokio::test]
        #[serial_test::serial]
        async fn $name() { $check(CASES[$index]).await; }
    )+};
}

redaction_cases!(docx_case;
    docx_tag_name_sup_does_not_destroy_styles: 0,
    docx_tag_name_u_redacts_content_and_preserves_styles: 1,
    docx_exponent_content_is_redacted_and_stays_raised: 2,
    docx_underlined_content_is_redacted_and_stays_underlined: 3,
);
redaction_cases!(pdf_case;
    pdf_tag_name_sup_does_not_destroy_styles: 0,
    pdf_tag_name_u_redacts_content_and_preserves_styles: 1,
    pdf_exponent_content_is_redacted_and_stays_raised: 2,
    pdf_underlined_content_is_redacted_and_stays_underlined: 3,
);
redaction_cases!(markdown_case;
    markdown_tag_name_sup_preserves_structural_tags: 0,
    markdown_tag_name_u_redacts_content_and_preserves_structural_tags: 1,
    markdown_exponent_content_is_redacted_inside_structural_tags: 2,
    markdown_underlined_content_is_redacted_inside_structural_tags: 3,
);

const LITERAL_REDACTED: &str = "<[REDACTED]>2</[REDACTED]> <u>under</u> end";

#[tokio::test]
#[serial_test::serial]
async fn docx_code_tag_names_are_redacted_as_content() {
    let output = redacted(
        LITERAL,
        "text/html",
        OutputFormat::Custom("docx".into()),
        "sup",
    )
    .await;
    let xml = package_xml(&output);
    let doc = roxmltree::Document::parse(&xml).unwrap();
    assert_eq!(text_of(doc.root()), LITERAL_REDACTED);
    assert!(!property(doc.root(), "vertAlign", "superscript"));
    assert!(!property(doc.root(), "u", "single"));
}

#[tokio::test]
#[serial_test::serial]
async fn pdf_code_tag_names_are_redacted_as_content() {
    let output = redacted(
        LITERAL,
        "text/html",
        OutputFormat::Custom("pdf".into()),
        "sup",
    )
    .await;
    let (text, content) = pdf_content(&output);
    assert_eq!(compact(&text), compact(LITERAL_REDACTED));
    assert!(content
        .operations
        .iter()
        .filter(|op| op.operator == "Ts")
        .all(|op| op.operands[0].as_float().expect("numeric rise") == 0.0));
    assert_eq!(
        content
            .operations
            .iter()
            .filter(|op| op.operator == "l")
            .count(),
        0
    );
}

#[tokio::test]
#[serial_test::serial]
async fn markdown_literal_tag_names_are_redacted_as_content() {
    let source = converted(LITERAL, "docx").await;
    let output = redacted(
        &source,
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        OutputFormat::Markdown,
        "sup",
    )
    .await;
    let mut options = comrak::Options::default();
    options.render.r#unsafe = true;
    let html = comrak::markdown_to_html(&output, &options);
    assert!(
        html.contains("&lt;[REDACTED]&gt;2&lt;/[REDACTED]&gt;"),
        "{html}"
    );
    assert!(
        !html.contains("<sup>") && !html.contains("<u>"),
        "literal DOCX text must stay literal: {html}"
    );
}
