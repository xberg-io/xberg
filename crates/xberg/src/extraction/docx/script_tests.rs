use super::{Paragraph, Run, parse_document};
#[cfg(feature = "tokio-runtime")]
use crate::core::config::{ExtractionConfig, OutputFormat};
use crate::extractors::security::{SecurityBudget, SecurityLimits};
use std::io::{Cursor, Write};

const WORD_NS: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
#[cfg(feature = "tokio-runtime")]
const DOCX_MIME: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.document";

fn script_run(text: &str, script: &str) -> Run {
    Run {
        text: text.into(),
        superscript: script == "superscript",
        subscript: script == "subscript",
        ..Default::default()
    }
}

fn xml_run(text: &str, script: &str) -> String {
    format!(r#"<w:r><w:rPr><w:vertAlign w:val="{script}"/></w:rPr><w:t xml:space="preserve">{text}</w:t></w:r>"#)
}

fn value_runs() -> String {
    format!(
        "{}{}{}{}{}{}",
        xml_run("7,2 x 10", "baseline"),
        xml_run("9", "superscript"),
        xml_run("/l; H", "baseline"),
        xml_run("2", "subscript"),
        xml_run("O", "baseline"),
        xml_run("", "superscript"),
    )
}

fn docx_package(body: &str, notes: bool) -> Vec<u8> {
    let document = format!(r#"<w:document xmlns:w="{WORD_NS}"><w:body>{body}<w:sectPr/></w:body></w:document>"#);
    let note_body = format!("<w:p>{}</w:p>", value_runs());
    let footnotes =
        format!(r#"<w:footnotes xmlns:w="{WORD_NS}"><w:footnote w:id="2">{note_body}</w:footnote></w:footnotes>"#);
    let endnotes =
        format!(r#"<w:endnotes xmlns:w="{WORD_NS}"><w:endnote w:id="2">{note_body}</w:endnote></w:endnotes>"#);
    let mut types = String::from(
        r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/>"#,
    );
    let mut parts = vec![("word/document.xml", document)];
    if notes {
        parts.extend([("word/footnotes.xml", footnotes), ("word/endnotes.xml", endnotes)]);
    }
    let mut relationships =
        String::from(r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#);
    for (index, (path, _)) in parts.iter().enumerate() {
        let kind = ["document", "footnotes", "endnotes"][index];
        let content_kind = if index == 0 { "document.main" } else { kind };
        types.push_str(&format!(
            r#"<Override PartName="/{path}" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.{content_kind}+xml"/>"#
        ));
        if index > 0 {
            relationships.push_str(&format!(
                r#"<Relationship Id="rId{index}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/{kind}" Target="{kind}.xml"/>"#
            ));
        }
    }
    types.push_str("</Types>");
    relationships.push_str("</Relationships>");
    parts.extend([
        ("[Content_Types].xml", types),
        ("word/_rels/document.xml.rels", relationships),
        ("_rels/.rels", String::from(r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#)),
    ]);
    let mut archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (path, content) in parts {
        archive
            .start_file(path, zip::write::FileOptions::<()>::default())
            .unwrap();
        archive.write_all(content.as_bytes()).unwrap();
    }
    archive.finish().unwrap().into_inner()
}

#[test]
fn should_render_issue_2275_scripts_on_individual_runs() {
    for (script, expected) in [("superscript", "^9^"), ("subscript", "~9~"), ("baseline", "9")] {
        assert_eq!(script_run("9", script).to_markdown(), expected, "{script}");
        assert_eq!(script_run("", script).to_markdown(), "", "empty {script}");
    }
}

#[test]
fn should_keep_issue_2275_adjacent_script_boundaries() {
    let cases = [("baseline", ""), ("superscript", "^"), ("subscript", "~")];
    let mut checked = 0;
    for (first, first_marker) in cases {
        for (second, second_marker) in cases {
            let mut paragraph = Paragraph::new();
            paragraph.runs = vec![
                script_run("10", "baseline"),
                script_run("1", first),
                script_run("2", second),
                script_run("/l", "baseline"),
            ];
            let expected = if first == second {
                format!("10{first_marker}12{first_marker}/l")
            } else {
                format!("10{first_marker}1{first_marker}{second_marker}2{second_marker}/l")
            };
            assert_eq!(paragraph.to_text(), "1012/l");
            assert_eq!(paragraph.runs_to_markdown(), expected, "{first} to {second}");
            checked += 1;
        }
    }
    assert_eq!(checked, 9);
}

#[test]
fn should_keep_issue_2275_scripts_inside_bold_italic_links() {
    let mut paragraph = Paragraph::new();
    paragraph.runs = vec![
        script_run("10", "baseline"),
        script_run("9", "superscript"),
        script_run("/l", "baseline"),
    ];
    for run in &mut paragraph.runs {
        run.bold = true;
        run.italic = true;
        run.hyperlink_url = Some("https://example.com/value".into());
    }
    assert_eq!(
        paragraph.runs_to_markdown(),
        "[***10^9^/l***](https://example.com/value)"
    );
    assert_eq!(
        paragraph.runs[1].to_markdown(),
        "[***^9^***](https://example.com/value)"
    );
}

#[test]
fn should_keep_issue_2275_scripts_when_inner_formatting_changes() {
    let mut paragraph = Paragraph::new();
    paragraph.runs = vec![
        script_run("10", "baseline"),
        script_run("1", "superscript"),
        script_run("2", "superscript"),
        script_run("3", "superscript"),
        script_run("/l", "baseline"),
    ];
    paragraph.runs[2].underline = true;
    paragraph.runs[3].strikethrough = true;
    for run in &mut paragraph.runs {
        run.bold = true;
    }
    assert_eq!(paragraph.runs_to_markdown(), "**10^1<u>2</u>~~3~~^/l**");
}

#[test]
fn should_keep_issue_2275_scripts_in_parsed_note_definitions() {
    let bytes = docx_package(
        "<w:p><w:r><w:footnoteReference w:id=\"2\"/></w:r><w:r><w:endnoteReference w:id=\"2\"/></w:r></w:p>",
        true,
    );
    let document = parse_document(&bytes, &mut SecurityBudget::with_defaults(), &SecurityLimits::default()).unwrap();
    assert_eq!(document.footnotes.len(), 1);
    assert_eq!(document.endnotes.len(), 1);
    assert_eq!(document.footnotes[0].paragraphs[0].runs.len(), 6);
    assert!(document.footnotes[0].paragraphs[0].runs[1].superscript);
    assert!(document.endnotes[0].paragraphs[0].runs[3].subscript);
    let definitions: Vec<_> = document
        .to_markdown(false)
        .lines()
        .filter(|line| line.starts_with("[^1]:") || line.starts_with("[^2]:"))
        .map(str::to_owned)
        .collect();
    assert_eq!(
        definitions,
        ["[^1]: 7,2 x 10^9^/l; H~2~O", "[^2]: 7,2 x 10^9^/l; H~2~O"]
    );
}

#[cfg(feature = "tokio-runtime")]
async fn extract_package(body: &str, output_format: OutputFormat) -> crate::types::extraction::ExtractedDocument {
    let config = ExtractionConfig {
        output_format,
        escape_markdown: false,
        use_cache: false,
        ..Default::default()
    };
    crate::core::extractor::extract_bytes(&docx_package(body, false), DOCX_MIME, &config)
        .await
        .unwrap()
}

#[cfg(feature = "tokio-runtime")]
#[tokio::test]
async fn should_keep_issue_2275_scripts_in_docx_table_markdown() {
    let body = format!(
        "<w:tbl><w:tr><w:tc><w:p><w:r><w:t>Value</w:t></w:r></w:p></w:tc></w:tr><w:tr><w:tc><w:p>{}</w:p></w:tc></w:tr></w:tbl>",
        value_runs()
    );
    let result = extract_package(&body, OutputFormat::Markdown).await;
    assert_eq!(result.tables.len(), 1);
    assert_eq!(result.tables[0].cells.len(), 2);
    assert_eq!(result.tables[0].cells[1], ["7,2 x 10^9^/l; H~2~O"]);
    assert_eq!(result.content.trim(), "| Value |\n| --- |\n| 7,2 x 10^9^/l; H~2~O |");
}

#[cfg(feature = "tokio-runtime")]
#[tokio::test]
async fn should_keep_issue_2275_body_markdown_control() {
    let result = extract_package(&format!("<w:p>{}</w:p>", value_runs()), OutputFormat::Markdown).await;
    assert_eq!(result.content.trim(), "7,2 x 10<sup>9</sup>/l; H~2~O");
}

#[cfg(feature = "tokio-runtime")]
#[tokio::test]
async fn should_keep_issue_2275_plain_text_control() {
    let result = extract_package(&format!("<w:p>{}</w:p>", value_runs()), OutputFormat::Plain).await;
    assert_eq!(result.content.trim(), "7,2 x 109/l; H2O");
}

#[cfg(feature = "tokio-runtime")]
#[tokio::test]
async fn should_keep_issue_2275_scripts_in_docx_heading_markdown() {
    let body = format!(
        "<w:p><w:pPr><w:pStyle w:val=\"Heading1\"/></w:pPr>{}</w:p>",
        value_runs()
    );
    let result = extract_package(&body, OutputFormat::Markdown).await;
    assert_eq!(result.content.trim(), "# 7,2 x 10<sup>9</sup>/l; H~2~O");
}
