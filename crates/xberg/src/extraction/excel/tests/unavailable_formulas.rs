use super::*;

const SHEET_NS: &str = r#"xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main""#;

fn read(worksheet_rows: &str, extension: &str, limits: &SecurityLimits) -> (ExcelWorkbook, Vec<ProcessingWarning>) {
    let bytes = make_xlsx_with_worksheet(&format!(
        "<worksheet {SHEET_NS}><sheetData>{worksheet_rows}</sheetData></worksheet>"
    ));
    read_excel_bytes(&bytes, extension, limits).expect("workbook must read")
}

fn warning_for<'a>(warnings: &'a [ProcessingWarning], needle: &str) -> Option<&'a ProcessingWarning> {
    warnings.iter().find(|warning| warning.message.contains(needle))
}

const ISSUE_ROW: &str = r#"<row r="1">
    <c r="A1"><f>1+1</f></c>
    <c r="B1"><f>2+2</f><v></v></c>
    <c r="C1" t="str"><f>"x"&amp;"y"</f><v></v></c>
    <c r="D1"><f>5+7</f><v>12</v></c>
</row>"#;

#[test]
fn should_show_formula_text_for_cells_without_a_saved_result_gh3442() {
    let (workbook, warnings) = read(ISSUE_ROW, ".xlsx", &test_limits(10_000));
    let sheet = &workbook.sheets[0];

    assert_eq!(
        sheet.table_cells,
        Some(vec![vec![
            "=1+1".to_owned(),
            "=2+2".to_owned(),
            String::new(),
            "12".to_owned()
        ]])
    );
    assert_eq!(
        sheet.markdown,
        "## Sheet1\n\n| =1+1 | =2+2 |  | 12 |\n| --- | --- | --- | --- |\n"
    );
    assert_eq!(excel_to_text(&workbook), "=1+1 =2+2 12");
    let warning = warning_for(&warnings, "no saved result").expect("a warning counts the cells");
    assert!(
        warning.message.contains("Sheet 'Sheet1' has 2 formula cells"),
        "{warning:?}"
    );
}

#[test]
fn should_keep_the_saved_result_of_a_formula_gh3442() {
    let (workbook, warnings) = read(
        r#"<row r="1"><c r="A1"><f>5+7</f><v>12</v></c><c r="B1" t="str"><f>"a"</f><v>kept</v></c></row>"#,
        ".xlsx",
        &test_limits(10_000),
    );

    assert_eq!(
        workbook.sheets[0].table_cells,
        Some(vec![vec!["12".to_owned(), "kept".to_owned()]])
    );
    assert!(warning_for(&warnings, "no saved result").is_none(), "{warnings:?}");
}

#[test]
fn should_show_expanded_shared_formula_text_without_a_saved_result_gh3442() {
    let (workbook, warnings) = read(
        r#"<row r="1"><c r="A1"><v>1</v></c><c r="B1"><f t="shared" ref="B1:B3" si="0">A1*2</f></c></row>
           <row r="2"><c r="A2"><v>2</v></c><c r="B2"><f t="shared" si="0"/></c></row>
           <row r="3"><c r="A3"><v>3</v></c><c r="B3"><f t="shared" si="0"/></c></row>"#,
        ".xlsx",
        &test_limits(10_000),
    );

    assert_eq!(
        workbook.sheets[0].table_cells,
        Some(vec![
            vec!["1".to_owned(), "=A1*2".to_owned()],
            vec!["2".to_owned(), "=A2*2".to_owned()],
            vec!["3".to_owned(), "=A3*2".to_owned()],
        ])
    );
    assert!(warning_for(&warnings, "has 3 formula cells").is_some(), "{warnings:?}");
}

#[test]
fn should_show_array_formula_text_without_a_saved_result_gh3442() {
    let (workbook, warnings) = read(
        r#"<row r="1"><c r="A1"><v>1</v></c><c r="B1"><f t="array" ref="B1">SUM(A1:A3)</f></c></row>"#,
        ".xlsx",
        &test_limits(10_000),
    );

    assert_eq!(
        workbook.sheets[0].table_cells,
        Some(vec![vec!["1".to_owned(), "=SUM(A1:A3)".to_owned()]])
    );
    assert!(warning_for(&warnings, "has 1 formula cell ").is_some(), "{warnings:?}");
}

