use super::*;

/// Start a new zip entry named `name` and write `content` to it, panicking on either step's
/// error (test-only helper: every caller already controls the zip contents and inputs).
fn write_zip_part<W: std::io::Write + std::io::Seek>(
    zip: &mut zip::write::ZipWriter<W>,
    opts: zip::write::SimpleFileOptions,
    name: &str,
    content: &[u8],
) {
    use std::io::Write;
    zip.start_file(name, opts).unwrap();
    zip.write_all(content).unwrap();
}

/// Fixed OOXML package parts a minimal single-sheet `.xlsx` needs alongside its
/// sheet-specific content, shared by every synthetic-workbook test fixture below.
const XLSX_CONTENT_TYPES_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="xml" ContentType="application/xml"/>
  <Default Extension="rels"
ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Override PartName="/xl/workbook.xml"
ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>
</Types>"#;
const XLSX_PACKAGE_RELS_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1"
Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument"
Target="xl/workbook.xml"/>
</Relationships>"#;
const XLSX_WORKBOOK_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
      xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <sheets>
<sheet name="Sheet1" sheetId="1" r:id="rId1"/>
  </sheets>
</workbook>"#;
const XLSX_WORKBOOK_RELS_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1"
Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet"
Target="worksheets/sheet1.xml"/>
</Relationships>"#;
const XLSX_EMPTY_SHEET1_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <sheetData/>
</worksheet>"#;

/// Build a minimal in-memory `.xlsx` zip that contains a synthetic
/// `xl/revisions/revisionHeaders.xml` with the given `<header>` elements.
///
/// `headers` is a slice of `(guid, user_name, date_time)` tuples.
fn make_xlsx_with_revision_headers(headers: &[(&str, &str, &str)]) -> Vec<u8> {
    use zip::write::{SimpleFileOptions, ZipWriter};

    let mut buffer = Vec::new();
    {
        let mut zip = ZipWriter::new(Cursor::new(&mut buffer));
        let opts = SimpleFileOptions::default();

        write_zip_part(&mut zip, opts, "[Content_Types].xml", XLSX_CONTENT_TYPES_XML.as_bytes());
        write_zip_part(&mut zip, opts, "_rels/.rels", XLSX_PACKAGE_RELS_XML.as_bytes());
        write_zip_part(&mut zip, opts, "xl/workbook.xml", XLSX_WORKBOOK_XML.as_bytes());
        write_zip_part(
            &mut zip,
            opts,
            "xl/_rels/workbook.xml.rels",
            XLSX_WORKBOOK_RELS_XML.as_bytes(),
        );
        write_zip_part(
            &mut zip,
            opts,
            "xl/worksheets/sheet1.xml",
            XLSX_EMPTY_SHEET1_XML.as_bytes(),
        );

        let mut headers_xml = String::from(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<headers xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">"#,
        );
        for (guid, user, dt) in headers {
            use std::fmt::Write as FmtWrite;
            let _ = write!(
                headers_xml,
                r#"<header guid="{{{guid}}}" dateTime="{dt}" userName="{user}" maxSheetId="1"/>"#,
            );
        }
        headers_xml.push_str("\n</headers>");

        write_zip_part(
            &mut zip,
            opts,
            "xl/revisions/revisionHeaders.xml",
            headers_xml.as_bytes(),
        );

        let _ = zip.finish().unwrap();
    }
    buffer
}

#[test]
fn should_return_none_revisions_when_xl_revisions_absent() {
    use std::io::Write;
    use zip::write::{SimpleFileOptions, ZipWriter};

    let mut buffer = Vec::new();
    {
        let mut zip = ZipWriter::new(Cursor::new(&mut buffer));
        let opts = SimpleFileOptions::default();

        zip.start_file("[Content_Types].xml", opts).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="xml" ContentType="application/xml"/>
</Types>"#,
        )
        .unwrap();

        zip.start_file("xl/workbook.xml", opts).unwrap();
        zip.write_all(br#"<?xml version="1.0"?><workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheets/></workbook>"#).unwrap();

        let _ = zip.finish().unwrap();
    }

    let result = extract_xlsx_revisions_from_bytes(&buffer, ExcelReadCancellation::default()).unwrap();
    assert!(result.is_none(), "expected None when xl/revisions/ is absent");
}

