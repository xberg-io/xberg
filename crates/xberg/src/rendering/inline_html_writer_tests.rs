use std::io::{Cursor, Read};

const NS: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const NESTED: &str = "x<sup>2<u>n</u>3</sup> tail\n";
const LITERALS: [&str; 6] = [
    "x<sup>2",
    "x</u> tail",
    "x<sup>2<u>n</sup></u> tail",
    "x<sup class=\"custom\">2</sup> tail",
    "x<unknown>2</unknown> tail",
    "`<sup>2</sup> <u>n</u>` tail",
];

fn word_runs(markdown: &str) -> Vec<(String, bool, bool)> {
    let bytes = super::render_docx(markdown).expect("actual DOCX writer");
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).expect("DOCX package");
    let mut xml = String::new();
    archive
        .by_name("word/document.xml")
        .unwrap()
        .read_to_string(&mut xml)
        .unwrap();
    let document = roxmltree::Document::parse(&xml).expect("actual document XML");
    let runs: Vec<_> = document
        .descendants()
        .filter(|node| node.has_tag_name((NS, "r")))
        .map(|node| {
            let text = node
                .descendants()
                .filter(|node| node.has_tag_name((NS, "t")))
                .filter_map(|node| node.text())
                .collect();
            let has = |name, value| {
                node.descendants()
                    .any(|node| node.has_tag_name((NS, name)) && node.attribute((NS, "val")) == Some(value))
            };
            (text, has("vertAlign", "superscript"), has("u", "single"))
        })
        .collect();
    assert!(!runs.is_empty());
    runs
}

fn pdf(markdown: &str) -> (String, Vec<lopdf::content::Operation>) {
    let bytes = super::render_pdf(markdown).expect("actual PDF writer");
    let document = lopdf::Document::load_mem(&bytes).expect("actual PDF");
    let pages = document.get_pages();
    assert_eq!(pages.len(), 1);
    let text = document.extract_text(&[1]).expect("actual selectable PDF text");
    let content = lopdf::content::Content::decode(&document.get_page_content(pages[&1])).expect("drawing operations");
    assert!(!content.operations.is_empty());
    (text, content.operations)
}

#[test]
fn should_restore_nested_inline_styles_in_docx_writer() {
    let runs = word_runs(NESTED);
    assert_eq!(runs.iter().map(|run| run.0.as_str()).collect::<String>(), "x2n3 tail");
    assert_eq!(
        runs.iter()
            .filter(|run| run.1)
            .map(|run| run.0.as_str())
            .collect::<Vec<_>>(),
        ["2", "n", "3"]
    );
    assert_eq!(
        runs.iter()
            .filter(|run| run.2)
            .map(|run| run.0.as_str())
            .collect::<Vec<_>>(),
        ["n"]
    );
    assert_eq!(runs.last(), Some(&(" tail".into(), false, false)));
}

#[test]
fn should_restore_nested_inline_styles_in_pdf_writer() {
    let (text, operations) = pdf(NESTED);
    assert_eq!(
        text.chars().filter(|c| !c.is_whitespace()).collect::<String>(),
        "x2n3tail"
    );
    let rises: Vec<_> = operations
        .iter()
        .filter(|operation| operation.operator == "Ts")
        .map(|operation| operation.operands[0].as_float().expect("numeric rise"))
        .collect();
    assert_eq!(rises.len(), 2, "one script opens and then restores the baseline");
    assert!(rises[0] > 0.0);
    assert_eq!(rises[1], 0.0);
    assert_eq!(
        operations.iter().filter(|operation| operation.operator == "l").count(),
        1
    );
}

#[test]
fn should_keep_unbalanced_unknown_and_code_tags_literal_in_docx_writer() {
    for markdown in LITERALS {
        let runs = word_runs(markdown);
        let expected = markdown.replace('`', "");
        assert_eq!(
            runs.iter().map(|run| run.0.as_str()).collect::<String>(),
            expected,
            "{markdown}"
        );
        assert!(runs.iter().all(|run| !run.1 && !run.2), "{markdown}: {runs:?}");
    }
}

#[test]
fn should_keep_unbalanced_unknown_and_code_tags_literal_in_pdf_writer() {
    for markdown in LITERALS {
        let (text, operations) = pdf(markdown);
        let expected: String = markdown.chars().filter(|c| !c.is_whitespace() && *c != '`').collect();
        assert_eq!(
            text.chars().filter(|c| !c.is_whitespace()).collect::<String>(),
            expected,
            "{markdown}"
        );
        assert!(
            operations
                .iter()
                .filter(|operation| operation.operator == "Ts")
                .all(|operation| operation.operands[0].as_float().expect("numeric rise") == 0.0),
            "{markdown}"
        );
        assert_eq!(
            operations.iter().filter(|operation| operation.operator == "l").count(),
            0,
            "{markdown}"
        );
    }
}

#[test]
fn should_keep_escaped_inline_tags_literal_in_docx_writer() {
    let runs = word_runs(r"x\<sup\>2\</sup\> \<u\>under\</u\> tail");
    assert_eq!(
        runs.iter().map(|run| run.0.as_str()).collect::<String>(),
        "x<sup>2</sup> <u>under</u> tail"
    );
    assert!(runs.iter().all(|run| !run.1 && !run.2));
}

#[test]
fn should_keep_escaped_inline_tags_literal_in_pdf_writer() {
    let (text, operations) = pdf(r"x\<sup\>2\</sup\> \<u\>under\</u\> tail");
    assert_eq!(
        text.chars().filter(|c| !c.is_whitespace()).collect::<String>(),
        "x<sup>2</sup><u>under</u>tail"
    );
    assert!(
        operations
            .iter()
            .filter(|operation| operation.operator == "Ts")
            .all(|operation| operation.operands[0].as_float().expect("numeric rise") == 0.0)
    );
    assert_eq!(
        operations.iter().filter(|operation| operation.operator == "l").count(),
        0
    );
}