#[test]
fn should_show_unavailable_formulas_for_every_xlsx_family_extension_gh3442() {
    for extension in [".xlsx", ".xlsm", ".xltm", ".xlam"] {
        let (workbook, _) = read(ISSUE_ROW, extension, &test_limits(10_000));
        assert_eq!(
            workbook.sheets[0].table_cells.as_ref().unwrap()[0][0],
            "=1+1",
            "{extension}"
        );
    }
}

#[test]
fn should_show_unavailable_formulas_in_a_sparse_sheet_gh3442() {
    let mut warnings = Vec::new();
    let bytes = make_xlsx_with_worksheet(&format!(
        r#"<worksheet {SHEET_NS}><sheetData>
<row r="1"><c r="A1"><f>1+1</f></c></row>
<row r="1048575"><c r="XFD1048575" t="inlineStr"><is><t>End</t></is></c></row>
</sheetData></worksheet>"#
    ));
    let mut workbook = calamine::Xlsx::new(Cursor::new(bytes.as_slice())).unwrap();
    assert_eq!(
        classify_xlsx_sheet_shape(&mut workbook, "Sheet1").unwrap(),
        XlsxSheetShape::Sparse
    );

    let sheet = process_xlsx_sheet_safe(&mut workbook, "Sheet1", &mut warnings).unwrap();

    assert_eq!(
        sheet.table_cells,
        Some(vec![
            vec!["=1+1".to_owned(), String::new()],
            vec![String::new(), "End".to_owned()],
        ])
    );
    assert!(warning_for(&warnings, "has 1 formula cell ").is_some(), "{warnings:?}");
}

#[test]
fn should_show_unavailable_formulas_in_a_truncated_sheet_gh3442() {
    let limits = SecurityLimits {
        max_table_cells: 2,
        ..Default::default()
    };
    let (workbook, warnings) = read(
        r#"<row r="1"><c r="A1"><f>1+1</f></c><c r="B1"><v>7</v></c></row>
           <row r="2"><c r="A2"><v>8</v></c><c r="B2"><v>9</v></c></row>"#,
        ".xlsx",
        &limits,
    );

    assert_eq!(
        workbook.sheets[0].table_cells,
        Some(vec![vec!["=1+1".to_owned(), "7".to_owned()]])
    );
    assert!(warning_for(&warnings, "was truncated").is_some(), "{warnings:?}");
    assert!(warning_for(&warnings, "has 1 formula cell ").is_some(), "{warnings:?}");
}

#[test]
fn should_not_widen_an_ods_table_past_the_charged_cells_for_an_unavailable_formula_gh3442() {
    let bytes = ods_with_table_rows(
        r#"<table:table-row><table:table-cell office:value-type="float" office:value="1"/></table:table-row><table:table-row table:number-rows-repeated="4"><table:table-cell/></table:table-row><table:table-row><table:table-cell table:number-columns-repeated="5"/><table:table-cell table:formula="of:=2+2"/></table:table-row>"#,
    );
    let limits = SecurityLimits {
        max_table_cells: 1,
        ..Default::default()
    };

    let (workbook, warnings) = read_excel_bytes(&bytes, ".ods", &limits).unwrap();

    assert_eq!(
        workbook.sheets[0].table_cells,
        Some(vec![vec!["1".to_owned()]]),
        "{warnings:?}"
    );
    let warning = warning_for(&warnings, "table-cell budget").expect("the unshown cell is counted");
    assert!(
        warning.message.contains("Sheet 'Data' has 1 formula cell "),
        "{warning:?}"
    );
}

#[test]
fn should_show_unavailable_formulas_in_ods_gh3442() {
    let bytes = ods_with_table_rows(
        r#"<table:table-row><table:table-cell table:formula="of:=1+1"/><table:table-cell table:formula="of:=5+7" office:value-type="float" office:value="12"/></table:table-row>"#,
    );

    let (workbook, warnings) = read_excel_bytes(&bytes, ".ods", &test_limits(10_000)).unwrap();

    assert_eq!(
        workbook.sheets[0].table_cells,
        Some(vec![vec!["=1+1".to_owned(), "12".to_owned()]]),
        "{warnings:?}"
    );
}
