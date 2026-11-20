//! xberg-io/xberg#78: Excel workbooks may carry embedded objects under `xl/embeddings/`
//! (e.g. an embedded text file, spreadsheet, or OLE object pasted into a cell). Those
//! bytes must not be silently dropped — the extractor reuses the shared
//! `extraction::ooxml_embedded` OOXML-embedded-object pipeline (already exercised for
//! DOCX/PPTX) with the `xl/embeddings/` prefix, attaching results to
//! `ExtractedDocument.children`.

#![allow(clippy::print_stdout, clippy::print_stderr, clippy::dbg_macro)]
// ~keep: test/bench binaries print by design; org logging policy exempts tests
// Gated on `excel` as well as `office`: every test here extracts an XLSX, and
// `office` does not enable `excel`. Under `--features office` alone the file
// compiled and every test failed with UnsupportedFormat -- a spurious red on a
// valid feature combination, which is the mirror of a vacuous green.
#![cfg(all(feature = "office", feature = "excel"))]

use std::io::Write as _;
use zip::write::{SimpleFileOptions, ZipWriter};

mod helpers;
use helpers::{extract_bytes_document, extract_uri_document};

const XLSX_MIME: &str = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";

/// Build a minimal in-memory `.xlsx` with one sheet and one embedded `.txt` file under
/// `xl/embeddings/`.
fn make_xlsx_with_embedded_text_file(embedded_contents: &[u8]) -> Vec<u8> {
    let mut buffer = Vec::new();
    {
        let mut zip = ZipWriter::new(std::io::Cursor::new(&mut buffer));
        let options = SimpleFileOptions::default();

        zip.start_file("[Content_Types].xml", options).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="xml" ContentType="application/xml"/>
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Override PartName="/xl/workbook.xml"
    ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>
  <Override PartName="/xl/worksheets/sheet1.xml"
    ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>
</Types>"#,
        )
        .unwrap();

        zip.start_file("_rels/.rels", options).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1"
    Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument"
    Target="xl/workbook.xml"/>
</Relationships>"#,
        )
        .unwrap();

        zip.start_file("xl/workbook.xml", options).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
          xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <sheets><sheet name="Sheet1" sheetId="1" r:id="rId1"/></sheets>
</workbook>"#,
        )
        .unwrap();

        zip.start_file("xl/_rels/workbook.xml.rels", options).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1"
    Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet"
    Target="worksheets/sheet1.xml"/>
</Relationships>"#,
        )
        .unwrap();

        zip.start_file("xl/worksheets/sheet1.xml", options).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <dimension ref="A1"/>
  <sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>Hi</t></is></c></row></sheetData>
</worksheet>"#,
        )
        .unwrap();

        zip.start_file("docProps/core.xml", options).unwrap();
        zip.write_all(
            br#"<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties"
                xmlns:dc="http://purl.org/dc/elements/1.1/">
                <dc:title>Workbook title</dc:title>
            </cp:coreProperties>"#,
        )
        .unwrap();

        zip.start_file("xl/revisions/revisionHeaders.xml", options).unwrap();
        zip.write_all(
            br#"<headers xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
                <header guid="{11111111-2222-3333-4444-555555555555}" userName="Reviewer"/>
            </headers>"#,
        )
        .unwrap();

        zip.start_file("xl/comments1.xml", options).unwrap();
        zip.write_all(
            br#"<comments xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
                <authors><author>Reviewer</author></authors>
                <commentList><comment ref="A1" authorId="0"><text><t>Check this</t></text></comment></commentList>
            </comments>"#,
        )
        .unwrap();

        zip.start_file("xl/embeddings/oleObject1.txt", options).unwrap();
        zip.write_all(embedded_contents).unwrap();

        zip.finish().unwrap();
    }
    buffer
}

#[tokio::test]
async fn should_attach_embedded_object_as_child_document() {
    let embedded_text = b"embedded note contents";
    let bytes = make_xlsx_with_embedded_text_file(embedded_text);

    let mut config = xberg::ExtractionConfig {
        use_cache: false,
        max_archive_depth: 1,
        ..Default::default()
    };
    let result = extract_bytes_document(&bytes, XLSX_MIME, &config)
        .await
        .expect("workbook with an embedded object must still extract successfully");

    let children = result
        .children
        .as_ref()
        .expect("embedded object under xl/embeddings/ must be attached as a child document");
    assert_eq!(children.len(), 1, "exactly one embedded object expected");
    assert_eq!(children[0].path, "oleObject1.txt");
    assert!(
        children[0].result.content.contains("embedded note contents"),
        "child document content must contain the embedded file's text: {:?}",
        children[0].result.content
    );

    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("embedded.xlsx");
    std::fs::write(&path, &bytes).unwrap();
    let from_path = extract_uri_document(&path, None, &config).await.unwrap();
    assert_eq!(from_path.content, result.content);
    for extracted in [&result, &from_path] {
        assert_eq!(extracted.metadata.title.as_deref(), Some("Workbook title"));
        let comments = extracted
            .metadata
            .additional
            .get("comments")
            .and_then(serde_json::Value::as_str)
            .unwrap();
        assert!(comments.contains("Check this"));
        let revisions = extracted.revisions.as_ref().unwrap();
        assert_eq!(revisions.len(), 1);
        assert_eq!(revisions[0].author.as_deref(), Some("Reviewer"));
    }
    let path_children = from_path.children.as_ref().unwrap();
    assert_eq!(path_children.len(), 1);
    assert_eq!(path_children[0].path, children[0].path);
    assert_eq!(path_children[0].result.content, children[0].result.content);

    config.max_archive_depth = 0;
    let disabled_path = directory.path().join("embedded-disabled.xlsx");
    std::fs::write(&disabled_path, &bytes).unwrap();
    let disabled = extract_uri_document(&disabled_path, None, &config).await.unwrap();
    assert_eq!(disabled.content, from_path.content);
    assert!(disabled.children.is_none());
}

#[tokio::test]
async fn should_extract_normally_when_no_embedded_objects_present() {
    // Reuse the same builder shape but with an empty embeddings directory marker
    // removed entirely by not writing any xl/embeddings/ entry at all.
    let mut buffer = Vec::new();
    {
        let mut zip = ZipWriter::new(std::io::Cursor::new(&mut buffer));
        let options = SimpleFileOptions::default();

        zip.start_file("[Content_Types].xml", options).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="xml" ContentType="application/xml"/>
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Override PartName="/xl/workbook.xml"
    ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>
</Types>"#,
        )
        .unwrap();
        zip.start_file("_rels/.rels", options).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1"
    Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument"
    Target="xl/workbook.xml"/>
</Relationships>"#,
        )
        .unwrap();
        zip.start_file("xl/workbook.xml", options).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
          xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <sheets><sheet name="Sheet1" sheetId="1" r:id="rId1"/></sheets>
</workbook>"#,
        )
        .unwrap();
        zip.start_file("xl/_rels/workbook.xml.rels", options).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1"
    Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet"
    Target="worksheets/sheet1.xml"/>
</Relationships>"#,
        )
        .unwrap();
        zip.start_file("xl/worksheets/sheet1.xml", options).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <dimension ref="A1"/>
  <sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>Plain</t></is></c></row></sheetData>
</worksheet>"#,
        )
        .unwrap();
        zip.finish().unwrap();
    }

    let result = extract_bytes_document(&buffer, XLSX_MIME, &xberg::ExtractionConfig::default())
        .await
        .expect("workbook without embedded objects must extract normally");

    assert!(
        result.children.is_none(),
        "no children expected when there are no embedded objects"
    );
    assert!(result.content.contains("Plain"));
}
