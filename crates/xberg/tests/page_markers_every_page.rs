//! Every page of a PDF and every slide of a PPTX gets a page marker, an empty one included.
#![cfg(all(feature = "pdf", feature = "office"))]

mod helpers;

use helpers::extract_bytes_document_blocking;
use std::io::Write;
use xberg::ExtractionConfig;
use zip::write::{SimpleFileOptions, ZipWriter};

const PPTX_MIME: &str = "application/vnd.openxmlformats-officedocument.presentationml.presentation";
const MARKER: &str = "<!-- PAGE ";

/// One A4 page per content stream, Helvetica, no embedded font.
fn build_pdf(pages: &[&str]) -> Vec<u8> {
    let mut objects: Vec<Vec<u8>> = vec![b"<< /Type /Catalog /Pages 2 0 R >>".to_vec()];
    let kids: Vec<String> = (0..pages.len()).map(|index| format!("{} 0 R", 4 + 2 * index)).collect();
    objects.push(format!("<< /Type /Pages /Kids [{}] /Count {} >>", kids.join(" "), pages.len()).into_bytes());
    objects.push(b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec());
    for text in pages {
        let stream = if text.is_empty() {
            String::new()
        } else {
            let lines: Vec<String> = text.lines().map(|line| format!("({line}) Tj")).collect();
            format!("BT /F1 14 Tf 18 TL 72 700 Td {} ET\n", lines.join(" T* "))
        };
        let contents_id = objects.len() + 2;
        objects.push(
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents {contents_id} 0 R \
                 /Resources << /Font << /F1 3 0 R >> >> >>"
            )
            .into_bytes(),
        );
        objects.push(format!("<< /Length {} >>\nstream\n{stream}endstream", stream.len()).into_bytes());
    }
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
    let size = objects.len() + 1;
    pdf.extend_from_slice(format!("trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes());
    pdf
}

fn slide_xml(text: &str) -> String {
    let paragraphs: String = text
        .lines()
        .map(|line| match line.strip_prefix("- ") {
            Some(item) => format!(
                r#"<a:p><a:pPr marL="285750" indent="-285750"><a:buChar char="&#8226;"/></a:pPr>
<a:r><a:t>{item}</a:t></a:r></a:p>"#
            ),
            None => format!(r#"<a:p><a:r><a:t>{line}</a:t></a:r></a:p>"#),
        })
        .collect();
    let body = if text.is_empty() {
        String::new()
    } else {
        format!(
            r#"<p:sp><p:nvSpPr><p:cNvPr id="2" name="Body"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/>
<p:txBody><a:bodyPr/>{paragraphs}</p:txBody></p:sp>"#
        )
    };
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
 xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
<p:cSld><p:spTree>{body}</p:spTree></p:cSld></p:sld>"#
    )
}

fn build_pptx(slides: &[&str]) -> Vec<u8> {
    build_pptx_with_broken_slide(slides, None)
}

/// A deck whose slide at `broken` (zero based) holds XML that does not parse.
fn build_pptx_with_broken_slide(slides: &[&str], broken: Option<usize>) -> Vec<u8> {
    let mut buffer = Vec::new();
    {
        let mut zip = ZipWriter::new(std::io::Cursor::new(&mut buffer));
        let options = SimpleFileOptions::default();
        let mut add = |name: &str, body: &str| {
            zip.start_file(name, options).unwrap();
            zip.write_all(body.as_bytes()).unwrap();
        };
        let overrides: String = (1..=slides.len())
            .map(|n| {
                format!(
                    r#"<Override PartName="/ppt/slides/slide{n}.xml"
 ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>"#
                )
            })
            .collect();
        add(
            "[Content_Types].xml",
            &format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
<Default Extension="xml" ContentType="application/xml"/>
<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
<Override PartName="/ppt/presentation.xml"
 ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>{overrides}</Types>"#
            ),
        );
        add(
            "_rels/.rels",
            r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1"
 Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument"
 Target="ppt/presentation.xml"/>
</Relationships>"#,
        );
        let ids: String = (1..=slides.len())
            .map(|n| format!(r#"<p:sldId id="{}" r:id="rId{n}"/>"#, 255 + n))
            .collect();
        add(
            "ppt/presentation.xml",
            &format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<p:presentation xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
 xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
<p:sldIdLst>{ids}</p:sldIdLst></p:presentation>"#
            ),
        );
        let rels: String = (1..=slides.len())
            .map(|n| {
                format!(
                    r#"<Relationship Id="rId{n}"
 Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide"
 Target="slides/slide{n}.xml"/>"#
                )
            })
            .collect();
        add(
            "ppt/_rels/presentation.xml.rels",
            &format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">{rels}</Relationships>"#
            ),
        );
        for (index, text) in slides.iter().enumerate() {
            let xml = if broken == Some(index) {
                "<p:sld><p:cSld><p:spTree></p:cSld></p:sld>".to_string()
            } else {
                slide_xml(text)
            };
            add(&format!("ppt/slides/slide{}.xml", index + 1), &xml);
        }
        zip.finish().unwrap();
    }
    buffer
}

