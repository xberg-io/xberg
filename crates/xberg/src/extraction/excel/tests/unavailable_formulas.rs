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

    let sheet = process_xlsx_sheet_safe(
        &mut workbook,
        "Sheet1",
        false,
        SheetFormulas::new(true, None),
        &mut warnings,
    )
    .unwrap();

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

#[test]
fn should_charge_uncached_cells_of_a_shared_formula_whose_anchor_has_a_saved_result_gh3442() {
    let limits = SecurityLimits {
        max_table_cells: 3,
        ..Default::default()
    };
    let (workbook, warnings) = read(
        r#"<row r="1"><c r="A1"><f t="shared" ref="A1:A9" si="0">B1*2</f><v>2</v></c></row>
           <row r="9"><c r="A9"><f t="shared" si="0"/></c></row>"#,
        ".xlsx",
        &limits,
    );

    assert_eq!(workbook.sheets[0].table_cells, Some(vec![vec!["2".to_owned()]]));
    assert!(warning_for(&warnings, "was truncated").is_some(), "{warnings:?}");
}

#[test]
fn should_keep_cached_errors_booleans_and_zero_of_formulas_gh3442() {
    let (workbook, warnings) = read(
        r#"<row r="1">
            <c r="A1" t="e"><f>1/0</f><v>#DIV/0!</v></c>
            <c r="B1" t="b"><f>FALSE()</f><v>0</v></c>
            <c r="C1" t="b"><f>TRUE()</f><v>1</v></c>
            <c r="D1"><f>1-1</f><v>0</v></c>
        </row>"#,
        ".xlsx",
        &test_limits(10_000),
    );

    assert_eq!(
        workbook.sheets[0].table_cells,
        Some(vec![vec![
            "#ERR: Div0".to_owned(),
            "false".to_owned(),
            "true".to_owned(),
            "0".to_owned()
        ]])
    );
    assert!(warning_for(&warnings, "no saved result").is_none(), "{warnings:?}");
}

#[test]
fn should_escape_a_formula_with_a_pipe_and_a_line_break_in_markdown_gh3442() {
    let (workbook, _) = read(
        r#"<row r="1"><c r="A1"><v>1</v></c><c r="B1"><f>IF(A1,"a|b",
"c")</f></c></row>
           <row r="2"><c r="A2"><v>2</v></c><c r="B2"><v>3</v></c></row>"#,
        ".xlsx",
        &test_limits(10_000),
    );
    let sheet = &workbook.sheets[0];

    assert_eq!(sheet.table_cells.as_ref().unwrap()[0][1], "=IF(A1,\"a|b\",\n\"c\")");
    assert_eq!(
        sheet.markdown,
        "## Sheet1\n\n| 1 | =IF(A1,\"a\\|b\",<br>\"c\") |\n| --- | --- |\n| 2 | 3 |\n"
    );
}

#[test]
fn should_treat_a_self_closing_result_element_as_unavailable_gh3442() {
    let (workbook, warnings) = read(
        r#"<row r="1"><c r="A1"><f>1+1</f><v/></c><c r="B1"><v>4</v></c></row>"#,
        ".xlsx",
        &test_limits(10_000),
    );

    assert_eq!(
        workbook.sheets[0].table_cells,
        Some(vec![vec!["=1+1".to_owned(), "4".to_owned()]])
    );
    assert!(warning_for(&warnings, "has 1 formula cell ").is_some(), "{warnings:?}");
}

#[test]
fn should_show_unavailable_formulas_in_a_1904_workbook_gh3442() {
    let worksheet = format!(
        r#"<worksheet {SHEET_NS}><sheetData><row r="1"><c r="A1"><f>1+1</f></c><c r="B1"><v>4</v></c></row></sheetData></worksheet>"#
    );
    let bytes = make_xlsx(
        r#"<workbookPr date1904="1"/><sheets><sheet name="Sheet1" sheetId="1" r:id="rId1"/></sheets>"#,
        WORKSHEET_REL,
        &[("xl/worksheets/sheet1.xml", worksheet.into_bytes())],
    );

    let (workbook, _) = read_excel_bytes(&bytes, ".xlsx", &test_limits(10_000)).unwrap();

    assert_eq!(
        workbook.sheets[0].table_cells,
        Some(vec![vec!["=1+1".to_owned(), "4".to_owned()]])
    );
}

#[test]
fn should_keep_the_formula_metadata_of_the_kept_rows_of_a_truncated_sheet_gh3442() {
    let limits = SecurityLimits {
        max_table_cells: 2,
        ..Default::default()
    };
    let (workbook, _) = read(
        r#"<row r="1"><c r="A1"><f>1+1</f><v>2</v></c><c r="B1"><v>7</v></c></row>
           <row r="2"><c r="A2"><f>9+9</f><v>18</v></c><c r="B2"><v>9</v></c></row>"#,
        ".xlsx",
        &limits,
    );

    assert_eq!(
        workbook.sheets[0].table_cells,
        Some(vec![vec!["2".to_owned(), "7".to_owned()]])
    );
    assert_eq!(workbook.metadata["formulas_Sheet1"], "A1=1+1");
}

