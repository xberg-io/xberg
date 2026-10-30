use super::*;

const CELL_TEXT_BYTES: usize = 80;
const WORKBOOK_TEXT_BYTES: usize = CELL_TEXT_BYTES * 2;
const XLSX_MIME: &str = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";

fn content_limits(max_content_size: usize) -> SecurityLimits {
    SecurityLimits {
        max_content_size,
        ..Default::default()
    }
}

fn inline_worksheet(text: &str, repeat: bool) -> Vec<u8> {
    let second = if repeat {
        format!(r#"<c r="B1" t="inlineStr"><is><t>{text}</t></is></c>"#)
    } else {
        String::new()
    };
    format!(
        r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
        <sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>{text}</t></is></c>{second}</row>
        </sheetData></worksheet>"#,
    )
    .into_bytes()
}

fn assert_text_limit(error: XbergError) {
    assert_content_limit(error, WORKBOOK_TEXT_BYTES, WORKBOOK_TEXT_BYTES - 1);
}

fn assert_content_limit(error: XbergError, size: usize, maximum: usize) {
    let XbergError::Security { message, .. } = error else {
        panic!("expected a security error, got {error}");
    };
    assert_eq!(
        message,
        format!("Content too large: {size} bytes (max: {maximum} bytes)")
    );
}

#[test]
fn should_limit_inline_xlsx_text_before_materializing_cells_gh2321() {
    for text in ["x".repeat(CELL_TEXT_BYTES), "é".repeat(CELL_TEXT_BYTES / 2)] {
        let worksheet = inline_worksheet(&text, true);
        let bytes = make_xlsx_with_worksheet(std::str::from_utf8(&worksheet).unwrap());
        let (workbook, _) = read_excel_bytes(&bytes, ".xlsx", &content_limits(WORKBOOK_TEXT_BYTES)).unwrap();
        assert_eq!(workbook.sheets.len(), 1);
        assert_eq!(
            workbook.sheets[0].table_cells.as_ref().unwrap(),
            &[vec![text.clone(), text]]
        );

        let error = read_excel_bytes(&bytes, ".xlsx", &content_limits(WORKBOOK_TEXT_BYTES - 1))
            .expect_err("inline cell text must be bounded before dense ranges and Markdown are built");
        assert_text_limit(error);
    }
}

#[test]
fn should_share_the_xlsx_text_budget_between_sheets_gh2321() {
    let text = "x".repeat(CELL_TEXT_BYTES);
    let worksheet = inline_worksheet(&text, false);
    let relationship_type = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet";
    let relationships = format!(
        r#"<Relationship Id="rId1" Type="{relationship_type}" Target="worksheets/sheet1.xml"/>
        <Relationship Id="rId2" Type="{relationship_type}" Target="worksheets/sheet2.xml"/>"#,
    );
    let bytes = make_xlsx(
        r#"<sheets><sheet name="First" sheetId="1" r:id="rId1"/>
        <sheet name="Second" sheetId="2" r:id="rId2"/></sheets>"#,
        &relationships,
        &[
            ("xl/worksheets/sheet1.xml", worksheet.clone()),
            ("xl/worksheets/sheet2.xml", worksheet),
        ],
    );
    let (workbook, _) = read_excel_bytes(&bytes, ".xlsx", &content_limits(WORKBOOK_TEXT_BYTES)).unwrap();
    assert_eq!(workbook.sheets.len(), 2);
    for sheet in workbook.sheets {
        assert_eq!(sheet.table_cells.unwrap(), [vec![text.clone()]]);
    }

    let error = read_excel_bytes(&bytes, ".xlsx", &content_limits(WORKBOOK_TEXT_BYTES - 1))
        .expect_err("the content budget must cover the complete workbook");
    assert_text_limit(error);
}

#[test]
fn should_charge_repeated_shared_strings_before_cloning_cells_gh2321() {
    let text = "x".repeat(CELL_TEXT_BYTES);
    let strings = format!(
        r#"<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" count="2" uniqueCount="1">
        <si><t>{text}</t></si></sst>"#,
    );
    let worksheet = br#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
        <sheetData><row r="1"><c r="A1" t="s"><v>0</v></c><c r="B1" t="s"><v>0</v></c></row>
        </sheetData></worksheet>"#;
    let bytes = make_xlsx(
        r#"<sheets><sheet name="Sheet1" sheetId="1" r:id="rId1"/></sheets>"#,
        WORKSHEET_REL,
        &[
            ("xl/worksheets/sheet1.xml", worksheet.to_vec()),
            ("xl/sharedStrings.xml", strings.into_bytes()),
        ],
    );
    let (workbook, _) = read_excel_bytes(&bytes, ".xlsx", &content_limits(WORKBOOK_TEXT_BYTES)).unwrap();
    assert_eq!(
        workbook.sheets[0].table_cells.as_ref().unwrap(),
        &[vec![text.clone(), text]]
    );

    let error = read_excel_bytes(&bytes, ".xlsx", &content_limits(WORKBOOK_TEXT_BYTES - 1))
        .expect_err("the unique shared-string pool does not bound repeated cell allocations");
    assert_text_limit(error);
}

#[cfg(feature = "tokio-runtime")]
#[tokio::test]
async fn should_apply_the_xlsx_text_budget_to_file_extraction_gh2321() {
    use crate::core::config::ExtractionConfig;
    use crate::extractors::excel::ExcelExtractor;
    use crate::plugins::InternalDocumentExtractor;

    let text = "x".repeat(CELL_TEXT_BYTES);
    let worksheet = inline_worksheet(&text, true);
    let bytes = make_xlsx_with_worksheet(std::str::from_utf8(&worksheet).unwrap());
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("content-limit.xlsx");
    std::fs::write(&path, &bytes).unwrap();
    let extractor = ExcelExtractor::new();
    let exact = ExtractionConfig {
        security_limits: Some(content_limits(WORKBOOK_TEXT_BYTES)),
        ..Default::default()
    };
    let document = extractor.extract_path(&path, XLSX_MIME, &exact).await.unwrap();
    let pages = document.prebuilt_pages.unwrap();
    assert_eq!(pages.len(), 1);
    assert_eq!(pages[0].tables[0].cells, [vec![text.clone(), text]]);

    let low = ExtractionConfig {
        security_limits: Some(content_limits(WORKBOOK_TEXT_BYTES - 1)),
        ..Default::default()
    };
    assert_text_limit(extractor.extract_content(&bytes, XLSX_MIME, &low).await.unwrap_err());
    assert_text_limit(
        extractor
            .extract_path(&path, XLSX_MIME, &low)
            .await
            .expect_err("file inputs must honor the same content limit as byte inputs"),
    );
}

#[cfg(feature = "tokio-runtime")]
#[tokio::test]
async fn should_apply_the_ods_formatted_text_budget_to_file_extraction_gh2321() {
    use crate::core::config::ExtractionConfig;
    use crate::extractors::excel::ExcelExtractor;
    use crate::plugins::InternalDocumentExtractor;

    const ODS_MIME: &str = "application/vnd.oasis.opendocument.spreadsheet";
    const FORMATTED_BYTES: usize = "1000000".len();
    let bytes = ods_with_table_rows(
        r#"<table:table-row><table:table-cell office:value-type="float" office:value="1e6"/></table:table-row>"#,
    );
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("content-limit.ods");
    std::fs::write(&path, &bytes).unwrap();
    let extractor = ExcelExtractor::new();
    let exact = ExtractionConfig {
        security_limits: Some(content_limits(FORMATTED_BYTES)),
        ..Default::default()
    };
    let document = extractor.extract_path(&path, ODS_MIME, &exact).await.unwrap();
    let pages = document.prebuilt_pages.unwrap();
    assert_eq!(pages.len(), 1);
    assert_eq!(pages[0].tables[0].cells, [vec!["1000000".to_owned()]]);

    let low = ExtractionConfig {
        security_limits: Some(content_limits(FORMATTED_BYTES - 1)),
        ..Default::default()
    };
    let (workbook, _) = read_excel_bytes(&bytes, ".ods", low.security_limits.as_ref().unwrap()).unwrap();
    assert_eq!(workbook.sheets[0].table_cells.as_ref().unwrap()[0][0], "1000000");
    let error = extractor.extract_content(&bytes, ODS_MIME, &low).await.unwrap_err();
    assert_content_limit(error, FORMATTED_BYTES, FORMATTED_BYTES - 1);
    let error = extractor
        .extract_path(&path, ODS_MIME, &low)
        .await
        .expect_err("formatted output must be bounded for paths even when its source spelling is shorter");
    assert_content_limit(error, FORMATTED_BYTES, FORMATTED_BYTES - 1);
}