const DOCX_MIME: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.document";

/// A DOCX whose second page is empty: two explicit page breaks in a row.
fn build_docx_with_blank_middle_page() -> Vec<u8> {
    let mut buffer = Vec::new();
    {
        let mut zip = ZipWriter::new(std::io::Cursor::new(&mut buffer));
        let options = SimpleFileOptions::default();
        let mut add = |name: &str, body: &str| {
            zip.start_file(name, options).unwrap();
            zip.write_all(body.as_bytes()).unwrap();
        };
        add(
            "[Content_Types].xml",
            r#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
<Default Extension="xml" ContentType="application/xml"/>
<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
<Override PartName="/word/document.xml"
 ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>"#,
        );
        add(
            "_rels/.rels",
            r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1"
 Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument"
 Target="word/document.xml"/>
</Relationships>"#,
        );
        let page_break = r#"<w:p><w:r><w:br w:type="page"/></w:r></w:p>"#;
        add(
            "word/document.xml",
            &format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>
<w:p><w:r><w:t>First page text</w:t></w:r></w:p>{page_break}{page_break}
<w:p><w:r><w:t>Third page text</w:t></w:r></w:p></w:body></w:document>"#
            ),
        );
        zip.finish().unwrap();
    }
    buffer
}

fn config(output_format: &str, structure: bool) -> ExtractionConfig {
    serde_json::from_value(serde_json::json!({
        "use_cache": false,
        "disable_ocr": true,
        "include_document_structure": structure,
        "output_format": output_format,
        "pages": {"extract_pages": true, "insert_page_markers": true}
    }))
    .unwrap()
}

fn marker_numbers(content: &str) -> Vec<u32> {
    content
        .lines()
        .filter_map(|line| line.trim().strip_prefix(MARKER)?.strip_suffix(" -->")?.parse().ok())
        .collect()
}

fn assert_every_page_marked(bytes: &[u8], mime: &str, pages: u32, label: &str) {
    for output_format in ["plain", "markdown", "html"] {
        for structure in [false, true] {
            let result = extract_bytes_document_blocking(bytes, mime, &config(output_format, structure)).unwrap();
            let got = marker_numbers(&result.content);
            let expected: Vec<u32> = (1..=pages).collect();
            let what = format!("{label} {output_format} structure={structure}");
            if output_format != "html" {
                assert_eq!(got, expected, "{what}: markers in content {:?}", result.content);
            }
            assert_eq!(result.counts.pages, pages as usize, "{what}: page count");
            let listed = result.pages.as_ref().map_or(0, Vec::len);
            assert_eq!(listed, pages as usize, "{what}: pages array");
        }
    }
}