#[test]
fn should_parse_two_revision_headers_with_correct_fields() {
    let xlsx = make_xlsx_with_revision_headers(&[
        ("11111111-2222-3333-4444-555555555555", "Alice", "2024-01-15T09:00:00Z"),
        ("AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE", "Bob", "2024-01-16T14:30:00Z"),
    ]);

    let revisions = extract_xlsx_revisions_from_bytes(&xlsx, ExcelReadCancellation::default())
        .unwrap()
        .expect("revisions should be Some when xl/revisions/revisionHeaders.xml is present");

    assert_eq!(revisions.len(), 2, "expected 2 revisions from 2 headers");

    assert_eq!(
        revisions[0].revision_id, "11111111-2222-3333-4444-555555555555",
        "guid should be stored without braces"
    );
    assert_eq!(revisions[0].author.as_deref(), Some("Alice"));
    assert_eq!(revisions[0].timestamp.as_deref(), Some("2024-01-15T09:00:00Z"));
    assert!(
        matches!(revisions[0].kind, RevisionKind::FormatChange),
        "kind should be FormatChange for v1 headers"
    );
    assert!(revisions[0].anchor.is_none(), "anchor should be None for v1");
    assert!(
        revisions[0].delta.content.is_empty(),
        "delta.content should be empty for v1"
    );

    assert_eq!(revisions[1].revision_id, "AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE");
    assert_eq!(revisions[1].author.as_deref(), Some("Bob"));
    assert_eq!(revisions[1].timestamp.as_deref(), Some("2024-01-16T14:30:00Z"));
}

#[test]
fn should_return_some_empty_vec_when_headers_xml_exists_but_has_no_header_elements() {
    use std::io::Write;
    use zip::write::{SimpleFileOptions, ZipWriter};

    let mut buffer = Vec::new();
    {
        let mut zip = ZipWriter::new(Cursor::new(&mut buffer));
        let opts = SimpleFileOptions::default();
        zip.start_file("[Content_Types].xml", opts).unwrap();
        zip.write_all(
            b"<?xml version=\"1.0\"?><Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"/>",
        )
        .unwrap();

        zip.start_file("xl/revisions/revisionHeaders.xml", opts).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<headers xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"/>
"#,
        )
        .unwrap();
        let _ = zip.finish().unwrap();
    }

    let revisions = extract_xlsx_revisions_from_bytes(&buffer, ExcelReadCancellation::default())
        .unwrap()
        .expect("revisions should be Some when file exists");
    assert!(revisions.is_empty(), "expected empty vec when no <header> elements");
}

#[test]
fn should_surface_revisions_in_full_xlsx_extraction() {
    let xlsx =
        make_xlsx_with_revision_headers(&[("DEADBEEF-0000-0000-0000-000000000001", "Carol", "2024-03-01T12:00:00Z")]);

    let (workbook, _warnings) = read_excel_bytes(&xlsx, ".xlsx", &test_limits(10_000)).expect("should parse workbook");
    let revisions = workbook
        .revisions
        .as_ref()
        .expect("revisions should be Some after full extraction");
    assert_eq!(revisions.len(), 1);
    assert_eq!(revisions[0].author.as_deref(), Some("Carol"));
}

/// `validate_zip_container` only inspects *declared* entry count/size/ratio from the
/// central directory; it never inspects a member's actual decompressed bytes, so
/// nothing before `extract_xlsx_revisions_from_archive` bounds a single member's
/// *actual* decompressed length. This pads `xl/revisions/revisionHeaders.xml` with
/// an XML comment past `MAX_EXCEL_ZIP_MEMBER_SIZE`, followed by a real `<header>`
/// element. Without `Read::take(MAX_EXCEL_ZIP_MEMBER_SIZE)` on the entry read, the
/// whole member is read, the comment closes, and the trailing `<header>` parses
/// into `Some(vec![..])`. With the cap, the read is truncated mid-comment, the XML
/// is no longer well-formed, and `roxmltree::Document::parse` fails, so the
/// function returns `None`.
#[test]
fn test_revisions_read_is_bounded_for_oversized_member() {
    let padding_len = MAX_EXCEL_ZIP_MEMBER_SIZE as usize + 4096;
    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <headers xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\">\n<!--",
    );
    xml.push_str(&"x".repeat(padding_len));
    xml.push_str(
        "-->\n<header guid=\"{DEADBEEF-0000-0000-0000-000000000099}\" \
         dateTime=\"2024-01-01T00:00:00Z\" userName=\"Late\" maxSheetId=\"1\"/>\n</headers>",
    );

    let mut buffer = Vec::new();
    {
        use std::io::Write as _;
        let mut zip = zip::ZipWriter::new(Cursor::new(&mut buffer));
        let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        zip.start_file("xl/revisions/revisionHeaders.xml", opts).unwrap();
        zip.write_all(xml.as_bytes()).unwrap();
        zip.finish().unwrap();
    }

    let result = extract_xlsx_revisions_from_bytes(&buffer, ExcelReadCancellation::default()).unwrap();
    assert!(
        result.is_none(),
        "a header sitting after MAX_EXCEL_ZIP_MEMBER_SIZE must never be reached; an \
         unbounded read would find it and return Some(..) instead of None, got {result:?}"
    );
}