#[test]
fn should_leave_formula_text_out_instead_of_failing_past_the_content_limit_gh3442() {
    // One long shared formula over many uncached cells expands to far more text than the file holds.
    let template = "A1+".repeat(500) + "1";
    let rows: String = (1..=70_000)
        .map(|row| {
            if row == 1 {
                format!(
                    r#"<row r="1"><c r="A1"><f t="shared" ref="A1:A70000" si="0">{template}</f></c><c r="B1"><v>5</v></c></row>"#
                )
            } else {
                format!(r#"<row r="{row}"><c r="A{row}"><f t="shared" si="0"/></c></row>"#)
            }
        })
        .collect();

    let (workbook, warnings) = read(&rows, ".xlsx", &SecurityLimits::default());

    assert_eq!(workbook.sheets[0].table_cells, Some(vec![vec!["5".to_owned()]]));
    let warning = warning_for(&warnings, "were left empty").expect("the omission is reported");
    assert!(warning.message.contains("has 70000 formula cells"), "{warning:?}");
    assert!(!workbook.metadata.contains_key("formulas_Sheet1"));
}

#[test]
fn should_show_unavailable_formulas_beside_ods_values_gh3442() {
    let bytes = ods_with_table_rows(
        r#"<table:table-row><table:table-cell table:formula="of:=1+1"/><table:table-cell office:value-type="float" office:value="4"/></table:table-row>"#,
    );

    let (workbook, _) = read_excel_bytes(&bytes, ".ods", &test_limits(10_000)).unwrap();

    assert_eq!(
        workbook.sheets[0].table_cells,
        Some(vec![vec!["=1+1".to_owned(), "4".to_owned()]])
    );
}

fn long_formula(length: usize) -> String {
    "1+".repeat(length / 2) + "1"
}

#[test]
fn should_not_charge_a_suppressed_sheets_formula_text_to_the_next_sheet_gh3442() {
    let limits = SecurityLimits {
        max_content_size: 8192,
        ..Default::default()
    };
    let long = long_formula(5000);
    let first = format!(
        r#"<worksheet {SHEET_NS}><sheetData><row r="1"><c r="A1"><v>1</v></c><c r="B1"><f>{long}</f></c></row><row r="2"><c r="B2"><f>{long}</f></c></row></sheetData></worksheet>"#
    );
    let second = format!(
        r#"<worksheet {SHEET_NS}><sheetData><row r="1"><c r="A1"><f>1+1</f></c><c r="B1"><v>4</v></c></row></sheetData></worksheet>"#
    );
    let bytes = make_xlsx(
        r#"<sheets><sheet name="Long" sheetId="1" r:id="rId1"/><sheet name="Short" sheetId="2" r:id="rId2"/></sheets>"#,
        &format!(
            r#"{WORKSHEET_REL}<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet2.xml"/>"#
        ),
        &[
            ("xl/worksheets/sheet1.xml", first.into_bytes()),
            ("xl/worksheets/sheet2.xml", second.into_bytes()),
        ],
    );

    let (workbook, warnings) = read_excel_bytes(&bytes, ".xlsx", &limits).unwrap();

    assert_eq!(workbook.sheets[0].table_cells, Some(vec![vec!["1".to_owned()]]));
    assert_eq!(
        workbook.sheets[1].table_cells,
        Some(vec![vec!["=1+1".to_owned(), "4".to_owned()]]),
        "{warnings:?}"
    );
}

#[test]
fn should_not_charge_the_cells_of_suppressed_formulas_to_the_table_budget_gh3442() {
    let limits = SecurityLimits {
        max_content_size: 8192,
        max_table_cells: 3,
        ..Default::default()
    };
    let long = long_formula(5000);
    let (workbook, warnings) = read(
        &format!(
            r#"<row r="1"><c r="A1"><v>1</v></c><c r="B1"><f>{long}</f></c></row>
               <row r="2"><c r="B2"><f>{long}</f></c></row>
               <row r="3"><c r="A3"><v>3</v></c></row>"#
        ),
        ".xlsx",
        &limits,
    );

    assert_eq!(
        workbook.sheets[0].table_cells,
        Some(vec![vec!["1".to_owned()], vec![String::new()], vec!["3".to_owned()]]),
        "{warnings:?}"
    );
    assert!(warning_for(&warnings, "were left empty").is_some(), "{warnings:?}");
    assert!(warning_for(&warnings, "was truncated").is_none(), "{warnings:?}");
}

#[test]
fn should_not_charge_the_formula_text_of_dropped_rows_to_the_next_sheet_gh3442() {
    let limits = SecurityLimits {
        max_content_size: 8192,
        max_table_cells: 3,
        ..Default::default()
    };
    let first = format!(
        r#"<worksheet {SHEET_NS}><sheetData><row r="1"><c r="A1"><v>1</v></c></row><row r="2"><c r="D2"><f>{}</f></c></row></sheetData></worksheet>"#,
        long_formula(5000)
    );
    let second = format!(
        r#"<worksheet {SHEET_NS}><sheetData><row r="1"><c r="A1"><f>{}</f></c></row></sheetData></worksheet>"#,
        long_formula(4000)
    );
    let bytes = make_xlsx(
        r#"<sheets><sheet name="First" sheetId="1" r:id="rId1"/><sheet name="Second" sheetId="2" r:id="rId2"/></sheets>"#,
        &format!(
            r#"{WORKSHEET_REL}<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet2.xml"/>"#
        ),
        &[
            ("xl/worksheets/sheet1.xml", first.into_bytes()),
            ("xl/worksheets/sheet2.xml", second.into_bytes()),
        ],
    );

    let (workbook, warnings) = read_excel_bytes(&bytes, ".xlsx", &limits).unwrap();

    assert_eq!(workbook.sheets[0].table_cells, Some(vec![vec!["1".to_owned()]]));
    assert_eq!(
        workbook.sheets[1].table_cells.as_ref().map(|cells| cells[0][0].len()),
        Some(4002),
        "{warnings:?}"
    );
}