#[test]
fn pdf_with_a_blank_middle_page_marks_all_three_pages() {
    let bytes = build_pdf(&["First page text", "", "Third page text"]);
    assert!(bytes.starts_with(b"%PDF-"));
    assert_every_page_marked(&bytes, "application/pdf", 3, "pdf");
}

#[test]
fn pptx_with_three_slides_marks_all_three_slides() {
    let bytes = build_pptx(&["First slide text", "Second slide text", "Third slide text"]);
    assert!(bytes.starts_with(b"PK"));
    assert_every_page_marked(&bytes, PPTX_MIME, 3, "pptx");
}

#[test]
fn pptx_with_a_blank_middle_slide_marks_all_three_slides() {
    let bytes = build_pptx(&["First slide text", "", "Third slide text"]);
    assert_every_page_marked(&bytes, PPTX_MIME, 3, "pptx blank middle");
}

#[test]
fn docx_with_a_blank_middle_page_marks_all_three_pages() {
    let bytes = build_docx_with_blank_middle_page();
    assert!(bytes.starts_with(b"PK"));
    for output_format in ["plain", "markdown"] {
        let result = extract_bytes_document_blocking(&bytes, DOCX_MIME, &config(output_format, true)).unwrap();
        assert_eq!(
            marker_numbers(&result.content),
            vec![1, 2, 3],
            "docx {output_format}: markers in content {:?}",
            result.content
        );
    }
}

/// Every marker stands on its own line, outside any list, in every output format.
fn assert_markers_stand_alone(bytes: &[u8], mime: &str, pages: u32, label: &str) {
    for output_format in ["plain", "markdown", "djot", "html"] {
        for structure in [false, true] {
            let result = extract_bytes_document_blocking(bytes, mime, &config(output_format, structure)).unwrap();
            let content = &result.content;
            let what = format!("{label} {output_format} structure={structure}");
            let expected: Vec<u32> = (1..=pages).collect();
            assert_eq!(
                marker_numbers(content),
                expected,
                "{what}: markers in content {content:?}"
            );
            let in_list = content.contains("- <!--") || content.contains("<li><!--");
            assert!(!in_list, "{what}: marker in a list: {content:?}");
            if structure && output_format != "plain" {
                let list_mark = if output_format == "html" { "<li>" } else { "- " };
                assert!(
                    content.contains(list_mark),
                    "{what}: the list did not render as a list: {content:?}"
                );
            }
        }
    }
}

#[test]
fn pdf_page_that_starts_with_a_list_gets_its_marker_before_the_list() {
    let bytes = build_pdf(&[
        "First page text",
        "- First item\n- Second item\n- Third item",
        "Third page text",
    ]);
    assert_markers_stand_alone(&bytes, "application/pdf", 3, "pdf list-first page");
}

#[test]
fn pptx_slide_that_starts_with_a_list_gets_its_marker_before_the_list() {
    let bytes = build_pptx(&[
        "First slide text",
        "Second slide text",
        "- First item\n- Second item\n- Third item",
    ]);
    assert_markers_stand_alone(&bytes, PPTX_MIME, 3, "pptx list-first slide");
}

#[test]
fn pptx_with_a_malformed_middle_slide_still_marks_all_three_slides() {
    let bytes = build_pptx_with_broken_slide(&["First slide text", "Second slide text", "Third slide text"], Some(1));
    for output_format in ["plain", "markdown", "djot", "html"] {
        let result = extract_bytes_document_blocking(&bytes, PPTX_MIME, &config(output_format, true)).unwrap();
        assert!(
            result.content.contains("Third slide text"),
            "{output_format}: slide 3 parsed"
        );
        assert!(
            !result.content.contains("Second slide text"),
            "{output_format}: slide 2 is the broken one"
        );
        assert_eq!(
            marker_numbers(&result.content),
            vec![1, 2, 3],
            "pptx malformed middle {output_format}: markers in content {:?}",
            result.content
        );
    }
}
